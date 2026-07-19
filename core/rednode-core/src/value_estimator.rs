// RedNode-OS — Value Estimator
//
// Scores every possible action: Benefit, Risk, Cost, Time,
// Learning value, Goal contribution. Planner selects highest-value.
//
// API:
//   POST /value/estimate — estimate value of an action
//   GET  /value/history  — past estimations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

static VALUEST: once_cell::sync::Lazy<Arc<RwLock<ValueEstimatorState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(ValueEstimatorState::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueEstimate {
    pub id: String,
    pub action: String,
    pub benefit: f32,
    pub risk: f32,
    pub cost: f32,
    pub time_cost: f32,
    pub learning_value: f32,
    pub goal_contribution: f32,
    pub composite_value: f32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueEstimatorState { pub history: VecDeque<ValueEstimate>, pub total: u64 }

impl Default for ValueEstimatorState { fn default() -> Self { Self { history: VecDeque::with_capacity(200), total: 0 } } }

fn gen_id() -> String { format!("val_{}", chrono::Utc::now().timestamp_millis()) }

pub async fn estimate(action: &str, benefit: f32, risk: f32, cost: f32, time_cost: f32, learning_value: f32, goal_contribution: f32) -> ValueEstimate {
    let composite = benefit * 0.25 + (1.0 - risk) * 0.20 + (1.0 - cost) * 0.15 + (1.0 - time_cost) * 0.10 + learning_value * 0.15 + goal_contribution * 0.15;
    let mut state = VALUEST.write().await;
    let e = ValueEstimate { id: gen_id(), action: action.into(), benefit, risk, cost, time_cost, learning_value, goal_contribution, composite_value: composite.clamp(0.0, 1.0), timestamp: Utc::now() };
    if state.history.len() >= 200 { state.history.pop_front(); }
    state.history.push_back(e.clone());
    state.total += 1;
    e
}

/// Compare multiple actions and return the highest-value one
pub async fn compare(actions: Vec<(&str, f32, f32, f32, f32, f32, f32)>) -> Option<ValueEstimate> {
    let mut best: Option<ValueEstimate> = None;
    for (action, benefit, risk, cost, time, learn, goal) in actions {
        let est = estimate(action, benefit, risk, cost, time, learn, goal).await;
        if best.as_ref().map_or(true, |b| est.composite_value > b.composite_value) { best = Some(est); }
    }
    best
}

pub async fn get_history(limit: usize) -> Vec<ValueEstimate> { VALUEST.read().await.history.iter().rev().take(limit).cloned().collect() }

pub async fn persist() { let s = VALUEST.read().await.clone(); if let Some(pool) = crate::memory::pool() { let json = match serde_json::to_value(&s) { Ok(v) => v, Err(_) => return }; let _ = sqlx::query("INSERT INTO value_estimator_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await; } }
pub async fn restore() { if let Some(pool) = crate::memory::pool() { let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM value_estimator_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None); if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<ValueEstimatorState>(json) { let mut s = VALUEST.write().await; *s = r; } } } }
pub async fn init_table() { if let Some(pool) = crate::memory::pool() { let _ = sqlx::query("CREATE TABLE IF NOT EXISTS value_estimator_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await; } }

#[cfg(test)]
mod tests { use super::*; #[test] fn test_default() { let s = ValueEstimatorState::default(); assert_eq!(s.total, 0); } }
