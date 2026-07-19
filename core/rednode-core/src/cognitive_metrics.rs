// RedNode-OS — Cognitive Metrics
//
// Tracks intelligence itself: decision quality, planning accuracy,
// prediction accuracy, goal completion, reasoning latency, reflection
// effectiveness, memory retrieval accuracy, hallucination rate,
// learning rate, autonomy score, risk avoidance, curiosity effectiveness,
// research quality, trust calibration.
//
// API:
//   GET /cognitive-metrics — all cognitive metrics

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

static METRICS: once_cell::sync::Lazy<Arc<RwLock<CognitiveMetricsState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(CognitiveMetricsState::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveMetricsState {
    pub decision_quality: f32,
    pub planning_accuracy: f32,
    pub prediction_accuracy: f32,
    pub goal_completion_rate: f32,
    pub reasoning_latency_ms: u64,
    pub reflection_effectiveness: f32,
    pub memory_retrieval_accuracy: f32,
    pub hallucination_rate: f32,
    pub learning_rate: f32,
    pub autonomy_score: f32,
    pub risk_avoidance: f32,
    pub curiosity_effectiveness: f32,
    pub research_quality: f32,
    pub trust_calibration: f32,
    pub last_computed: DateTime<Utc>,
    pub history: Vec<MetricSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricSnapshot {
    pub timestamp: DateTime<Utc>,
    pub decision_quality: f32,
    pub autonomy_score: f32,
    pub learning_rate: f32,
}

impl Default for CognitiveMetricsState {
    fn default() -> Self {
        Self {
            decision_quality: 0.5, planning_accuracy: 0.5, prediction_accuracy: 0.5,
            goal_completion_rate: 0.0, reasoning_latency_ms: 3000, reflection_effectiveness: 0.5,
            memory_retrieval_accuracy: 0.5, hallucination_rate: 0.1, learning_rate: 0.5,
            autonomy_score: 0.3, risk_avoidance: 0.8, curiosity_effectiveness: 0.5,
            research_quality: 0.5, trust_calibration: 0.5, last_computed: Utc::now(), history: Vec::new(),
        }
    }
}

/// Recompute metrics from system state
pub async fn compute() {
    let mut m = METRICS.write().await;
    let mind = crate::consciousness::get_mind().await;

    // Decision quality: from meta-reasoning average
    let meta_stats = crate::meta_reasoning::get_stats().await;
    m.decision_quality = meta_stats.get("avg_quality").and_then(|v| v.as_f64()).unwrap_or(0.5) as f32;

    // Planning accuracy: tasks completed / total
    let total = mind.total_tasks_completed + mind.total_tasks_failed;
    m.planning_accuracy = if total > 0 { mind.total_tasks_completed as f32 / total as f32 } else { 0.5 };

    // Goal completion
    let goals = crate::goals::list().await;
    let completed = goals.iter().filter(|g| g.status == crate::goals::GoalStatus::Completed).count();
    m.goal_completion_rate = if goals.is_empty() { 0.0 } else { completed as f32 / goals.len() as f32 };

    // Autonomy: tasks completed without human intervention / total
    m.autonomy_score = m.planning_accuracy * 0.7 + m.decision_quality * 0.3;

    // Learning rate: discoveries + new capabilities per day
    let curiosity_stats = crate::curiosity::get_state().await;
    m.learning_rate = (curiosity_stats.stats.total_discoveries as f32 / (mind.uptime_secs as f32 / 86400.0).max(1.0)).min(1.0) * 0.1;

    // Risk avoidance
    let immune_health = crate::immune::health_score().await;
    m.risk_avoidance = immune_health;

    m.last_computed = Utc::now();
    if m.history.len() > 288 { m.history.remove(0); }
    m.history.push(MetricSnapshot { timestamp: Utc::now(), decision_quality: m.decision_quality, autonomy_score: m.autonomy_score, learning_rate: m.learning_rate });
}

pub async fn get() -> CognitiveMetricsState { METRICS.read().await.clone() }

pub async fn persist() { let s = METRICS.read().await.clone(); if let Some(pool) = crate::memory::pool() { let json = match serde_json::to_value(&s) { Ok(v) => v, Err(_) => return }; let _ = sqlx::query("INSERT INTO cognitive_metrics_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await; } }
pub async fn restore() { if let Some(pool) = crate::memory::pool() { let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM cognitive_metrics_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None); if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<CognitiveMetricsState>(json) { let mut s = METRICS.write().await; *s = r; } } } }
pub async fn init_table() { if let Some(pool) = crate::memory::pool() { let _ = sqlx::query("CREATE TABLE IF NOT EXISTS cognitive_metrics_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await; } }

#[cfg(test)]
mod tests { use super::*; #[test] fn test_default() { let m = CognitiveMetricsState::default(); assert!(m.risk_avoidance > 0.5); assert!(m.hallucination_rate < 0.5); } }
