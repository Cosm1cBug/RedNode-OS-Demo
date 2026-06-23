use crate::security::Risk;
use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlanStep {
    pub tool: String,
    pub agent: String,
    pub args: serde_json::Value,
    pub risk: Risk,
}

/// Tool registry context injected into the LLM prompt.
/// Dynamic tool context — loaded from tools.json at runtime.
/// This means new tools added by the evolution engine are immediately
/// available to the LLM planner without recompilation.
fn load_tool_context() -> String {
    // Try to load from tools.json (dynamic)
    let project_root = std::env::var("REDNODE_SOURCE")
        .unwrap_or_else(|_| std::env::var("REDNODE_HOME")
            .map(|h| format!("{}/source", h))
            .unwrap_or_else(|_| ".".into()));
    
    let dynamic = crate::evolution::generate_tool_context(&project_root);
    
    if dynamic.lines().count() > 5 {
        tracing::info!("Planner: loaded tool context dynamically from tools.json");
        dynamic
    } else {
        tracing::warn!("Planner: tools.json not found, using minimal fallback");
        // Minimal fallback — just enough for basic operation
        String::from(r#"Available tools (name | agent | risk | description):
- shell.run_safe | system-agent | medium | Run a safe shell command
- research.search | research-agent | low | Search the web via SearXNG
- research.query | research-agent | low | Query knowledge base
- cam.events | surveillance-agent | low | Camera detection events
- pihole.stats | infra-agent | low | Pi-hole DNS stats
- nas.health | storage-agent | low | TrueNAS health
- net.status | network-agent | low | Network status
"#)
    }
}

/// Plan an intent using Ollama LLM, with keyword fallback.
pub async fn plan(intent: &str) -> Vec<PlanStep> {
    match plan_with_llm(intent).await {
        Ok(steps) if !steps.is_empty() => {
            tracing::info!(
                intent,
                steps = steps.len(),
                "LLM planner: {} steps",
                steps.len()
            );
            steps
        }
        Ok(_) => {
            tracing::warn!(intent, "LLM returned empty plan — falling back to keyword");
            plan_keyword_fallback(intent)
        }
        Err(e) => {
            tracing::warn!(intent, error = %e, "LLM planner failed — falling back to keyword");
            plan_keyword_fallback(intent)
        }
    }
}

/// Call Ollama to generate a structured plan from natural language.
async fn plan_with_llm(intent: &str) -> Result<Vec<PlanStep>> {
    let ollama_url =
        std::env::var("OLLAMA_URL").unwrap_or_else(|_| "http://127.0.0.1:11434".into());
    let model =
        std::env::var("REDNODE_MODEL").unwrap_or_else(|_| "qwen2.5:14b-instruct-q4_K_M".into());

    let system_prompt = format!(
        "You are the RedNode-OS planner. You convert user intentions into \
         executable tool plans.\n\n\
         {}\n\n\
         Rules:\n\
         1. Only use tools from the list above.\n\
         2. Use the minimum steps needed — don't over-plan.\n\
         3. Order steps logically (diagnose before fix, read before write).\n\
         4. If the intent is unclear, use research.query to look it up.\n\
         5. Set args as relevant JSON (e.g. {{\"path\":\"/etc/hostname\"}} for fs.read).\n\
         6. Respond with ONLY a JSON array. No markdown, no explanation.\n\n\
         Example:\n\
         User: \"check disk space and list docker containers\"\n\
         Response: [\n\
           {{\"tool\":\"shell.run_safe\",\"agent\":\"system-agent\",\"args\":{{\"cmd\":\"df\"}},\"risk\":\"medium\"}},\n\
           {{\"tool\":\"docker.ps\",\"agent\":\"system-agent\",\"args\":{{}},\"risk\":\"low\"}}\n\
         ]",
        &load_tool_context()
    );

    let user_prompt = format!("User intent: \"{}\"\nJSON array:", intent);

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()?;

    let resp = client
        .post(format!("{}/api/chat", ollama_url))
        .json(&serde_json::json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system_prompt},
                {"role": "user", "content": user_prompt}
            ],
            "stream": false,
            "options": {
                "temperature": 0.1,
                "num_predict": 1024,
                "top_p": 0.9
            }
        }))
        .send()
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("Ollama returned {}", resp.status());
    }

    let body: serde_json::Value = resp.json().await?;
    let response_text = body["message"]["content"].as_str().unwrap_or("[]");

    let json_str = extract_json_array(response_text);
    let raw_steps: Vec<serde_json::Value> = serde_json::from_str(&json_str).map_err(|e| {
        anyhow::anyhow!(
            "Failed to parse LLM response as JSON: {} — raw: {}",
            e,
            &json_str[..json_str.len().min(200)]
        )
    })?;

    // Parse and validate each step
    let mut steps = Vec::new();
    for raw in raw_steps {
        let tool = raw["tool"].as_str().unwrap_or("").to_string();
        let agent = raw["agent"].as_str().unwrap_or("").to_string();
        let args = raw.get("args").cloned().unwrap_or(serde_json::json!({}));
        let risk_str = raw["risk"].as_str().unwrap_or("low");
        let risk = match risk_str {
            "low" => Risk::Low,
            "medium" => Risk::Medium,
            "high" => Risk::High,
            "critical" => Risk::Critical,
            _ => crate::security::assess_risk(&tool),
        };

        // Skip steps with empty tool names (LLM hallucination guard)
        if tool.is_empty() || agent.is_empty() {
            tracing::warn!("Skipping step with empty tool/agent: {:?}", raw);
            continue;
        }

        // Override LLM's risk assessment with our authoritative one
        let authoritative_risk = crate::security::assess_risk(&tool);
        steps.push(PlanStep {
            tool,
            agent,
            args,
            risk: authoritative_risk,
        });
    }

    Ok(steps)
}

