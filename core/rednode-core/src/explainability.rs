// RedNode-OS — Explainability Engine
//
// Every autonomous decision explains: why it was made, alternatives
// considered, evidence used, confidence level, assumptions made.
//
// API:
//   GET  /explain/:decision_id — get explanation for a decision
//   GET  /explain/recent       — recent explanations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

static EXPLAIN: once_cell::sync::Lazy<Arc<RwLock<ExplainState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(ExplainState::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Explanation {
    pub id: String,
    pub decision_id: String,
    pub action: String,
    pub why: String,
    pub alternatives_considered: Vec<Alternative>,
    pub evidence: Vec<Evidence>,
    pub confidence: f32,
    pub assumptions: Vec<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Alternative { pub action: String, pub reason_rejected: String, pub estimated_score: f32 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence { pub source: String, pub content: String, pub trust_score: f32 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExplainState { pub explanations: VecDeque<Explanation>, pub total: u64 }

impl Default for ExplainState { fn default() -> Self { Self { explanations: VecDeque::with_capacity(200), total: 0 } } }

fn gen_id() -> String { format!("expl_{}", chrono::Utc::now().timestamp_millis()) }

/// Record an explanation for a decision
pub async fn record(decision_id: &str, action: &str, why: &str, alternatives: Vec<Alternative>, evidence: Vec<Evidence>, confidence: f32, assumptions: Vec<String>) -> Explanation {
    let mut state = EXPLAIN.write().await;
    let e = Explanation { id: gen_id(), decision_id: decision_id.into(), action: action.into(), why: why.into(), alternatives_considered: alternatives, evidence, confidence, assumptions, timestamp: Utc::now() };
    if state.explanations.len() >= 200 { state.explanations.pop_front(); }
    state.explanations.push_back(e.clone());
    state.total += 1;
    e
}

pub async fn get_for_decision(decision_id: &str) -> Option<Explanation> {
    EXPLAIN.read().await.explanations.iter().find(|e| e.decision_id == decision_id).cloned()
}

pub async fn get_recent(limit: usize) -> Vec<Explanation> {
    EXPLAIN.read().await.explanations.iter().rev().take(limit).cloned().collect()
}

pub async fn persist() { let s = EXPLAIN.read().await.clone(); if let Some(pool) = crate::memory::pool() { let json = match serde_json::to_value(&s) { Ok(v) => v, Err(_) => return }; let _ = sqlx::query("INSERT INTO explainability_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await; } }
pub async fn restore() { if let Some(pool) = crate::memory::pool() { let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM explainability_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None); if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<ExplainState>(json) { let mut s = EXPLAIN.write().await; *s = r; } } } }
pub async fn init_table() { if let Some(pool) = crate::memory::pool() { let _ = sqlx::query("CREATE TABLE IF NOT EXISTS explainability_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await; } }

#[cfg(test)]
mod tests { use super::*; #[test] fn test_default() { let s = ExplainState::default(); assert_eq!(s.total, 0); } }
