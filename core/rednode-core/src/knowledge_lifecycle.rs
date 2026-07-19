// RedNode-OS — Knowledge Lifecycle Manager
//
// Knowledge progresses through: Created → Verified → Updated →
// Deprecated → Archived → Deleted. Prevents stale information
// from affecting decisions.
//
// API:
//   GET  /knowledge/lifecycle — lifecycle statistics
//   POST /knowledge/deprecate/:id — mark knowledge as deprecated

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

static LIFECYCLE: once_cell::sync::Lazy<Arc<RwLock<LifecycleState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(LifecycleState::default())));

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum KnowledgeStage { Created, Verified, Updated, Deprecated, Archived, Deleted }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeItem { pub id: String, pub title: String, pub stage: KnowledgeStage, pub created_at: DateTime<Utc>, pub last_transition: DateTime<Utc>, pub transitions: Vec<StageTransition>, pub usage_count: u64 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StageTransition { pub from: KnowledgeStage, pub to: KnowledgeStage, pub reason: String, pub timestamp: DateTime<Utc> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LifecycleState { pub items: HashMap<String, KnowledgeItem>, pub stage_counts: HashMap<String, u64>, pub total_transitions: u64 }

impl Default for LifecycleState { fn default() -> Self { Self { items: HashMap::new(), stage_counts: HashMap::new(), total_transitions: 0 } } }

pub async fn register(id: &str, title: &str) {
    let mut state = LIFECYCLE.write().await;
    state.items.insert(id.into(), KnowledgeItem { id: id.into(), title: title.into(), stage: KnowledgeStage::Created, created_at: Utc::now(), last_transition: Utc::now(), transitions: vec![], usage_count: 0 });
    *state.stage_counts.entry("Created".into()).or_insert(0) += 1;
}

pub async fn transition(id: &str, to: KnowledgeStage, reason: &str) -> bool {
    let mut state = LIFECYCLE.write().await;
    if let Some(item) = state.items.get_mut(id) {
        let from = item.stage.clone();
        item.transitions.push(StageTransition { from, to: to.clone(), reason: reason.into(), timestamp: Utc::now() });
        item.stage = to;
        item.last_transition = Utc::now();
        state.total_transitions += 1;
        return true;
    }
    false
}

pub async fn record_usage(id: &str) {
    let mut state = LIFECYCLE.write().await;
    if let Some(item) = state.items.get_mut(id) { item.usage_count += 1; }
}

pub async fn get_stats() -> serde_json::Value {
    let s = LIFECYCLE.read().await;
    serde_json::json!({ "total_items": s.items.len(), "total_transitions": s.total_transitions, "stage_counts": s.stage_counts })
}

pub async fn persist() { let s = LIFECYCLE.read().await.clone(); if let Some(pool) = crate::memory::pool() { let json = match serde_json::to_value(&s) { Ok(v) => v, Err(_) => return }; let _ = sqlx::query("INSERT INTO knowledge_lifecycle_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await; } }
pub async fn restore() { if let Some(pool) = crate::memory::pool() { let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM knowledge_lifecycle_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None); if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<LifecycleState>(json) { let mut s = LIFECYCLE.write().await; *s = r; } } } }
pub async fn init_table() { if let Some(pool) = crate::memory::pool() { let _ = sqlx::query("CREATE TABLE IF NOT EXISTS knowledge_lifecycle_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await; } }

#[cfg(test)]
mod tests { use super::*; #[test] fn test_stage_eq() { assert_eq!(KnowledgeStage::Created, KnowledgeStage::Created); } }
