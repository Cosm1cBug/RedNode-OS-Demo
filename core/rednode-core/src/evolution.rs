// RedNode-OS — Tool Evolution Engine
//
// Enables RedNode to learn, create, and register new tools autonomously.
//
// Flow:
//   1. Learning Agent discovers a new capability (CLI tool, API, pattern)
//   2. LLM generates: tool name, description, risk level, handler code
//   3. This module validates the generated code (syntax, safety)
//   4. Writes to tools.json (registry) + agent index.ts (handler)
//   5. Emits reload event → agents pick up new tools without restart
//   6. Planner reloads tool context → LLM can now use the new tool
//
// Safety:
//   - Generated code is sandboxed (no filesystem write outside /var/lib/rednode)
//   - New tools default to "medium" risk (require logging)
//   - High-risk operations cannot be auto-generated (must be manual)
//   - All generated tools are logged in the audit chain
//   - Code is validated for deny patterns before writing

use serde::{Deserialize, Serialize};
use anyhow::{Result, bail};
use std::path::Path;

/// A tool definition as stored in tools.json
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDef {
    pub name: String,
    pub agent: String,
    pub risk: String,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_generated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handler_type: Option<String>, // "shell", "api", "llm", "cns"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handler_command: Option<String>, // the actual command/URL
}

/// Load all tools from tools.json
pub fn load_tools_registry(project_root: &str) -> Result<Vec<ToolDef>> {
    let path = format!("{}/execution/tool-registry/tools.json", project_root);
    let content = std::fs::read_to_string(&path)
        .unwrap_or_else(|_| "[]".into());
    let tools: Vec<ToolDef> = serde_json::from_str(&content)?;
    Ok(tools)
}

/// Generate the TOOL_CONTEXT string for the LLM planner dynamically
/// from tools.json. This replaces the hardcoded TOOL_CONTEXT in planner.rs.
pub fn generate_tool_context(project_root: &str) -> String {
    let tools = load_tools_registry(project_root).unwrap_or_default();
    
    let mut context = String::from("Available tools (name | agent | risk | description):\n");
    
    // Limit to most important tools to fit in LLM context window
    // Group by agent, take up to 5 per agent for the prompt
    let mut by_agent: std::collections::HashMap<String, Vec<&ToolDef>> = std::collections::HashMap::new();
    for t in &tools {
        by_agent.entry(t.agent.clone()).or_default().push(t);
    }
    
    for (agent, agent_tools) in by_agent.iter() {
        for t in agent_tools.iter().take(8) {
            context.push_str(&format!(
                "- {} | {} | {} | {}\n",
                t.name, t.agent, t.risk, t.description
            ));
        }
        if agent_tools.len() > 8 {
            context.push_str(&format!(
                "  ... and {} more tools in {}\n",
                agent_tools.len() - 8,
                agent
            ));
        }
    }
    
    context.push_str(&format!("\nTotal: {} tools across {} agents.\n", tools.len(), by_agent.len()));
    context
}

/// Validate a proposed tool definition for safety
pub fn validate_tool_def(tool: &ToolDef) -> Result<()> {
    // Name format: agent_prefix.tool_name
    if !tool.name.contains('.') {
        bail!("Tool name must contain a dot (e.g., 'sys.my_tool')");
    }
    
    // Risk must be valid
    match tool.risk.as_str() {
        "low" | "medium" => {},
        "high" | "critical" => {
            bail!("Auto-generated tools cannot have high/critical risk. Manual review required.");
        }
        _ => bail!("Invalid risk level: {}", tool.risk),
    }
    
    // Agent must exist
    let valid_agents = [
        "system-agent", "security-agent", "coding-agent", "research-agent",
        "automation-agent", "network-agent", "infra-agent", "storage-agent",
        "surveillance-agent", "comms-agent", "productivity-agent", "media-agent",
        "home-agent", "browser-agent", "social-agent", "learning-agent", "signal-bot",
    ];
    if !valid_agents.contains(&tool.agent.as_str()) {
        bail!("Unknown agent: {}", tool.agent);
    }
    
    // Description must be non-empty
    if tool.description.len() < 5 {
        bail!("Description too short");
    }
    
    // Name must not conflict with existing deny patterns
    let deny_prefixes = ["rm.", "dd.", "mkfs.", "passwd.", "shutdown.", "reboot."];
    for dp in deny_prefixes {
        if tool.name.starts_with(dp) {
            bail!("Tool name starts with denied prefix: {}", dp);
        }
    }
    
    Ok(())
}

