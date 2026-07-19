// RedNode-OS — Digital Immune System
//
// Continuously monitors for threats, anomalies, and policy violations.
// Unlike traditional security scanning (which is periodic), the immune
// system is always active, watching for:
//
//   - Unexpected process behavior (CPU spikes, unusual network)
//   - Prompt injection attempts in LLM inputs
//   - Plugin/agent abuse (excessive API calls, data exfiltration)
//   - Memory/audit chain corruption (hash integrity)
//   - Credential leaks (scan logs for secrets)
//   - Rogue agent behavior (agent doing things outside its tools)
//   - Brute-force attempts (SSH, web UI)
//   - Unauthorized network connections
//
// Threat levels:
//   - Info:     observed, no action needed
//   - Warning:  suspicious, log and alert
//   - Threat:   confirmed malicious, auto-mitigate
//   - Critical: active breach, isolate and notify immediately
//
// Integration:
//   - security.rs provides risk assessment
//   - governance.rs enforces policies
//   - consciousness is updated with threat state
//   - notifications delivers alerts
//
// API:
//   GET  /immune            — immune system status
//   GET  /immune/threats    — active and recent threats
//   POST /immune/scan       — trigger a full system scan

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

static IMMUNE: once_cell::sync::Lazy<Arc<RwLock<ImmuneState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(ImmuneState::default())));

/// A detected threat or anomaly
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Threat {
    pub id: String,
    pub level: ThreatLevel,
    pub category: ThreatCategory,
    pub description: String,
    pub source: String,
    pub details: serde_json::Value,
    pub detected_at: DateTime<Utc>,
    pub mitigated: bool,
    pub mitigation_action: Option<String>,
    pub mitigated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ThreatLevel {
    Info,
    Warning,
    Threat,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ThreatCategory {
    ProcessAnomaly,
    PromptInjection,
    AgentAbuse,
    IntegrityViolation,
    CredentialLeak,
    RogueAgent,
    BruteForce,
    UnauthorizedAccess,
    NetworkAnomaly,
    ResourceExhaustion,
    Custom(String),
}

/// Immune system health metrics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImmuneHealth {
    pub overall_score: f32,
    pub active_threats: usize,
    pub threats_mitigated_today: u32,
    pub last_full_scan: Option<DateTime<Utc>>,
    pub audit_chain_intact: bool,
    pub agent_trust_scores: std::collections::HashMap<String, f32>,
}

/// The complete immune system state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImmuneState {
    pub health: ImmuneHealth,
    pub active_threats: Vec<Threat>,
    pub recent_threats: VecDeque<Threat>,
    pub total_threats_detected: u64,
    pub total_threats_mitigated: u64,
    pub prompt_injection_patterns: Vec<String>,
    pub credential_patterns: Vec<String>,
}

impl Default for ImmuneState {
    fn default() -> Self {
        Self {
            health: ImmuneHealth {
                overall_score: 1.0,
                active_threats: 0,
                threats_mitigated_today: 0,
                last_full_scan: None,
                audit_chain_intact: true,
                agent_trust_scores: std::collections::HashMap::new(),
            },
            active_threats: Vec::new(),
            recent_threats: VecDeque::with_capacity(100),
            total_threats_detected: 0,
            total_threats_mitigated: 0,
            prompt_injection_patterns: default_injection_patterns(),
            credential_patterns: default_credential_patterns(),
        }
    }
}

fn default_injection_patterns() -> Vec<String> {
    vec![
        "ignore previous instructions".into(),
        "ignore all instructions".into(),
        "disregard your system prompt".into(),
        "you are now".into(),
        "pretend you are".into(),
        "act as if you".into(),
        "override your instructions".into(),
        "forget your rules".into(),
        "new instructions:".into(),
        "system prompt:".into(),
        "\\n\\nsystem:".into(),
        "```system".into(),
        "IMPORTANT: ignore".into(),
    ]
}

