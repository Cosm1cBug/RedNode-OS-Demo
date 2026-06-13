use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(rename_all="lowercase")] pub enum Risk { Low, Medium, High, Critical }
const DENY_PATTERNS: &[&str] = &["rm -rf /", "dd if=", "mkfs", ":(){ :|:& };", "chmod 777 /"];

pub fn assess_risk(tool: &str) -> Risk {
    match tool {
        "fs.read" | "process.list" | "docker.ps" | "net.status" | "sec.triage" | "research.search" => Risk::Low,
        "shell.run_safe" | "code.generate" | "code.test" | "workflow.run" | "vpn.connect" => Risk::Medium,
        "service.restart" | "firewall.rules" | "sec.patch" | "sec.harden_ssh" => Risk::High,
        _ => Risk::Critical,
    }
}
pub fn validate_tool(tool: &str, args: &serde_json::Value) -> Result<Risk> {
    let cmd = args.to_string();
    for pat in DENY_PATTERNS { if cmd.contains(pat) { anyhow::bail!("Security deny pattern: {}", pat); } }
    Ok(assess_risk(tool))
}
pub fn needs_approval(risk: &Risk) -> bool { matches!(risk, Risk::High | Risk::Critical) }
