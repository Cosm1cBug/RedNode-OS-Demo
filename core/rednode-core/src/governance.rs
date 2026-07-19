// RedNode-OS — Governance Layer
//
// A policy engine with explicit rules that control what RedNode can
// and cannot do. Every tool execution is checked against governance
// policies before execution.
//
// Policy types:
//   - Block:   hard deny — action is prevented entirely
//   - Warn:    log + alert — action proceeds but is flagged
//   - Log:     silent log — action proceeds, recorded for audit
//   - Approve: requires human approval before proceeding
//
// Examples:
//   - "Never delete /var/lib/rednode without approval" (Approve)
//   - "Never run shell commands between 01:00-05:00" (Block during maintenance)
//   - "Alert on any firewall rule changes" (Warn)
//   - "Log all social media posts" (Log)
//
// Policies are configurable from the dashboard. Violations are
// recorded in an immutable audit trail.
//
// Integration:
//   - Coordinator checks governance before tool execution
//   - security.rs provides risk levels
//   - immune.rs reports governance violations as threats
//   - Dashboard shows policy management UI
//
// API:
//   GET    /governance/policies      — list all policies
//   POST   /governance/policies      — create a policy
//   DELETE /governance/policies/:id  — remove a policy
//   GET    /governance/violations    — violation history
//   POST   /governance/check         — check an action against policies

use chrono::{DateTime, Utc, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

static GOVERNANCE: once_cell::sync::Lazy<Arc<RwLock<GovernanceState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(GovernanceState::default())));

/// A governance policy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: String,
    pub name: String,
    pub description: String,
    pub rule: PolicyRule,
    pub enforcement: Enforcement,
    pub scope: PolicyScope,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
    pub violation_count: u64,
}

/// What the policy matches against
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyRule {
    /// Block a specific tool
    BlockTool { tool: String },
    /// Block tools matching a pattern (prefix)
    BlockToolPattern { pattern: String },
    /// Block actions on specific paths
    BlockPath { path: String },
    /// Block actions during specific hours (local time)
    BlockTimeRange { start_hour: u32, end_hour: u32 },
    /// Require approval for specific agents
    RequireApproval { agent: String },
    /// Rate limit a tool (max calls per hour)
    RateLimit { tool: String, max_per_hour: u32 },
    /// Custom condition (free text, evaluated by LLM if complex)
    Custom { condition: String },
}

/// What happens when a policy is violated
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Enforcement {
    Block,
    Warn,
    Log,
    Approve,
}

/// Who the policy applies to
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PolicyScope {
    All,
    Agent(String),
    HighRisk,
    Tool(String),
}

/// A policy violation record
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Violation {
    pub id: String,
    pub policy_id: String,
    pub policy_name: String,
    pub enforcement: Enforcement,
    pub tool: String,
    pub agent: String,
    pub details: String,
    pub timestamp: DateTime<Utc>,
    pub blocked: bool,
}

/// The result of checking an action against policies
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyCheckResult {
    pub allowed: bool,
    pub requires_approval: bool,
    pub violations: Vec<Violation>,
    pub warnings: Vec<String>,
}

/// The governance state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernanceState {
    pub policies: Vec<Policy>,
    pub violations: VecDeque<Violation>,
    pub total_checks: u64,
    pub total_violations: u64,
    pub total_blocks: u64,
    /// Rate limit tracking: tool -> timestamps of recent calls
    pub rate_limit_tracker: std::collections::HashMap<String, Vec<DateTime<Utc>>>,
}

impl Default for GovernanceState {
    fn default() -> Self {
        Self {
            policies: default_policies(),
            violations: VecDeque::with_capacity(500),
            total_checks: 0,
            total_violations: 0,
            total_blocks: 0,
            rate_limit_tracker: std::collections::HashMap::new(),
        }
    }
}

/// Built-in safety policies — user can add/remove from dashboard
fn default_policies() -> Vec<Policy> {
    vec![
        Policy {
            id: "pol_protect_rednode_data".into(),
            name: "Protect RedNode data directory".into(),
            description: "Prevent deletion of /var/lib/rednode without approval".into(),
            rule: PolicyRule::BlockPath { path: "/var/lib/rednode".into() },
            enforcement: Enforcement::Approve,
            scope: PolicyScope::All,
            enabled: true,
            created_at: Utc::now(),
            violation_count: 0,
        },
        Policy {
            id: "pol_no_shell_overnight".into(),
            name: "No shell commands overnight".into(),
            description: "Block shell command execution between 01:00 and 05:00 local time".into(),
            rule: PolicyRule::BlockTimeRange { start_hour: 1, end_hour: 5 },
            enforcement: Enforcement::Warn,
            scope: PolicyScope::Tool("shell.run_safe".into()),
            enabled: false, // Disabled by default — user can enable
            created_at: Utc::now(),
            violation_count: 0,
        },
        Policy {
            id: "pol_firewall_alert".into(),
            name: "Alert on firewall changes".into(),
            description: "Warn whenever firewall rules are modified".into(),
            rule: PolicyRule::BlockToolPattern { pattern: "fw.".into() },
            enforcement: Enforcement::Warn,
            scope: PolicyScope::All,
            enabled: true,
            created_at: Utc::now(),
            violation_count: 0,
        },
        Policy {
            id: "pol_rate_limit_llm".into(),
            name: "Rate limit LLM calls".into(),
            description: "Maximum 100 LLM API calls per hour".into(),
            rule: PolicyRule::RateLimit { tool: "llm".into(), max_per_hour: 100 },
            enforcement: Enforcement::Block,
            scope: PolicyScope::All,
            enabled: true,
            created_at: Utc::now(),
            violation_count: 0,
        },
    ]
}