/// Extract the first JSON array from LLM output (handles markdown wrapping).
fn extract_json_array(text: &str) -> String {
    // Strip markdown code fences if present
    let cleaned = text
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    // Find first [ and last ]
    if let Some(start) = cleaned.find('[') {
        if let Some(end) = cleaned.rfind(']') {
            if end > start {
                return cleaned[start..=end].to_string();
            }
        }
    }
    "[]".to_string()
}

/// Keyword-based fallback — used when Ollama is unreachable.
fn plan_keyword_fallback(intent: &str) -> Vec<PlanStep> {
    let s = intent.to_lowercase();

    // Security
    if s.contains("ssh") && s.contains("harden") {
        return vec![
            PlanStep {
                tool: "sec.ssh_audit".into(),
                agent: "security-agent".into(),
                args: serde_json::json!({}),
                risk: Risk::Medium,
            },
            PlanStep {
                tool: "sec.harden_ssh".into(),
                agent: "security-agent".into(),
                args: serde_json::json!({}),
                risk: Risk::High,
            },
        ];
    }
    if s.contains("cve") || s.contains("vulnerabilit") {
        return vec![PlanStep {
            tool: "sec.cve_check".into(),
            agent: "security-agent".into(),
            args: serde_json::json!({}),
            risk: Risk::Low,
        }];
    }
    if s.contains("security")
        && (s.contains("event") || s.contains("alert") || s.contains("threat"))
    {
        return vec![PlanStep {
            tool: "sec.triage".into(),
            agent: "security-agent".into(),
            args: serde_json::json!({}),
            risk: Risk::Low,
        }];
    }

    // System
    if s.contains("docker") || s.contains("container") {
        return vec![PlanStep {
            tool: "docker.ps".into(),
            agent: "system-agent".into(),
            args: serde_json::json!({}),
            risk: Risk::Low,
        }];
    }
    if s.contains("process") || s.contains("cpu") || s.contains("top") {
        return vec![PlanStep {
            tool: "process.list".into(),
            agent: "system-agent".into(),
            args: serde_json::json!({}),
            risk: Risk::Low,
        }];
    }
    if s.contains("system") || s.contains("health") || s.contains("status") {
        return vec![
            PlanStep {
                tool: "process.list".into(),
                agent: "system-agent".into(),
                args: serde_json::json!({}),
                risk: Risk::Low,
            },
            PlanStep {
                tool: "docker.ps".into(),
                agent: "system-agent".into(),
                args: serde_json::json!({}),
                risk: Risk::Low,
            },
        ];
    }
    if s.contains("disk") || s.contains("storage") || s.contains("space") {
        return vec![PlanStep {
            tool: "shell.run_safe".into(),
            agent: "system-agent".into(),
            args: serde_json::json!({"cmd": "df"}),
            risk: Risk::Medium,
        }];
    }

    // Network
    if s.contains("network") || s.contains("connection") || s.contains("port") {
        return vec![PlanStep {
            tool: "net.status".into(),
            agent: "network-agent".into(),
            args: serde_json::json!({}),
            risk: Risk::Low,
        }];
    }
    if s.contains("firewall") {
        return vec![PlanStep {
            tool: "firewall.rules".into(),
            agent: "network-agent".into(),
            args: serde_json::json!({}),
            risk: Risk::High,
        }];
    }
    if s.contains("dns") || s.contains("pihole") || s.contains("pi-hole") || s.contains("blocked") {
        return vec![PlanStep {
            tool: "pihole.stats".into(),
            agent: "infra-agent".into(),
            args: serde_json::json!({}),
            risk: Risk::Low,
        }];
    }

    // Storage
    if s.contains("nas") || s.contains("truenas") || s.contains("pool") || s.contains("smart") {
        return vec![PlanStep {
            tool: "nas.health".into(),
            agent: "storage-agent".into(),
            args: serde_json::json!({}),
            risk: Risk::Low,
        }];
    }
    if s.contains("snapshot") || s.contains("backup") {
        return vec![PlanStep {
            tool: "nas.snapshot_list".into(),
            agent: "storage-agent".into(),
            args: serde_json::json!({}),
            risk: Risk::Low,
        }];
    }

    // Cameras
    if s.contains("camera")
        || s.contains("cctv")
        || s.contains("surveillance")
        || s.contains("front door")
        || s.contains("driveway")
    {
        return vec![PlanStep {
            tool: "cam.events".into(),
            agent: "surveillance-agent".into(),
            args: serde_json::json!({}),
            risk: Risk::Low,
        }];
    }
    if s.contains("who") && (s.contains("door") || s.contains("outside") || s.contains("porch")) {
        return vec![PlanStep {
            tool: "cam.search".into(),
            agent: "surveillance-agent".into(),
            args: serde_json::json!({"label": "person"}),
            risk: Risk::Low,
        }];
    }

    // Code
    if s.contains("code") || s.contains("debug") || s.contains("lint") || s.contains("clippy") {
        return vec![PlanStep {
            tool: "code.analyze".into(),
            agent: "coding-agent".into(),
            args: serde_json::json!({}),
            risk: Risk::Low,
        }];
    }
    if s.contains("test") {
        return vec![PlanStep {
            tool: "code.test".into(),
            agent: "coding-agent".into(),
            args: serde_json::json!({}),
            risk: Risk::Medium,
        }];
    }

    // Default: research
    vec![PlanStep {
        tool: "research.query".into(),
        agent: "research-agent".into(),
        args: serde_json::json!({"query": intent}),
        risk: Risk::Low,
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_json_array_plain() {
        let input = r#"[{"tool":"docker.ps","agent":"system-agent","args":{},"risk":"low"}]"#;
        let result = extract_json_array(input);
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0]["tool"], "docker.ps");
    }

    #[test]
    fn test_extract_json_array_markdown_wrapped() {
        let input = "```json\n[{\"tool\":\"fs.read\",\"agent\":\"system-agent\",\"args\":{},\"risk\":\"low\"}]\n```";
        let result = extract_json_array(input);
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.len(), 1);
    }

    #[test]
    fn test_extract_json_array_with_explanation() {
        let input = "Here is the plan:\n[{\"tool\":\"net.status\",\"agent\":\"network-agent\",\"args\":{},\"risk\":\"low\"}]\nThis will check network.";
        let result = extract_json_array(input);
        let parsed: Vec<serde_json::Value> = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed.len(), 1);
    }

    #[test]
    fn test_keyword_fallback_ssh() {
        let steps = plan_keyword_fallback("harden ssh config");
        assert_eq!(steps.len(), 2);
        assert_eq!(steps[0].tool, "sec.ssh_audit");
        assert_eq!(steps[1].tool, "sec.harden_ssh");
    }

    #[test]
    fn test_keyword_fallback_docker() {
        let steps = plan_keyword_fallback("show me docker containers");
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].tool, "docker.ps");
    }

    #[test]
    fn test_keyword_fallback_cameras() {
        let steps = plan_keyword_fallback("who was at the front door");
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].tool, "cam.search");
    }

    #[test]
    fn test_keyword_fallback_pihole() {
        let steps = plan_keyword_fallback("show pihole stats");
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].tool, "pihole.stats");
    }

    #[test]
    fn test_keyword_fallback_unknown() {
        let steps = plan_keyword_fallback("what is the meaning of life");
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].tool, "research.query");
    }
}