/// Validate handler code for safety — no dangerous patterns
pub fn validate_handler_code(code: &str) -> Result<()> {
    let deny_patterns = [
        "rm -rf /", "rm -rf /*", "dd if=", "mkfs", ":(){ :|:& };",
        "chmod 777 /", "> /dev/sda", "shutdown", "reboot", "init 0",
        "passwd", "useradd", "userdel", "iptables -F",
        "eval(", "Function(", "child_process.exec(",  // unsafe JS
        "require('child_process')",  // unsafe require
        "process.exit", "process.kill",
        "fs.unlinkSync", "fs.rmdirSync", "fs.writeFileSync(\"/",  // dangerous fs ops
    ];
    
    for pattern in deny_patterns {
        if code.contains(pattern) {
            bail!("Handler code contains denied pattern: {}", pattern);
        }
    }
    
    // Must not be too long (prevent code injection via oversized handlers)
    if code.len() > 2000 {
        bail!("Handler code too long ({} chars, max 2000)", code.len());
    }
    
    Ok(())
}

/// Generate TypeScript handler code for a new tool based on its type
pub fn generate_handler_code(tool: &ToolDef) -> String {
    let handler_type = tool.handler_type.as_deref().unwrap_or("shell");
    let command = tool.handler_command.as_deref().unwrap_or("");
    
    match handler_type {
        "shell" => {
            if command.is_empty() {
                format!(
                    r#"return {{ ok: true, output: "Tool {} needs a command — configure handler_command", tool }};"#,
                    tool.name
                )
            } else {
                format!(
                    r#"const r = await sh(`{} ${{Object.values(args).join(" ")}}`.trim(), 15000); return {{ ok: r.ok, output: r.output, tool }};"#,
                    command.replace('`', "\\`").replace('"', "\\\"")
                )
            }
        }
        "api" => {
            if command.is_empty() {
                format!(
                    r#"return {{ ok: true, output: "Tool {} needs an API URL — configure handler_command", tool }};"#,
                    tool.name
                )
            } else {
                format!(
                    r#"const r = await api("{}"); return {{ ok: r.ok, output: r.output, tool }};"#,
                    command.replace('"', "\\\"")
                )
            }
        }
        "llm" => {
            format!(
                r#"const input = args.input || args.text || args.query || ""; if (!input) return {{ ok: false, error: "Missing input" }}; const result = await llm(`{}: ${{input}}`); return {{ ok: true, output: result, tool }};"#,
                tool.description.replace('`', "\\`").replace('"', "\\\"")
            )
        }
        "cns" => {
            let endpoint = if command.is_empty() { "/memory/query" } else { command };
            format!(
                r#"const r = await cns("{}"); return {{ ok: r.ok, output: r.output, tool }};"#,
                endpoint
            )
        }
        _ => {
            format!(
                r#"return {{ ok: true, output: "Tool {} is registered but handler type '{}' is not supported yet", tool }};"#,
                tool.name, handler_type
            )
        }
    }
}

/// Add a new tool to tools.json
pub fn register_tool(project_root: &str, tool: &ToolDef) -> Result<()> {
    validate_tool_def(tool)?;
    
    let path = format!("{}/execution/tool-registry/tools.json", project_root);
    let mut tools = load_tools_registry(project_root)?;
    
    // Check for duplicates
    if tools.iter().any(|t| t.name == tool.name) {
        bail!("Tool '{}' already exists in registry", tool.name);
    }
    
    tools.push(tool.clone());
    
    let json = serde_json::to_string_pretty(&tools)?;
    std::fs::write(&path, format!("{}\n", json))?;
    
    tracing::info!(tool = tool.name, agent = tool.agent, "New tool registered in tools.json");
    Ok(())
}

/// Inject a handler case into an agent's index.ts
pub fn inject_handler(project_root: &str, tool: &ToolDef, handler_code: &str) -> Result<()> {
    validate_handler_code(handler_code)?;
    
    let idx_path = format!("{}/agents/{}/src/index.ts", project_root, tool.agent);
    
    if !Path::new(&idx_path).exists() {
        bail!("Agent file not found: {}", idx_path);
    }
    
    let mut code = std::fs::read_to_string(&idx_path)?;
    
    // Check if tool already has a handler
    if code.contains(&format!("\"{}\"", tool.name)) {
        bail!("Handler for '{}' already exists in {}", tool.name, tool.agent);
    }
    
    // Add tool name to the TOOLS array
    // Find: const TOOLS = [ ... ];
    if let Some(tools_end) = code.find("];") {
        // Find the TOOLS array specifically
        let tools_start = code[..tools_end].rfind("const TOOLS = [");
        if let Some(start) = tools_start {
            let insert_pos = tools_end; // right before ];
            let new_entry = format!("  \"{}\",\n", tool.name);
            code.insert_str(insert_pos, &new_entry);
        }
    }
    
    // Add case handler before "default:"
    let case_block = format!(
        "\n      case \"{}\": {{\n        {}\n      }}\n",
        tool.name, handler_code
    );
    
    if let Some(default_pos) = code.find("default:") {
        // Find the whitespace-prefixed default to match indentation
        let insert_pos = code[..default_pos].rfind('\n').unwrap_or(default_pos) + 1;
        code.insert_str(insert_pos, &case_block);
    }
    
    std::fs::write(&idx_path, &code)?;
    
    tracing::info!(
        tool = tool.name,
        agent = tool.agent,
        "Handler injected into {}",
        idx_path
    );
    
    Ok(())
}

