use crate::{planner::{plan, PlanStep}, security};
use serde_json::json;
use std::collections::HashMap;

/// Execute a plan: validate → approve/dispatch → audit.
/// Independent steps (different agents, no data dependency) run in parallel.
/// Steps needing approval are queued without blocking others.
pub async fn coordinate(intent: &str, session: &str) -> (Vec<serde_json::Value>, Vec<serde_json::Value>) {
    let steps = plan(intent).await;

    // ── Phase 1: Security validation + approval gate (sequential, fast) ──
    // Separate steps into: executable (Low/Medium) vs needs_approval (High/Critical) vs denied
    let mut executable: Vec<(usize, &PlanStep, String)> = Vec::new(); // (index, step, risk_str)
    let mut non_executable: Vec<(usize, serde_json::Value)> = Vec::new(); // (index, result)

    for (i, step) in steps.iter().enumerate() {
        let risk = match security::validate_tool(&step.tool, &step.args) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(tool=%step.tool, error=%e, "tool denied by security policy");
                crate::events::emit_tool_result(&step.tool, &step.agent, "denied", None);
                non_executable.push((i, json!({"tool": step.tool, "status": "denied", "error": e.to_string()})));
                continue;
            }
        };
        let risk_str = format!("{:?}", risk).to_lowercase();

        if security::needs_approval(&risk) {
            let approval_id = crate::memory::create_approval(
                "cns", &step.tool, &step.args, &risk_str, Some(intent), Some(session)
            ).await.ok();
            crate::events::emit_approval_needed(&step.tool, &risk_str, approval_id);
            tracing::info!(tool=%step.tool, risk=%risk_str, "tool requires approval — queued");
            non_executable.push((i, json!({
                "tool": step.tool, "agent": step.agent,
                "status": "needs_approval", "risk": risk_str, "approval_id": approval_id
            })));
        } else {
            executable.push((i, step, risk_str));
        }
    }

    // ── Phase 2: Parallel execution of independent steps ──
    // Group steps by agent — steps targeting different agents are independent.
    // Steps targeting the same agent execute sequentially (agent handles one task at a time).
    let mut agent_groups: HashMap<String, Vec<(usize, &PlanStep, String)>> = HashMap::new();
    for (i, step, risk_str) in &executable {
        agent_groups.entry(step.agent.clone()).or_default().push((*i, step, risk_str.clone()));
    }

    // State cache: results from earlier steps available to later ones within same session
    let state_cache = std::sync::Arc::new(tokio::sync::RwLock::new(HashMap::<String, serde_json::Value>::new()));

    // Execute each agent group concurrently
    let intent_owned = intent.to_string();
    let session_owned = session.to_string();
    let mut handles = Vec::new();

    for (agent_name, agent_steps) in agent_groups {
        let intent_c = intent_owned.clone();
        let session_c = session_owned.clone();
        let cache = state_cache.clone();

        let handle = tokio::spawn(async move {
            let mut group_results: Vec<(usize, serde_json::Value)> = Vec::new();

            for (idx, step, risk_str) in agent_steps {
                let tool = step.tool.clone();
                let agent = step.agent.clone();
                let args = step.args.clone();

                // Inject cached state from previous steps (if any)
                let mut enriched_args = args.clone();
                {
                    let cache_read = cache.read().await;
                    if !cache_read.is_empty() {
                        if let serde_json::Value::Object(ref mut map) = enriched_args {
                            map.insert("_state_cache".to_string(), json!(*cache_read));
                        }
                    }
                }

                // Dispatch to agent via NATS
                let agent_subject = format!("rednode.agent.{}.task", agent.replace("-agent", ""));
                let task_payload = json!({
                    "tool": tool,
                    "args": enriched_args,
                    "intent": intent_c,
                    "session_id": session_c,
                    "risk": risk_str
                });

                let agent_result = match crate::bus::request(&agent_subject, task_payload, 8000).await {
                    Ok(v) => v,
                    Err(_) => {
                        tracing::warn!(tool=%tool, agent=%agent, "agent timeout — falling back to local executor");
                        match crate::executor::execute(&tool, &args, &agent).await {
                            Ok((out, audit_id)) => {
                                crate::events::emit_tool_result(&tool, &agent, "executed_local", Some(audit_id));
                                json!({"ok": true, "output": out, "audit_id": audit_id, "fallback": true})
                            }
                            Err(e) => {
                                crate::events::emit_tool_result(&tool, &agent, "failed", None);
                                json!({"ok": false, "error": e.to_string()})
                            }
                        }
                    }
                };

                let status = if agent_result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
                    "executed"
                } else {
                    "failed"
                };

                let audit_id = agent_result.get("audit_id").and_then(|v| v.as_i64())
                    .or_else(|| agent_result.get("result").and_then(|r| r.get("audit_id")).and_then(|v| v.as_i64()));

                crate::events::emit_tool_result(&tool, &agent, status, audit_id);

                // Cache this step's result for subsequent steps
                {
                    let mut cache_write = cache.write().await;
                    cache_write.insert(tool.clone(), agent_result.clone());
                }

                group_results.push((idx, json!({
                    "tool": tool,
                    "agent": agent,
                    "status": status,
                    "result": agent_result
                })));
            }

            group_results
        });

        handles.push(handle);
    }

    // Collect all parallel results
    let mut all_indexed_results: Vec<(usize, serde_json::Value)> = non_executable;
    for handle in handles {
        match handle.await {
            Ok(group_results) => all_indexed_results.extend(group_results),
            Err(e) => tracing::error!("agent group execution panicked: {}", e),
        }
    }

    // Sort by original step index to maintain order in response
    all_indexed_results.sort_by_key(|(i, _)| *i);
    let results: Vec<serde_json::Value> = all_indexed_results.into_iter().map(|(_, r)| r).collect();

    let plan_json: Vec<serde_json::Value> = steps.into_iter()
        .map(|s| serde_json::to_value(s).unwrap())
        .collect();

    (plan_json, results)
}