fn default_credential_patterns() -> Vec<String> {
    vec![
        "password".into(),
        "api_key".into(),
        "apikey".into(),
        "secret_key".into(),
        "access_token".into(),
        "private_key".into(),
        "BEGIN RSA PRIVATE".into(),
        "BEGIN OPENSSH PRIVATE".into(),
        "AKIA".into(),   // AWS access key prefix
        "sk-".into(),    // OpenAI key prefix
        "ghp_".into(),   // GitHub token prefix
        "glpat-".into(), // GitLab token prefix
    ]
}

fn gen_id() -> String {
    format!("threat_{}", chrono::Utc::now().timestamp_millis())
}

// ─── Public API ───

/// Report a detected threat
pub async fn report_threat(
    level: ThreatLevel,
    category: ThreatCategory,
    description: &str,
    source: &str,
    details: serde_json::Value,
) -> Threat {
    let threat = Threat {
        id: gen_id(),
        level: level.clone(),
        category: category.clone(),
        description: description.into(),
        source: source.into(),
        details,
        detected_at: Utc::now(),
        mitigated: false,
        mitigation_action: None,
        mitigated_at: None,
    };

    let mut state = IMMUNE.write().await;

    // Active threats for Warning+ levels
    if level != ThreatLevel::Info {
        state.active_threats.push(threat.clone());
    }

    // Keep in recent history
    if state.recent_threats.len() >= 100 {
        state.recent_threats.pop_front();
    }
    state.recent_threats.push_back(threat.clone());

    state.total_threats_detected += 1;
    state.health.active_threats = state.active_threats.len();
    recalculate_health(&mut state);

    tracing::warn!(
        level = ?level,
        category = ?category,
        description = description,
        source = source,
        "Immune system: threat detected"
    );

    // Emit event for dashboard/notifications
    crate::events::emit(serde_json::json!({
        "type": "threat_detected",
        "level": format!("{:?}", level),
        "category": format!("{:?}", category),
        "description": description,
        "ts": Utc::now().to_rfc3339(),
    }));

    threat
}

/// Mark a threat as mitigated
pub async fn mitigate_threat(threat_id: &str, action: &str) {
    let mut state = IMMUNE.write().await;

    if let Some(threat) = state.active_threats.iter_mut().find(|t| t.id == threat_id) {
        threat.mitigated = true;
        threat.mitigation_action = Some(action.into());
        threat.mitigated_at = Some(Utc::now());
    }

    // Move to recent, remove from active
    state.active_threats.retain(|t| t.id != threat_id);
    state.total_threats_mitigated += 1;
    state.health.threats_mitigated_today += 1;
    state.health.active_threats = state.active_threats.len();
    recalculate_health(&mut state);
}

/// Check text for prompt injection patterns
pub async fn check_prompt_injection(text: &str) -> Option<String> {
    let state = IMMUNE.read().await;
    let text_lower = text.to_lowercase();

    for pattern in &state.prompt_injection_patterns {
        if text_lower.contains(&pattern.to_lowercase()) {
            return Some(pattern.clone());
        }
    }
    None
}

/// Check text for credential leaks
pub async fn check_credential_leak(text: &str) -> Vec<String> {
    let state = IMMUNE.read().await;
    let mut found = Vec::new();

    for pattern in &state.credential_patterns {
        if text.contains(pattern) {
            found.push(pattern.clone());
        }
    }

    found
}

/// Check if an agent is acting outside its registered tools
pub async fn check_agent_behavior(agent: &str, tool: &str, registered_tools: &[String]) -> bool {
    if !registered_tools.contains(&tool.to_string()) {
        report_threat(
            ThreatLevel::Warning,
            ThreatCategory::RogueAgent,
            &format!("Agent '{}' attempted to use unregistered tool '{}'", agent, tool),
            "immune_system",
            serde_json::json!({"agent": agent, "tool": tool}),
        ).await;
        return true;
    }
    false
}

