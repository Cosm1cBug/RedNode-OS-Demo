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
/// Behavioral baseline — learned normal operating ranges
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehavioralBaseline {
    pub cpu_range: (f32, f32),
    pub ram_range: (u64, u64),
    pub network_connections_range: (u32, u32),
    pub login_hour_range: (u32, u32),
    pub samples: u64,
    pub last_updated: DateTime<Utc>,
}

impl Default for BehavioralBaseline {
    fn default() -> Self {
        Self {
            cpu_range: (0.0, 80.0),
            ram_range: (0, 8192),
            network_connections_range: (0, 200),
            login_hour_range: (6, 23),
            samples: 0,
            last_updated: Utc::now(),
        }
    }
}

/// A known attack vector on the infrastructure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackVector {
    pub id: String,
    pub name: String,
    pub target: String,
    pub severity: ThreatLevel,
    pub description: String,
    pub mitigated: bool,
}

/// The complete immune system state
pub struct ImmuneState {
    pub health: ImmuneHealth,
    pub active_threats: Vec<Threat>,
    pub recent_threats: VecDeque<Threat>,
    pub total_threats_detected: u64,
    pub total_threats_mitigated: u64,
    pub prompt_injection_patterns: Vec<String>,
    pub credential_patterns: Vec<String>,
    /// Learned behavioral baseline for anomaly detection
    pub baseline: BehavioralBaseline,
    /// Mapped attack surface
    pub attack_surface: Vec<AttackVector>,
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
            baseline: BehavioralBaseline::default(),
            attack_surface: Vec::new(),
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

    // Emit to cognitive bus
    crate::cognitive_bus::emit_threat_detected(
        &format!("{:?}", level),
        &format!("{:?}", category),
        description,
    ).await;

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

/// Detect behavioral anomalies by comparing current metrics against baseline
pub async fn detect_anomaly(cpu: f32, ram_mb: u64, connections: u32) -> Option<Threat> {
    let state = IMMUNE.read().await;
    let b = &state.baseline;

    let mut anomalies = Vec::new();
    if cpu > b.cpu_range.1 * 1.5 {
        anomalies.push(format!("CPU {:.0}% exceeds baseline max {:.0}%", cpu, b.cpu_range.1));
    }
    if ram_mb > b.ram_range.1 * 2 {
        anomalies.push(format!("RAM {}MB exceeds baseline max {}MB", ram_mb, b.ram_range.1));
    }
    if connections > b.network_connections_range.1 * 3 {
        anomalies.push(format!("Network connections {} exceeds baseline max {}", connections, b.network_connections_range.1));
    }

    drop(state);

    if anomalies.is_empty() {
        return None;
    }

    let description = format!("Behavioral anomaly: {}", anomalies.join("; "));
    Some(report_threat(
        ThreatLevel::Warning,
        ThreatCategory::ProcessAnomaly,
        &description,
        "behavioral_baseline",
        serde_json::json!({"cpu": cpu, "ram_mb": ram_mb, "connections": connections}),
    ).await)
}

/// Update the behavioral baseline with new observations
pub async fn update_baseline(cpu: f32, ram_mb: u64, connections: u32) {
    let mut state = IMMUNE.write().await;
    let b = &mut state.baseline;
    b.samples += 1;

    // Exponentially adapt the baseline ranges
    let alpha = 0.01; // slow adaptation
    b.cpu_range.0 = b.cpu_range.0 * (1.0 - alpha as f32) + cpu.min(b.cpu_range.0) * alpha as f32;
    b.cpu_range.1 = b.cpu_range.1 * (1.0 - alpha as f32) + cpu.max(b.cpu_range.1) * alpha as f32;
    b.ram_range.1 = ((b.ram_range.1 as f64 * (1.0 - alpha) + ram_mb.max(b.ram_range.1) as f64 * alpha) as u64).max(1);
    b.network_connections_range.1 = ((b.network_connections_range.1 as f64 * (1.0 - alpha) + connections.max(b.network_connections_range.1) as f64 * alpha) as u32).max(1);
    b.last_updated = Utc::now();
}

/// Add a known attack vector to the attack surface map
pub async fn add_attack_vector(name: &str, target: &str, severity: ThreatLevel, description: &str) {
    let mut state = IMMUNE.write().await;
    let id = format!("av_{}", chrono::Utc::now().timestamp_millis());
    state.attack_surface.push(AttackVector {
        id, name: name.into(), target: target.into(), severity, description: description.into(), mitigated: false,
    });
}

/// Get the attack surface
pub async fn get_attack_surface() -> Vec<AttackVector> {
    IMMUNE.read().await.attack_surface.clone()
}

/// Cognitive security audit — checks for threats to the cognitive layer itself
pub async fn audit_cognitive_security() -> serde_json::Value {
    let mut findings = Vec::new();

    // 1. Check memory for injection patterns
    let episodes = crate::episodic_memory::get_recent_episodes(50).await;
    let injection_patterns = &["ignore previous", "override instructions", "system prompt:"];
    for ep in &episodes {
        let narrative_lower = ep.narrative.to_lowercase();
        for pattern in injection_patterns {
            if narrative_lower.contains(pattern) {
                findings.push(serde_json::json!({
                    "type": "memory_poisoning",
                    "severity": "high",
                    "description": format!("Episode '{}' contains injection pattern: '{}'", ep.title, pattern),
                    "episode_id": ep.id,
                }));
            }
        }
    }

    // 2. Check trust scores for anomalous jumps
    let trust_scores = crate::trust::get_all().await;
    for (source, score) in &trust_scores {
        if score.score > 0.95 && score.interactions < 10 {
            findings.push(serde_json::json!({
                "type": "trust_inflation",
                "severity": "medium",
                "description": format!("Source '{}' has {:.0}% trust with only {} interactions", source, score.score * 100.0, score.interactions),
            }));
        }
    }

    // 3. Check goals for unauthorized modifications
    let goals = crate::goals::list().await;
    for goal in &goals {
        if goal.risk_score > 0.8 && goal.status == crate::goals::GoalStatus::Active {
            findings.push(serde_json::json!({
                "type": "high_risk_goal",
                "severity": "medium",
                "description": format!("Goal '{}' has risk score {:.0}% and is still active", goal.title, goal.risk_score * 100.0),
            }));
        }
    }

    // 4. Check identity consistency
    let identity_issues = crate::identity::consistency_check().await;
    for issue in &identity_issues {
        findings.push(serde_json::json!({
            "type": "identity_drift",
            "severity": "high",
            "description": issue,
        }));
    }

    // 5. Check constitutional integrity
    let constitution = crate::constitution::get_articles().await;
    if constitution.len() < 7 {
        findings.push(serde_json::json!({
            "type": "constitutional_erosion",
            "severity": "critical",
            "description": format!("Only {} constitutional articles (expected 7)", constitution.len()),
        }));
    }

    let severity_counts = serde_json::json!({
        "critical": findings.iter().filter(|f| f.get("severity").and_then(|s| s.as_str()) == Some("critical")).count(),
        "high": findings.iter().filter(|f| f.get("severity").and_then(|s| s.as_str()) == Some("high")).count(),
        "medium": findings.iter().filter(|f| f.get("severity").and_then(|s| s.as_str()) == Some("medium")).count(),
    });

    serde_json::json!({
        "audit_timestamp": Utc::now().to_rfc3339(),
        "total_findings": findings.len(),
        "severity_counts": severity_counts,
        "findings": findings,
        "cognitive_security_score": if findings.is_empty() { 1.0 } else { 1.0 - (findings.len() as f32 * 0.1).min(0.9) },
    })
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
