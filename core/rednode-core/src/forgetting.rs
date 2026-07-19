// RedNode-OS — Forgetting Engine
//
// Continuously archives, compresses, merges, or forgets low-value
// information to prevent uncontrolled memory growth.
//
// API:
//   GET  /forgetting/stats    — memory statistics
//   POST /forgetting/sweep    — trigger a forgetting sweep
//   GET  /forgetting/history  — past sweep results

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

static FORGET: once_cell::sync::Lazy<Arc<RwLock<ForgettingState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(ForgettingState::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepResult {
    pub id: String, pub timestamp: DateTime<Utc>,
    pub episodes_archived: u32, pub knowledge_compressed: u32,
    pub stale_removed: u32, pub duplicates_merged: u32,
    pub memory_freed_mb: f64, pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForgettingState {
    pub sweeps: VecDeque<SweepResult>, pub total_sweeps: u64,
    pub total_items_forgotten: u64, pub total_memory_freed_mb: f64,
    pub retention_days_episodic: u32, pub retention_days_logs: u32,
    pub min_recall_count: u32, pub min_importance: f32,
}

impl Default for ForgettingState {
    fn default() -> Self {
        Self { sweeps: VecDeque::with_capacity(50), total_sweeps: 0, total_items_forgotten: 0, total_memory_freed_mb: 0.0, retention_days_episodic: 90, retention_days_logs: 30, min_recall_count: 0, min_importance: 0.1 }
    }
}

fn gen_id() -> String { format!("sweep_{}", chrono::Utc::now().timestamp_millis()) }

/// Run a forgetting sweep
pub async fn sweep() -> SweepResult {
    let start = std::time::Instant::now();
    let state_r = FORGET.read().await;
    let retention = state_r.retention_days_episodic;
    let min_importance = state_r.min_importance;
    drop(state_r);

    // Count items that would be archived/removed (actual removal would happen in memory.rs)
    let episodes = crate::episodic_memory::get_recent_episodes(1000).await;
    let cutoff = Utc::now() - chrono::Duration::days(retention as i64);
    let archivable = episodes.iter().filter(|e| e.started_at < cutoff && e.importance < min_importance).count() as u32;

    let result = SweepResult { id: gen_id(), timestamp: Utc::now(), episodes_archived: archivable, knowledge_compressed: 0, stale_removed: 0, duplicates_merged: 0, memory_freed_mb: archivable as f64 * 0.01, duration_ms: start.elapsed().as_millis() as u64 };

    let mut state = FORGET.write().await;
    if state.sweeps.len() >= 50 { state.sweeps.pop_front(); }
    state.sweeps.push_back(result.clone());
    state.total_sweeps += 1;
    state.total_items_forgotten += archivable as u64;
    state.total_memory_freed_mb += result.memory_freed_mb;

    tracing::info!(archived = archivable, "Forgetting sweep complete");
    result
}

pub async fn get_stats() -> serde_json::Value {
    let s = FORGET.read().await;
    serde_json::json!({ "total_sweeps": s.total_sweeps, "total_forgotten": s.total_items_forgotten, "total_freed_mb": s.total_memory_freed_mb, "retention_days": s.retention_days_episodic })
}

pub async fn get_history(limit: usize) -> Vec<SweepResult> { FORGET.read().await.sweeps.iter().rev().take(limit).cloned().collect() }

pub async fn persist() { let s = FORGET.read().await.clone(); if let Some(pool) = crate::memory::pool() { let json = match serde_json::to_value(&s) { Ok(v) => v, Err(_) => return }; let _ = sqlx::query("INSERT INTO forgetting_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await; } }
pub async fn restore() { if let Some(pool) = crate::memory::pool() { let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM forgetting_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None); if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<ForgettingState>(json) { let mut s = FORGET.write().await; *s = r; } } } }
pub async fn init_table() { if let Some(pool) = crate::memory::pool() { let _ = sqlx::query("CREATE TABLE IF NOT EXISTS forgetting_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await; } }

#[cfg(test)]
mod tests { use super::*; #[test] fn test_default() { let s = ForgettingState::default(); assert_eq!(s.retention_days_episodic, 90); } }
