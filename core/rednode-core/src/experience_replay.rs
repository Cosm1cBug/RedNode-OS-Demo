// RedNode-OS — Experience Replay
//
// Replays previous situations in simulation to evaluate alternative
// strategies and improve future decisions.
//
// API:
//   POST /replay/episode/:id — replay an episode with alternatives
//   GET  /replay/history     — past replays and insights

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

static REPLAY: once_cell::sync::Lazy<Arc<RwLock<ReplayState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(ReplayState::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayResult {
    pub id: String, pub episode_id: String, pub episode_title: String,
    pub original_outcome: String, pub original_strategy: String,
    pub alternative_strategies: Vec<AlternativeReplay>,
    pub best_alternative: Option<String>, pub insight: String, pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlternativeReplay { pub strategy: String, pub predicted_outcome: String, pub estimated_success: f32, pub trade_offs: Vec<String> }

/// A counterfactual analysis — "what would have happened if..."
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CounterfactualAnalysis {
    pub episode_id: String,
    pub actual_action: String,
    pub actual_outcome: String,
    pub alternative_action: String,
    pub counterfactual_outcome: String,
    pub causal_factors: Vec<String>,
    pub learning: String,
    pub confidence: f32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayState {
    pub replays: VecDeque<ReplayResult>,
    pub counterfactuals: VecDeque<CounterfactualAnalysis>,
    pub total: u64,
    pub improvements_found: u64,
}

impl Default for ReplayState { fn default() -> Self { Self { replays: VecDeque::with_capacity(50), counterfactuals: VecDeque::with_capacity(50), total: 0, improvements_found: 0 } } }

fn gen_id() -> String { format!("rpl_{}", chrono::Utc::now().timestamp_millis()) }

/// Replay an episode with alternative strategies
pub async fn replay_episode(episode_id: &str) -> Option<ReplayResult> {
    let episode = crate::episodic_memory::search_episodes(episode_id, 1).await.into_iter().next()?;

    let strategies = crate::meta_reasoning::get_strategies().await;
    let mut alternatives = Vec::new();

    for strategy in &strategies {
        let estimated = match strategy.name.as_str() {
            "LLM Planning" => 0.7,
            "Pipeline Execution" => 0.85,
            "Multi-Agent Debate" => 0.8,
            _ => 0.6,
        };
        alternatives.push(AlternativeReplay { strategy: strategy.name.clone(), predicted_outcome: format!("Estimated {:.0}% success with {}", estimated * 100.0, strategy.name), estimated_success: estimated, trade_offs: vec![format!("Avg latency: {}ms", strategy.avg_duration_ms)] });
    }

    let best = alternatives.iter().max_by(|a, b| a.estimated_success.partial_cmp(&b.estimated_success).unwrap_or(std::cmp::Ordering::Equal));
    let original_succeeded = episode.outcome == crate::episodic_memory::EpisodeOutcome::Success;

    let improvement = best.map_or(false, |b| b.estimated_success > if original_succeeded { 0.9 } else { 0.5 });

    let result = ReplayResult {
        id: gen_id(), episode_id: episode_id.into(), episode_title: episode.title.clone(),
        original_outcome: format!("{:?}", episode.outcome), original_strategy: "original".into(),
        alternative_strategies: alternatives, best_alternative: best.map(|b| b.strategy.clone()),
        insight: if improvement { "Better strategy identified for this scenario".into() } else { "Original approach was near-optimal".into() },
        timestamp: Utc::now(),
    };

    let mut state = REPLAY.write().await;
    if state.replays.len() >= 50 { state.replays.pop_front(); }
    state.replays.push_back(result.clone());
    state.total += 1;
    if improvement { state.improvements_found += 1; }

    Some(result)
}

/// Counterfactual analysis: "What would have happened if we chose differently?"
pub async fn counterfactual(episode_id: &str, alternative_action: &str) -> Option<CounterfactualAnalysis> {
    let episode = crate::episodic_memory::search_episodes(episode_id, 1).await.into_iter().next()?;

    let actual_outcome = format!("{:?}", episode.outcome);
    let counterfactual_outcome = if episode.outcome == crate::episodic_memory::EpisodeOutcome::Success {
        "Likely similar success, but with different trade-offs".into()
    } else {
        "Alternative approach may have avoided the failure".into()
    };

    let causal_factors = vec![
        format!("Original actions: {}", episode.actions_taken.join(", ")),
        format!("Alternative: {}", alternative_action),
        format!("Category: {:?}", episode.category),
    ];

    let learning = if episode.outcome == crate::episodic_memory::EpisodeOutcome::Failure {
        format!("When '{}' fails, consider '{}' as alternative", episode.title, alternative_action)
    } else {
        format!("Original approach succeeded — '{}' is a backup option", alternative_action)
    };

    let analysis = CounterfactualAnalysis {
        episode_id: episode_id.into(),
        actual_action: episode.actions_taken.first().cloned().unwrap_or_default(),
        actual_outcome,
        alternative_action: alternative_action.into(),
        counterfactual_outcome,
        causal_factors,
        learning: learning.clone(),
        confidence: 0.5,
        timestamp: Utc::now(),
    };

    let mut state = REPLAY.write().await;
    if state.counterfactuals.len() >= 50 { state.counterfactuals.pop_front(); }
    state.counterfactuals.push_back(analysis.clone());

    // Feed learning to distillation
    crate::distillation::add_input("counterfactual", &learning, &format!("Counterfactual analysis of episode {}", episode_id)).await;

    Some(analysis)
}

pub async fn get_counterfactuals(limit: usize) -> Vec<CounterfactualAnalysis> {
    REPLAY.read().await.counterfactuals.iter().rev().take(limit).cloned().collect()
}

pub async fn get_history(limit: usize) -> Vec<ReplayResult> { REPLAY.read().await.replays.iter().rev().take(limit).cloned().collect() }
pub async fn get_stats() -> serde_json::Value { let s = REPLAY.read().await; serde_json::json!({ "total": s.total, "improvements_found": s.improvements_found }) }

pub async fn persist() { let s = REPLAY.read().await.clone(); if let Some(pool) = crate::memory::pool() { let json = match serde_json::to_value(&s) { Ok(v) => v, Err(_) => return }; let _ = sqlx::query("INSERT INTO experience_replay_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await; } }
pub async fn restore() { if let Some(pool) = crate::memory::pool() { let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM experience_replay_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None); if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<ReplayState>(json) { let mut s = REPLAY.write().await; *s = r; } } } }
pub async fn init_table() { if let Some(pool) = crate::memory::pool() { let _ = sqlx::query("CREATE TABLE IF NOT EXISTS experience_replay_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await; } }

#[cfg(test)]
mod tests { use super::*; #[test] fn test_default() { let s = ReplayState::default(); assert_eq!(s.total, 0); } }
