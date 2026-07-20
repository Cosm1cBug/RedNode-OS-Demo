// RedNode-OS — Multi-Agent Debate
//
// Instead of a single planner deciding everything:
//   Planner proposes → Critic challenges → Security reviews →
//   Economy optimizes → Research validates → Consciousness decides
//
// This reduces bad autonomous decisions by forcing multiple perspectives.
//
// API:
//   POST /debate/start   — start a debate on a proposed action
//   GET  /debate/history  — past debate results

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

static DEBATES: once_cell::sync::Lazy<Arc<RwLock<DebateState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(DebateState::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebateRequest {
    pub topic: String,
    pub proposed_action: String,
    pub context: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebateResult {
    pub id: String,
    pub topic: String,
    pub proposed_action: String,
    pub perspectives: Vec<Perspective>,
    pub consensus: Consensus,
    pub final_decision: String,
    pub confidence: f32,
    pub duration_ms: u64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Perspective {
    pub role: DebateRole,
    pub position: String,
    pub arguments: Vec<String>,
    pub concerns: Vec<String>,
    pub approval: bool,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DebateRole {
    Planner,
    Critic,
    Security,
    Economy,
    Research,
    Governance,
    DevilsAdvocate,
    Historian,
    Scientist,
    Ethicist,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Consensus {
    Unanimous,
    Majority,
    Split,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebateState {
    pub history: VecDeque<DebateResult>,
    pub total_debates: u64,
    pub unanimous_rate: f32,
}

impl Default for DebateState {
    fn default() -> Self {
        Self { history: VecDeque::with_capacity(50), total_debates: 0, unanimous_rate: 0.0 }
    }
}

fn gen_id() -> String { format!("debate_{}", chrono::Utc::now().timestamp_millis()) }

/// Run a multi-agent debate on a proposed action
pub async fn debate(request: DebateRequest) -> DebateResult {
    let start = std::time::Instant::now();

    // Generate perspectives from each role
    let mut perspectives = Vec::new();

    // Planner: proposes and defends
    perspectives.push(Perspective {
        role: DebateRole::Planner,
        position: format!("Propose: {}", request.proposed_action),
        arguments: vec!["This achieves the stated goal".into(), "Steps are logically ordered".into()],
        concerns: vec![],
        approval: true,
        confidence: 0.8,
    });

    // Critic: challenges assumptions
    let action_lower = request.proposed_action.to_lowercase();
    let critic_concerns: Vec<String> = vec![
        if action_lower.contains("delete") || action_lower.contains("remove") {
            Some("Destructive action — is this reversible?".into())
        } else { None },
        if action_lower.contains("restart") || action_lower.contains("reboot") {
            Some("Service disruption — are there dependent services?".into())
        } else { None },
        Some("What is the rollback plan if this fails?".into()),
    ].into_iter().flatten().collect();

    perspectives.push(Perspective {
        role: DebateRole::Critic,
        position: "Challenge assumptions and identify risks".into(),
        arguments: vec!["Every action should have a rollback plan".into()],
        concerns: critic_concerns.clone(),
        approval: critic_concerns.len() < 2,
        confidence: 0.7,
    });

    // Security: reviews for threats
    let security_ok = !action_lower.contains("disable") && !action_lower.contains("bypass");
    perspectives.push(Perspective {
        role: DebateRole::Security,
        position: if security_ok { "No security concerns identified".into() } else { "Security risk detected".into() },
        arguments: vec!["Action reviewed against security policies".into()],
        concerns: if security_ok { vec![] } else { vec!["Action may weaken security posture".into()] },
        approval: security_ok,
        confidence: 0.85,
    });

    // Economy: optimizes cost
    let budget = crate::economy::check_budget(60000, 5).await;
    perspectives.push(Perspective {
        role: DebateRole::Economy,
        position: if budget.allowed { "Within budget constraints".into() } else { "Budget limit concern".into() },
        arguments: vec![format!("CPU remaining: {}ms, API calls remaining: {}", budget.cpu_remaining, budget.api_remaining)],
        concerns: if budget.allowed { vec![] } else { vec![budget.reason.clone()] },
        approval: budget.allowed,
        confidence: 0.9,
    });

    // Governance: checks policies
    let gov_check = crate::governance::check("debate_action", "system", &request.context).await;
    perspectives.push(Perspective {
        role: DebateRole::Governance,
        position: if gov_check.allowed { "No policy violations".into() } else { "Policy violation detected".into() },
        arguments: vec!["Checked against all active governance policies".into()],
        concerns: gov_check.warnings.clone(),
        approval: gov_check.allowed,
        confidence: 0.9,
    });

    // Devil's Advocate: argues against the proposal regardless
    let devils_concerns: Vec<String> = vec![
        "What if this fails in a way we haven't considered?".into(),
        if action_lower.len() > 50 { "Complex action — higher chance of unforeseen side effects".into() }
        else { "Even simple actions can have unexpected consequences".into() },
    ];
    perspectives.push(Perspective {
        role: DebateRole::DevilsAdvocate,
        position: "Arguing against to stress-test the proposal".into(),
        arguments: vec!["Every proposal should survive rigorous challenge".into()],
        concerns: devils_concerns,
        approval: false, // Devil's advocate always votes no — that's the role
        confidence: 0.6,
    });

    // Historian: checks if similar actions failed before
    let similar_episodes = crate::episodic_memory::search_episodes(&request.proposed_action, 3).await;
    let past_failures: Vec<_> = similar_episodes.iter()
        .filter(|e| e.outcome == crate::episodic_memory::EpisodeOutcome::Failure)
        .collect();
    let historian_ok = past_failures.is_empty();
    perspectives.push(Perspective {
        role: DebateRole::Historian,
        position: if historian_ok { "No similar past failures found".into() }
            else { format!("Found {} similar past failures", past_failures.len()) },
        arguments: if historian_ok { vec!["No historical precedent for failure".into()] }
            else { past_failures.iter().map(|e| format!("Past failure: {}", e.title)).collect() },
        concerns: if historian_ok { vec![] }
            else { vec!["Similar actions have failed before — review lessons learned".into()] },
        approval: historian_ok,
        confidence: 0.75,
    });

    // Scientist: demands evidence and reproducibility
    let has_evidence = !request.context.is_null() && request.context != serde_json::json!({});
    perspectives.push(Perspective {
        role: DebateRole::Scientist,
        position: if has_evidence { "Supporting evidence provided".into() } else { "No supporting evidence — decision based on assumptions".into() },
        arguments: vec!["Decisions should be evidence-based and reproducible".into()],
        concerns: if has_evidence { vec![] } else { vec!["No empirical evidence for expected outcome".into()] },
        approval: has_evidence,
        confidence: if has_evidence { 0.8 } else { 0.4 },
    });

    // Ethicist: evaluates against ethics values
    let ethics_eval = crate::ethics::evaluate(&request.proposed_action, "debate evaluation").await;
    let ethics_ok = ethics_eval.recommendation == crate::ethics::EthicalRecommendation::Proceed
        || ethics_eval.recommendation == crate::ethics::EthicalRecommendation::ProceedWithCaution;
    perspectives.push(Perspective {
        role: DebateRole::Ethicist,
        position: format!("Ethics assessment: {:?}", ethics_eval.recommendation),
        arguments: vec![ethics_eval.reasoning.clone()],
        concerns: ethics_eval.values_considered.iter()
            .filter_map(|v| v.concern.clone())
            .collect(),
        approval: ethics_ok,
        confidence: 0.85,
    });

    // Calculate consensus
    let approval_count = perspectives.iter().filter(|p| p.approval).count();
    let total = perspectives.len();
    let consensus = match approval_count {
        n if n == total => Consensus::Unanimous,
        n if n > total / 2 => Consensus::Majority,
        n if n == total / 2 => Consensus::Split,
        _ => Consensus::Blocked,
    };

    let avg_confidence = perspectives.iter().map(|p| p.confidence).sum::<f32>() / total as f32;

    let final_decision = match consensus {
        Consensus::Unanimous => format!("APPROVED: {}", request.proposed_action),
        Consensus::Majority => format!("APPROVED WITH CONCERNS: {}", request.proposed_action),
        Consensus::Split => format!("REQUIRES HUMAN DECISION: {}", request.proposed_action),
        Consensus::Blocked => format!("BLOCKED: {}", request.proposed_action),
    };

    let result = DebateResult {
        id: gen_id(),
        topic: request.topic,
        proposed_action: request.proposed_action,
        perspectives,
        consensus: consensus.clone(),
        final_decision,
        confidence: avg_confidence,
        duration_ms: start.elapsed().as_millis() as u64,
        timestamp: Utc::now(),
    };

    let mut state = DEBATES.write().await;
    if state.history.len() >= 50 { state.history.pop_front(); }
    state.history.push_back(result.clone());
    state.total_debates += 1;

    let unanimous_count = state.history.iter().filter(|d| d.consensus == Consensus::Unanimous).count();
    state.unanimous_rate = unanimous_count as f32 / state.history.len() as f32;

    tracing::info!(consensus = ?consensus, confidence = avg_confidence, "Debate concluded");

    // Emit to cognitive bus
    crate::cognitive_bus::emit(
        crate::cognitive_bus::CognitiveEventType::DebateCompleted,
        "debate",
        serde_json::json!({
            "topic": result.topic,
            "consensus": format!("{:?}", consensus),
            "confidence": avg_confidence,
            "decision": result.final_decision,
        }),
    ).await;

    // Store debate outcome as episodic memory (Problem 9: debates become knowledge)
    crate::episodic_memory::record_episode(
        &format!("Debate: {}", result.topic),
        &result.final_decision,
        crate::episodic_memory::EpisodeCategory::Interaction,
        result.perspectives.iter().map(|p| format!("{:?}", p.role)).collect(),
        vec![result.final_decision.clone()],
        if consensus == Consensus::Unanimous || consensus == Consensus::Majority {
            crate::episodic_memory::EpisodeOutcome::Success
        } else {
            crate::episodic_memory::EpisodeOutcome::PartialSuccess
        },
        vec![format!("Consensus: {:?}, Confidence: {:.0}%", consensus, avg_confidence * 100.0)],
        avg_confidence,
    ).await;

    // Feed debate summary to distillation for knowledge extraction
    crate::distillation::add_input(
        "debate",
        &result.final_decision,
        &format!("Debate on '{}' with {:?} consensus", result.topic, consensus),
    ).await;

    crate::events::emit(serde_json::json!({
        "type": "debate_concluded",
        "consensus": format!("{:?}", consensus),
        "confidence": avg_confidence,
        "ts": Utc::now().to_rfc3339(),
    }));

    result
}

pub async fn get_history(limit: usize) -> Vec<DebateResult> {
    DEBATES.read().await.history.iter().rev().take(limit).cloned().collect()
}

pub async fn get_stats() -> serde_json::Value {
    let state = DEBATES.read().await;
    serde_json::json!({
        "total_debates": state.total_debates,
        "unanimous_rate": state.unanimous_rate,
        "history_count": state.history.len(),
    })
}

pub async fn persist() {
    let state = DEBATES.read().await.clone();
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(&state) { Ok(v) => v, Err(_) => return };
        let _ = sqlx::query("INSERT INTO debate_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await;
    }
}
pub async fn restore() {
    if let Some(pool) = crate::memory::pool() {
        let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM debate_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None);
        if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<DebateState>(json) { let mut s = DEBATES.write().await; *s = r; } }
    }
}
pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query("CREATE TABLE IF NOT EXISTS debate_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_consensus_eq() { assert_eq!(Consensus::Unanimous, Consensus::Unanimous); }
    #[test]
    fn test_default() { let s = DebateState::default(); assert_eq!(s.total_debates, 0); }
}
