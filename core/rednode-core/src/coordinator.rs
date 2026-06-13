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
                results.push(json!({"tool": step.tool, "status":"denied", "error": e.to_string()}));
                continue;
            }
        };
        let risk_str = format!("{:?}", risk).to_lowercase();

        // Approval gate
        if security::needs_approval(&risk) {
            // create approval record
            let approval_id = crate::memory::create_approval(
                "cns", &step.tool, &step.args, &risk_str, Some(intent), Some(session)
            ).await.ok();
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
                tracing::warn!(tool=%step.tool, "agent timeout, falling back to local executor");
                match crate::executor::execute(&step.tool, &step.args, &step.agent).await {
                    Ok((out, audit_id)) => json!({"ok": true, "output": out, "audit_id": audit_id, "fallback": true}),
                    Err(e) => json!({"ok": false, "error": e.to_string()}),
                }
            }
        };

        results.push(json!({
            "tool": step.tool,
            "agent": step.agent,
            "status": if agent_result.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) { "executed" } else { "failed" },
            "result": agent_result
        }));
    }
    let plan_json: Vec<serde_json::Value> = steps.into_iter().map(|s| serde_json::to_value(s).unwrap()).collect();
    (plan_json, results)
}
