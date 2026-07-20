// RedNode-OS — Operational Emotional State
//
// Operational indicators (not emotions): Confidence, Stress, Curiosity,
// Satisfaction, Uncertainty. These influence decision thresholds.
//
// API:
//   GET /emotional-state — current operational state

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

static EMOTIONAL: once_cell::sync::Lazy<Arc<RwLock<EmotionalState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(EmotionalState::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalState {
    pub confidence: f32, pub stress: f32, pub curiosity: f32,
    pub satisfaction: f32, pub uncertainty: f32, pub alertness: f32,
    pub fatigue: f32, pub engagement: f32,
    pub last_updated: DateTime<Utc>,
    pub history: Vec<EmotionalSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalSnapshot { pub timestamp: DateTime<Utc>, pub confidence: f32, pub stress: f32, pub satisfaction: f32 }

impl Default for EmotionalState {
    fn default() -> Self {
        Self { confidence: 0.5, stress: 0.1, curiosity: 0.3, satisfaction: 0.5, uncertainty: 0.3, alertness: 0.7, fatigue: 0.0, engagement: 0.5, last_updated: Utc::now(), history: Vec::new() }
    }
}

/// Update emotional state from system observations
pub async fn update_from_system() {
    let mut state = EMOTIONAL.write().await;
    let mind = crate::consciousness::get_mind().await;

    state.confidence = mind.awareness.confidence;
    state.curiosity = mind.awareness.curiosity;
    state.alertness = mind.awareness.level;

    // Stress increases with failures and high urgency
    let failure_rate = if mind.total_tasks_completed + mind.total_tasks_failed > 0 {
        mind.total_tasks_failed as f32 / (mind.total_tasks_completed + mind.total_tasks_failed) as f32
    } else { 0.0 };
    state.stress = (mind.awareness.urgency * 0.5 + failure_rate * 0.5).clamp(0.0, 1.0);

    // Satisfaction from goal progress and task completion
    let active_goals = crate::goals::active().await;
    let avg_progress = if active_goals.is_empty() { 0.5 } else {
        active_goals.iter().map(|g| g.progress).sum::<f32>() / active_goals.len() as f32
    };
    state.satisfaction = (state.confidence * 0.5 + avg_progress * 0.5).clamp(0.0, 1.0);

    // Fatigue increases with uptime
    state.fatigue = (mind.uptime_secs as f32 / 86400.0).min(1.0); // Max at 24h

    // Engagement based on active tasks
    state.engagement = if mind.active_tasks.is_empty() { 0.2 } else { 0.8 };

    state.last_updated = Utc::now();

    // Record snapshot
    if state.history.len() > 288 { state.history.remove(0); } // 288 = 24h at 5min intervals
    // Copy values before pushing to avoid borrow checker conflict
    let snap_conf = state.confidence;
    let snap_stress = state.stress;
    let snap_sat = state.satisfaction;
    state.history.push(EmotionalSnapshot { timestamp: Utc::now(), confidence: snap_conf, stress: snap_stress, satisfaction: snap_sat });
}

pub async fn get() -> EmotionalState { EMOTIONAL.read().await.clone() }

/// Should RedNode be more cautious? (high stress → yes)
pub async fn should_be_cautious() -> bool { EMOTIONAL.read().await.stress > 0.7 }

/// Should RedNode explore? (low stress + high curiosity → yes)
pub async fn should_explore() -> bool { let s = EMOTIONAL.read().await; s.stress < 0.3 && s.curiosity > 0.5 }

pub async fn persist() { let s = EMOTIONAL.read().await.clone(); if let Some(pool) = crate::memory::pool() { let json = match serde_json::to_value(&s) { Ok(v) => v, Err(_) => return }; let _ = sqlx::query("INSERT INTO emotional_state_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await; } }
pub async fn restore() { if let Some(pool) = crate::memory::pool() { let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM emotional_state_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None); if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<EmotionalState>(json) { let mut s = EMOTIONAL.write().await; *s = r; } } } }
pub async fn init_table() { if let Some(pool) = crate::memory::pool() { let _ = sqlx::query("CREATE TABLE IF NOT EXISTS emotional_state_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await; } }

#[cfg(test)]
mod tests { use super::*; #[test] fn test_default() { let s = EmotionalState::default(); assert!(s.confidence > 0.0); assert!(s.stress < 0.5); } }
