use serde::{Deserialize, Serialize};
use anyhow::Result;

/// Risk levels for tool execution. Determines approval requirements.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Risk {
    Low,
    Medium,
    High,
    Critical,
}

/// Dangerous command patterns — blocked unconditionally.
const DENY_PATTERNS: &[&str] = &[
    "rm -rf /",
    "rm -rf /*",
    "dd if=",
    "mkfs",
    ":(){ :|:& };",
    "chmod 777 /",
    "chmod -R 777",
    "> /dev/sda",
    "wget|sh",
    "curl|sh",
    "curl|bash",
    "wget|bash",
    "/dev/null 2>&1 &",
    "nohup",
    "shutdown",
    "reboot",
    "init 0",
    "init 6",
    "passwd",
    "useradd",
    "userdel",
    "groupadd",
    "chown -R root",
    "iptables -F",   # flushing firewall = bad
    "nft flush",
];

/// Path patterns that fs.read should never access.
const DENY_PATHS: &[&str] = &[
    "/etc/shadow",
    "/etc/passwd",
    "/etc/sudoers",
    "/root/",
    ".ssh/",
    ".gnupg/",
    ".age",
    "age.key",
    ".env",
    "secrets/",
    ".git/config",
    ".git/credentials",
    ".netrc",
];

/// Assess risk level for a tool based on its name.
pub fn assess_risk(tool: &str) -> Risk {
    match tool {
        // Low — read-only, no side effects
        "fs.read" | "process.list" | "docker.ps" | "service.status"
        | "net.status" | "dns.check" | "traffic.analyze"
        | "sec.triage" | "sec.cve_check" | "sec.threat_intel" | "sec.ioc_check"
        | "research.search" | "research.query" | "kb.query"
        | "code.analyze" | "git.status"
        | "pihole.stats" | "pihole.top_blocked" | "pihole.top_clients" | "pihole.enable"
        | "nas.health" | "nas.usage" | "nas.datasets" | "nas.disks"
        | "nas.smart" | "nas.alerts" | "nas.snapshot_list" | "nas.share_list"
        | "cam.status" | "cam.events" | "cam.snapshot" | "cam.clip"
        | "cam.search" | "cam.zones" | "cam.review" | "cam.anomaly"
        | "cam.person_detect"
        | "nas.pools" | "pihole.anomaly" | "pihole.query_log"
        // Productivity
        | "notes.create" | "notes.search" | "notes.list" | "notes.read"
        | "tasks.create" | "tasks.list" | "tasks.complete" | "tasks.delete"
        | "bookmarks.save" | "bookmarks.search"
        // OCR / Documents
        | "docs.ocr" | "docs.ingest_pdf"
        // Media
        | "media.search" | "media.library" | "media.playing"
        | "media.recent" | "media.sessions"
        // Home Assistant (read-only)
        | "home.status" | "home.climate" | "home.entities" | "home.automation"
        // Browser (read-only)
        | "browser.read" | "browser.screenshot" | "browser.search" | "browser.links"
        // Knowledge Graph
        | "kg.entities" | "kg.relationships" | "kg.add"
        // Social Media (read-only)
        | "social.draft" | "social.feed" | "social.analytics"
        | "social.platforms" | "social.monitor"
        => Risk::Low,

        // Medium — can execute code or make controlled changes
        "shell.run_safe" | "code.generate" | "code.test" | "code.refactor"
        | "workflow.create" | "workflow.run" | "schedule.add" | "trigger.fire"
        | "vpn.connect" | "sec.ssh_audit" | "sec.yara"
        | "kb.ingest"
        | "pihole.disable" | "pihole.add_block" | "pihole.remove_block"
        | "nas.snapshot_create" | "nas.share_create" | "nas.replicate" | "nas.backup_rednode"
        | "cam.alert_config" | "cam.retain_event"
        // Media (control)
        | "media.play" | "media.pause"
        // Home Assistant (control)
        | "home.lights" | "home.switch" | "home.scenes"
        // Browser (write)
        | "browser.scrape" | "browser.download"
        // Social Media (write)
        | "social.post" | "social.schedule" | "social.reply" | "social.dm"
        => Risk::Medium,

        // High — system-altering, needs human approval
        "service.restart" | "firewall.rules" | "sec.patch" | "sec.harden_ssh"
        | "browser.fill"
        | "nas.snapshot_delete"
        | "fw.block_ip" | "fw.unblock_ip" | "fw.isolate_device"
        => Risk::High,

        // Critical — anything unknown defaults to critical
        _ => Risk::Critical,
    }
}

