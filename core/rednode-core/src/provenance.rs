// RedNode-OS — Provenance Engine
//
// Tracks origin of every piece of knowledge: Source, Time learned,
// Reliability, Verification status.
//
// API:
//   GET  /provenance/:id   — provenance for a knowledge item
//   GET  /provenance/recent — recent provenance records

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

static PROV: once_cell::sync::Lazy<Arc<RwLock<ProvenanceState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(ProvenanceState::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceRecord {
    pub id: String,
    pub knowledge_id: String,
    pub content_summary: String,
    pub source: String,
    pub source_type: String,
    pub learned_at: DateTime<Utc>,
    pub verified: bool,
    pub verified_at: Option<DateTime<Utc>>,
    pub verification_method: Option<String>,
    pub reliability: f32,
    pub cited_by: Vec<String>,
    pub superseded_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceState { pub records: HashMap<String, ProvenanceRecord>, pub recent: VecDeque<String>, pub total: u64 }

impl Default for ProvenanceState { fn default() -> Self { Self { records: HashMap::new(), recent: VecDeque::with_capacity(500), total: 0 } } }

fn gen_id() -> String { format!("prov_{}", chrono::Utc::now().timestamp_millis()) }

pub async fn record(knowledge_id: &str, content: &str, source: &str, source_type: &str, reliability: f32) -> ProvenanceRecord {
    let mut state = PROV.write().await;
    let r = ProvenanceRecord { id: gen_id(), knowledge_id: knowledge_id.into(), content_summary: content.chars().take(200).collect(), source: source.into(), source_type: source_type.into(), learned_at: Utc::now(), verified: false, verified_at: None, verification_method: None, reliability: reliability.clamp(0.0, 1.0), cited_by: vec![], superseded_by: None };
    state.records.insert(knowledge_id.into(), r.clone());
    if state.recent.len() >= 500 { state.recent.pop_front(); }
    state.recent.push_back(knowledge_id.into());
    state.total += 1;
    r
}

pub async fn verify(knowledge_id: &str, method: &str) {
    let mut state = PROV.write().await;
    if let Some(r) = state.records.get_mut(knowledge_id) { r.verified = true; r.verified_at = Some(Utc::now()); r.verification_method = Some(method.into()); r.reliability = (r.reliability + 0.1).min(1.0); }
}

pub async fn get(knowledge_id: &str) -> Option<ProvenanceRecord> { PROV.read().await.records.get(knowledge_id).cloned() }

pub async fn get_recent(limit: usize) -> Vec<ProvenanceRecord> {
    let state = PROV.read().await;
    state.recent.iter().rev().take(limit).filter_map(|id| state.records.get(id).cloned()).collect()
}

pub async fn persist() { let s = PROV.read().await.clone(); if let Some(pool) = crate::memory::pool() { let json = match serde_json::to_value(&s) { Ok(v) => v, Err(_) => return }; let _ = sqlx::query("INSERT INTO provenance_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await; } }
pub async fn restore() { if let Some(pool) = crate::memory::pool() { let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM provenance_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None); if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<ProvenanceState>(json) { let mut s = PROV.write().await; *s = r; } } } }
pub async fn init_table() { if let Some(pool) = crate::memory::pool() { let _ = sqlx::query("CREATE TABLE IF NOT EXISTS provenance_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await; } }

#[cfg(test)]
mod tests { use super::*; #[test] fn test_default() { let s = ProvenanceState::default(); assert_eq!(s.total, 0); } }