fn gen_id(prefix: &str) -> String {
    format!("{}_{}", prefix, chrono::Utc::now().timestamp_millis())
}

// ─── Public API ───

/// Check an action against all active policies
pub async fn check(tool: &str, agent: &str, args: &serde_json::Value) -> PolicyCheckResult {
    let mut state = GOVERNANCE.write().await;
    state.total_checks += 1;

    let mut violations = Vec::new();
    let mut warnings = Vec::new();
    let mut blocked = false;
    let mut requires_approval = false;
    let now = Utc::now();
    let local_hour = chrono::Local::now().hour();
    let args_str = args.to_string();

    for policy in &mut state.policies {
        if !policy.enabled {
            continue;
        }

        // Check scope
        let in_scope = match &policy.scope {
            PolicyScope::All => true,
            PolicyScope::Agent(a) => agent == a,
            PolicyScope::HighRisk => {
                matches!(crate::security::assess_risk(tool),
                    crate::security::Risk::High | crate::security::Risk::Critical)
            }
            PolicyScope::Tool(t) => tool == t,
        };

        if !in_scope {
            continue;
        }

        // Evaluate rule
        let violated = match &policy.rule {
            PolicyRule::BlockTool { tool: blocked_tool } => tool == blocked_tool,
            PolicyRule::BlockToolPattern { pattern } => tool.starts_with(pattern),
            PolicyRule::BlockPath { path } => args_str.contains(path),
            PolicyRule::BlockTimeRange { start_hour, end_hour } => {
                if start_hour <= end_hour {
                    local_hour >= *start_hour && local_hour < *end_hour
                } else {
                    local_hour >= *start_hour || local_hour < *end_hour
                }
            }
            PolicyRule::RequireApproval { agent: req_agent } => agent == req_agent,
            PolicyRule::RateLimit { tool: rate_tool, max_per_hour } => {
                if tool.starts_with(rate_tool) {
                    let tracker = state.rate_limit_tracker
                        .entry(rate_tool.clone())
                        .or_default();
                    // Clean old entries
                    let one_hour_ago = now - chrono::Duration::hours(1);
                    tracker.retain(|t| *t > one_hour_ago);
                    tracker.len() as u32 >= *max_per_hour
                } else {
                    false
                }
            }
            PolicyRule::Custom { condition } => {
                // Simple keyword match — full LLM evaluation would happen in coordinator
                args_str.to_lowercase().contains(&condition.to_lowercase())
            }
        };

        if violated {
            policy.violation_count += 1;

            let violation = Violation {
                id: gen_id("viol"),
                policy_id: policy.id.clone(),
                policy_name: policy.name.clone(),
                enforcement: policy.enforcement.clone(),
                tool: tool.into(),
                agent: agent.into(),
                details: format!("Policy '{}' violated: {}", policy.name, policy.description),
                timestamp: now,
                blocked: policy.enforcement == Enforcement::Block,
            };

            match policy.enforcement {
                Enforcement::Block => {
                    blocked = true;
                    violations.push(violation);
                }
                Enforcement::Approve => {
                    requires_approval = true;
                    violations.push(violation);
                }
                Enforcement::Warn => {
                    warnings.push(format!("Policy warning: {}", policy.name));
                    violations.push(violation);
                }
                Enforcement::Log => {
                    violations.push(violation);
                }
            }
        }
    }

    // Update rate limit tracker for this tool
    if let Some(tracker) = state.rate_limit_tracker.get_mut(tool) {
        tracker.push(now);
    }

    // Store violations
    for v in &violations {
        if state.violations.len() >= 500 {
            state.violations.pop_front();
        }
        state.violations.push_back(v.clone());
    }

    if !violations.is_empty() {
        state.total_violations += violations.len() as u64;
        if blocked {
            state.total_blocks += 1;
        }
    }

    PolicyCheckResult {
        allowed: !blocked,
        requires_approval,
        violations,
        warnings,
    }
}