/// Validate a tool call against security policies.
/// Returns the risk level if allowed, or an error if denied.
pub fn validate_tool(tool: &str, args: &serde_json::Value) -> Result<Risk> {
    let args_str = args.to_string().to_lowercase();

    // Check deny patterns in arguments
    for pat in DENY_PATTERNS {
        if args_str.contains(&pat.to_lowercase()) {
            anyhow::bail!("SECURITY DENY: pattern '{}' matched in args for tool '{}'", pat, tool);
        }
    }

    // Check path-specific denials for fs.read
    if tool == "fs.read" {
        if let Some(path) = args.get("path").and_then(|v| v.as_str()) {
            // Path traversal
            if path.contains("..") {
                anyhow::bail!("SECURITY DENY: path traversal '..' in fs.read path: {}", path);
            }
            // Denied paths
            for deny in DENY_PATHS {
                if path.contains(deny) {
                    anyhow::bail!("SECURITY DENY: access to '{}' blocked by policy", deny);
                }
            }
        }
    }

    // Check shell.run_safe for metacharacters
    if tool == "shell.run_safe" {
        if let Some(cmd) = args.get("cmd").and_then(|v| v.as_str()) {
            if cmd.contains([';', '|', '&', '$', '`', '>', '<', '\\', '\n']) {
                anyhow::bail!("SECURITY DENY: shell metacharacters in shell.run_safe: {}", cmd);
            }
        }
    }

    Ok(assess_risk(tool))
}

/// Whether a risk level requires human approval before execution.
pub fn needs_approval(risk: &Risk) -> bool {
    matches!(risk, Risk::High | Risk::Critical)
}

/// Whether a risk level should be auto-denied (never executed).
pub fn is_auto_deny(risk: &Risk) -> bool {
    matches!(risk, Risk::Critical)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deny_patterns() {
        let result = validate_tool("shell.run_safe", &serde_json::json!({"cmd": "rm -rf /"}));
        assert!(result.is_err());
    }

    #[test]
    fn test_path_traversal() {
        let result = validate_tool("fs.read", &serde_json::json!({"path": "../../etc/shadow"}));
        assert!(result.is_err());
    }

    #[test]
    fn test_denied_path() {
        let result = validate_tool("fs.read", &serde_json::json!({"path": "/root/.ssh/id_rsa"}));
        assert!(result.is_err());
    }

    #[test]
    fn test_safe_path() {
        let result = validate_tool("fs.read", &serde_json::json!({"path": "/tmp/test.txt"}));
        assert!(result.is_ok());
    }

    #[test]
    fn test_risk_levels() {
        assert_eq!(assess_risk("fs.read"), Risk::Low);
        assert_eq!(assess_risk("shell.run_safe"), Risk::Medium);
        assert_eq!(assess_risk("sec.harden_ssh"), Risk::High);
        assert_eq!(assess_risk("unknown_dangerous_tool"), Risk::Critical);
        assert_eq!(assess_risk("pihole.stats"), Risk::Low);
        assert_eq!(assess_risk("cam.events"), Risk::Low);
        assert_eq!(assess_risk("nas.snapshot_create"), Risk::Medium);
    }

    #[test]
    fn test_approval_needed() {
        assert!(!needs_approval(&Risk::Low));
        assert!(!needs_approval(&Risk::Medium));
        assert!(needs_approval(&Risk::High));
        assert!(needs_approval(&Risk::Critical));
    }

    #[test]
    fn test_shell_metachar_denied() {
        let result = validate_tool("shell.run_safe", &serde_json::json!({"cmd": "ls | grep foo"}));
        assert!(result.is_err());
    }
}