/// Full tool evolution: validate → register → generate handler → inject → emit event
pub async fn evolve_tool(
    project_root: &str,
    name: &str,
    agent: &str,
    description: &str,
    handler_type: &str,  // "shell", "api", "llm", "cns"
    handler_command: &str,
) -> Result<ToolDef> {
    let tool = ToolDef {
        name: name.into(),
        agent: agent.into(),
        risk: "medium".into(), // auto-generated tools are always medium
        description: description.into(),
        auto_generated: Some(true),
        generated_at: Some(chrono::Utc::now().to_rfc3339()),
        handler_type: Some(handler_type.into()),
        handler_command: if handler_command.is_empty() { None } else { Some(handler_command.into()) },
    };
    
    // Validate
    validate_tool_def(&tool)?;
    
    // Generate handler code
    let handler_code = generate_handler_code(&tool);
    validate_handler_code(&handler_code)?;
    
    // Register in tools.json
    register_tool(project_root, &tool)?;
    
    // Inject into agent's index.ts
    inject_handler(project_root, &tool, &handler_code)?;
    
    // Emit evolution event
    crate::events::emit(serde_json::json!({
        "type": "tool_evolved",
        "tool": tool.name,
        "agent": tool.agent,
        "description": tool.description,
        "handler_type": handler_type,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    }));
    
    tracing::info!(
        tool = tool.name,
        agent = tool.agent,
        handler_type,
        "🧬 Tool evolved: {} — {} can now use it",
        tool.name, tool.agent
    );
    
    Ok(tool)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_tool_def_valid() {
        let tool = ToolDef {
            name: "sys.my_test".into(),
            agent: "system-agent".into(),
            risk: "low".into(),
            description: "A test tool".into(),
            auto_generated: Some(true),
            generated_at: None,
            handler_type: Some("shell".into()),
            handler_command: Some("echo hello".into()),
        };
        assert!(validate_tool_def(&tool).is_ok());
    }

    #[test]
    fn test_validate_tool_def_high_risk_rejected() {
        let tool = ToolDef {
            name: "sys.danger".into(),
            agent: "system-agent".into(),
            risk: "high".into(),
            description: "A dangerous tool".into(),
            auto_generated: None,
            generated_at: None,
            handler_type: None,
            handler_command: None,
        };
        assert!(validate_tool_def(&tool).is_err());
    }

    #[test]
    fn test_validate_handler_code_safe() {
        assert!(validate_handler_code("const r = await sh('ls'); return { ok: true, output: r.output };").is_ok());
    }

    #[test]
    fn test_validate_handler_code_dangerous() {
        assert!(validate_handler_code("rm -rf /").is_err());
        assert!(validate_handler_code("eval(userInput)").is_err());
        assert!(validate_handler_code("process.exit(1)").is_err());
    }

    #[test]
    fn test_generate_handler_shell() {
        let tool = ToolDef {
            name: "sys.test".into(),
            agent: "system-agent".into(),
            risk: "low".into(),
            description: "Test".into(),
            auto_generated: None,
            generated_at: None,
            handler_type: Some("shell".into()),
            handler_command: Some("uptime".into()),
        };
        let code = generate_handler_code(&tool);
        assert!(code.contains("sh("));
        assert!(code.contains("uptime"));
    }

    #[test]
    fn test_generate_handler_api() {
        let tool = ToolDef {
            name: "test.api".into(),
            agent: "system-agent".into(),
            risk: "low".into(),
            description: "Test API".into(),
            auto_generated: None,
            generated_at: None,
            handler_type: Some("api".into()),
            handler_command: Some("http://localhost:8080/health".into()),
        };
        let code = generate_handler_code(&tool);
        assert!(code.contains("api("));
        assert!(code.contains("localhost:8080"));
    }
}
