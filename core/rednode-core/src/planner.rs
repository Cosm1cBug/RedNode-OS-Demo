use crate::security::Risk;
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize, Clone)] pub struct PlanStep {
    pub tool: String, pub agent: String, pub args: serde_json::Value, pub risk: Risk
}
pub async fn plan(intent: &str) -> Vec<PlanStep> {
    let s = intent.to_lowercase();
    if s.contains("ssh") && s.contains("harden") {
        return vec![
            PlanStep{ tool:"sec.ssh_audit".into(), agent:"security-agent".into(), args:serde_json::json!({}), risk:Risk::Medium },
            PlanStep{ tool:"sec.harden_ssh".into(), agent:"security-agent".into(), args:serde_json::json!({}), risk:Risk::High },
        ];
    }
    if s.contains("docker") || s.contains("system") || s.contains("health") {
        return vec![
            PlanStep{ tool:"process.list".into(), agent:"system-agent".into(), args:serde_json::json!({}), risk:Risk::Low },
            PlanStep{ tool:"docker.ps".into(), agent:"system-agent".into(), args:serde_json::json!({}), risk:Risk::Low },
        ];
    }
    if s.contains("network") || s.contains("firewall") {
        return vec![PlanStep{ tool:"net.status".into(), agent:"network-agent".into(), args:serde_json::json!({}), risk:Risk::Low }];
    }
    if s.contains("code") || s.contains("debug") {
        return vec![PlanStep{ tool:"code.analyze".into(), agent:"coding-agent".into(), args:serde_json::json!({}), risk:Risk::Low }];
    }
    vec![PlanStep{ tool:"research.query".into(), agent:"research-agent".into(), args:serde_json::json!({"query":intent}), risk:Risk::Low }]
}