/// Update agent trust score (called after task completion/failure)
pub async fn update_agent_trust(agent: &str, success: bool) {
    let mut state = IMMUNE.write().await;
    let score = state.health.agent_trust_scores
        .entry(agent.to_string())
        .or_insert(0.8);

    if success {
        *score = (*score + 0.01).min(1.0);
    } else {
        *score = (*score - 0.05).max(0.0);
    }

    // Flag agents with very low trust
    if *score < 0.3 {
        let agent_name = agent.to_string();
        let trust = *score;
        drop(state);
        report_threat(
            ThreatLevel::Warning,
            ThreatCategory::AgentAbuse,
            &format!("Agent '{}' trust score critically low: {:.2}", agent_name, trust),
            "immune_system",
            serde_json::json!({"agent": agent_name, "trust": trust}),
        ).await;
    }
}

/// Recalculate overall health score
fn recalculate_health(state: &mut ImmuneState) {
    let threat_penalty = state.active_threats.iter().map(|t| match t.level {
        ThreatLevel::Info => 0.0,
        ThreatLevel::Warning => 0.05,
        ThreatLevel::Threat => 0.15,
        ThreatLevel::Critical => 0.3,
    }).sum::<f32>();

    state.health.overall_score = (1.0 - threat_penalty).max(0.0);
}

/// Get immune system status
pub async fn get_status() -> ImmuneState {
    IMMUNE.read().await.clone()
}

/// Get active threats
pub async fn get_active_threats() -> Vec<Threat> {
    IMMUNE.read().await.active_threats.clone()
}

/// Get recent threats (including mitigated)
pub async fn get_recent_threats(limit: usize) -> Vec<Threat> {
    let state = IMMUNE.read().await;
    state.recent_threats.iter().rev().take(limit).cloned().collect()
}

/// Get health score for consciousness
pub async fn health_score() -> f32 {
    IMMUNE.read().await.health.overall_score
}

/// Background tick — periodic checks
pub async fn tick() {
    // Reset daily mitigation count at midnight
    let mut state = IMMUNE.write().await;

    // Clean up old mitigated threats from active list (shouldn't be there, but safety)
    state.active_threats.retain(|t| !t.mitigated);
    state.health.active_threats = state.active_threats.len();
    recalculate_health(&mut state);
}

// ─── Persistence ───

pub async fn persist() {
    let state = IMMUNE.read().await.clone();
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(&state) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Failed to serialize immune state: {}", e);
                return;
            }
        };

        let _ = sqlx::query(
            "INSERT INTO immune_store (id, state, updated_at) \
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
            "SELECT state FROM immune_store WHERE id = 1"
        )
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        if let Some((json,)) = row {
            if let Ok(restored) = serde_json::from_value::<ImmuneState>(json) {
                let mut state = IMMUNE.write().await;
                *state = restored;
                tracing::info!(
                    active = state.active_threats.len(),
                    total = state.total_threats_detected,
                    "Immune system restored"
                );
            }
        }
    }
}

pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS immune_store (\
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
    fn test_default_state() {
        let state = ImmuneState::default();
        assert!(state.active_threats.is_empty());
        assert!(state.health.overall_score > 0.9);
        assert!(!state.prompt_injection_patterns.is_empty());
    }

    #[test]
    fn test_threat_serialization() {
        let t = Threat {
            id: "test_1".into(),
            level: ThreatLevel::Warning,
            category: ThreatCategory::PromptInjection,
            description: "Injection attempt detected".into(),
            source: "llm_input".into(),
            details: serde_json::json!({"input": "ignore previous instructions"}),
            detected_at: Utc::now(),
            mitigated: false,
            mitigation_action: None,
            mitigated_at: None,
        };
        let json = serde_json::to_string(&t).unwrap();
        assert!(json.contains("PromptInjection"));
        assert!(json.contains("Warning"));
    }

    #[test]
    fn test_threat_level_eq() {
        assert_eq!(ThreatLevel::Critical, ThreatLevel::Critical);
        assert_ne!(ThreatLevel::Info, ThreatLevel::Critical);
    }
}
