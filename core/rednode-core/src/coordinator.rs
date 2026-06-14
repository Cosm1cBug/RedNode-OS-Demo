use crate::{planner::{plan, PlanStep}, security};
use serde_json::json;

pub async fn coordinate(intent: &str, session: &str) -> (Vec<serde_json::Value>, Vec<serde_json::Value>) {
    let steps = plan(intent).await;
    let mut results = Vec::new();

    for step in &steps {
        // Security validation
        let risk = match security::validate_tool(&step.tool, &step.args) {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!(tool=%step.tool, error=%e, "tool denied by security policy");
                crate::events::emit_tool_result(&step.tool, &step.agent, "denied", None);
                results.push(json!({"tool": step.tool, "status":"denied", "error": e.to_string()}));
                continue;
            }
        };
        let risk_str = format!("{:?}", risk).to_lowercase();

        // Approval gate — High/Critical require human approval
        if security::needs_approval(&risk) {
            let approval_id = crate::memory::create_approval(
                "cns", &step.tool, &step.args, &risk_str, Some(intent), Some(session)
            ).await.ok();

            // Emit to event bus — dashboard shows approval needed in real time
            crate::events::emit_approval_needed(&step.tool, &risk_str, approval_id);

            tracing::info!(
                tool=%step.tool, risk=%risk_str, approval_id=?approval_id,
                "tool requires approval — queued"
            );

            results.push(json!({
                "tool": step.tool,
                "agent": step.agent,
                "status": "needs_approval",
                "risk": risk_str,
                "approval_id": approval_id
            }));
            continue;
        }

        // Dispatch to Agent via NATS request/reply – 8s timeout
        let agent_subject = format!("rednode.agent.{}.task", step.agent.replace("-agent",""));
        let task_payload = json!({
            "tool": step.tool,
            "args": step.args,
            "intent": intent,
            "session_id": session,
            "risk": risk_str
        });

        let agent_result = match crate::bus::request(&agent_subject, task_payload, 8000).await {
            Ok(v) => v,
            Err(_) => {
                // Fallback: execute directly via local executor (agent offline – dev mode)
                tracing::warn!(tool=%step.tool, agent=%step.agent, "agent timeout — falling back to local executor");
                match crate::executor::execute(&step.tool, &step.args, &step.agent).await {
                    Ok((out, audit_id)) => {
                        crate::events::emit_tool_result(&step.tool, &step.agent, "executed_local", Some(audit_id));
                        json!({"ok": true, "output": out, "audit_id": audit_id, "fallback": true})
                    }
                    Err(e) => {
                        crate::events::emit_tool_result(&step.tool, &step.agent, "failed", None);
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

        crate::events::emit_tool_result(&step.tool, &step.agent, status, audit_id);

        results.push(json!({
            "tool": step.tool,
            "agent": step.agent,
            "status": status,
            "result": agent_result
        }));
    }

    let plan_json: Vec<serde_json::Value> = steps.into_iter()
        .map(|s| serde_json::to_value(s).unwrap())
        .collect();

    (plan_json, results)
}