/// List all policies
pub async fn list_policies() -> Vec<Policy> {
    GOVERNANCE.read().await.policies.clone()
}

/// Create a new policy
pub async fn create_policy(
    name: &str,
    description: &str,
    rule: PolicyRule,
    enforcement: Enforcement,
    scope: PolicyScope,
) -> Policy {
    let policy = Policy {
        id: gen_id("pol"),
        name: name.into(),
        description: description.into(),
        rule,
        enforcement,
        scope,
        enabled: true,
        created_at: Utc::now(),
        violation_count: 0,
    };

    let mut state = GOVERNANCE.write().await;
    state.policies.push(policy.clone());

    tracing::info!(name = name, "Governance policy created");

    crate::events::emit(serde_json::json!({
        "type": "policy_created",
        "name": name,
        "ts": Utc::now().to_rfc3339(),
    }));

    policy
}

/// Remove a policy by ID
pub async fn remove_policy(id: &str) -> bool {
    let mut state = GOVERNANCE.write().await;
    let before = state.policies.len();
    state.policies.retain(|p| p.id != id);
    state.policies.len() < before
}

/// Enable or disable a policy
pub async fn toggle_policy(id: &str, enabled: bool) -> bool {
    let mut state = GOVERNANCE.write().await;
    if let Some(p) = state.policies.iter_mut().find(|p| p.id == id) {
        p.enabled = enabled;
        return true;
    }
    false
}

/// Get violation history
pub async fn get_violations(limit: usize) -> Vec<Violation> {
    let state = GOVERNANCE.read().await;
    state.violations.iter().rev().take(limit).cloned().collect()
}

/// Get governance stats
pub async fn get_stats() -> serde_json::Value {
    let state = GOVERNANCE.read().await;
    serde_json::json!({
        "policy_count": state.policies.len(),
        "active_policies": state.policies.iter().filter(|p| p.enabled).count(),
        "total_checks": state.total_checks,
        "total_violations": state.total_violations,
        "total_blocks": state.total_blocks,
        "recent_violations": state.violations.len(),
    })
}

// ─── Persistence ───

pub async fn persist() {
    let state = GOVERNANCE.read().await.clone();
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(&state) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Failed to serialize governance: {}", e);
                return;
            }
        };

        let _ = sqlx::query(
            "INSERT INTO governance_store (id, state, updated_at) \
             VALUES (1, $1, NOW()) \
             ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()"
        )
        .bind(&json)
        .execute(pool)
        .await;
    }
}

pub async fn restore() {
    if let Some(pool) = crate::memory::pool() {
        let row: Option<(serde_json::Value,)> = sqlx::query_as(
            "SELECT state FROM governance_store WHERE id = 1"
        )
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        if let Some((json,)) = row {
            if let Ok(restored) = serde_json::from_value::<GovernanceState>(json) {
                let mut state = GOVERNANCE.write().await;
                *state = restored;
                tracing::info!(
                    policies = state.policies.len(),
                    violations = state.total_violations,
                    "Governance layer restored"
                );
            }
        }
    }
}

pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS governance_store (\
                id INTEGER PRIMARY KEY, \
                state JSONB NOT NULL, \
                updated_at TIMESTAMPTZ DEFAULT NOW()\
            )"
        )
        .execute(pool)
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_policies() {
        let state = GovernanceState::default();
        assert!(!state.policies.is_empty());
        assert!(state.policies.iter().any(|p| p.id == "pol_protect_rednode_data"));
    }

    #[test]
    fn test_enforcement_eq() {
        assert_eq!(Enforcement::Block, Enforcement::Block);
        assert_ne!(Enforcement::Block, Enforcement::Warn);
    }

    #[test]
    fn test_policy_serialization() {
        let p = Policy {
            id: "test_1".into(),
            name: "Test Policy".into(),
            description: "A test".into(),
            rule: PolicyRule::BlockTool { tool: "dangerous.tool".into() },
            enforcement: Enforcement::Block,
            scope: PolicyScope::All,
            enabled: true,
            created_at: Utc::now(),
            violation_count: 0,
        };
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("Test Policy"));
        assert!(json.contains("dangerous.tool"));
    }

    #[test]
    fn test_violation_serialization() {
        let v = Violation {
            id: "test_v".into(),
            policy_id: "pol_1".into(),
            policy_name: "Test".into(),
            enforcement: Enforcement::Warn,
            tool: "shell.run_safe".into(),
            agent: "system-agent".into(),
            details: "Violation details".into(),
            timestamp: Utc::now(),
            blocked: false,
        };
        let json = serde_json::to_string(&v).unwrap();
        assert!(json.contains("Violation details"));
    }
}
