// RedNode-OS — Adaptive Architecture
//
// Allows RedNode to propose redesigning its own architecture when a
// better modular design is discovered, subject to benchmarking and approval.
//
// API:
//   POST /architecture/propose  — propose an architectural change
//   GET  /architecture/proposals — past proposals
//   GET  /architecture/current   — current architecture description

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

static ARCH: once_cell::sync::Lazy<Arc<RwLock<AdaptiveArchState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(AdaptiveArchState::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchProposal {
    pub id: String, pub title: String, pub description: String,
    pub current_design: String, pub proposed_design: String,
    pub rationale: String, pub estimated_improvement: f32,
    pub risks: Vec<String>, pub status: ProposalStatus,
    pub sandbox_result: Option<String>, pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ProposalStatus { Draft, SandboxTesting, Approved, Rejected, Implemented }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdaptiveArchState {
    pub proposals: Vec<ArchProposal>, pub total: u64,
    pub current_module_count: u32, pub current_endpoint_count: u32,
}

impl Default for AdaptiveArchState {
    fn default() -> Self { Self { proposals: Vec::new(), total: 0, current_module_count: 67, current_endpoint_count: 120 } }
}

fn gen_id() -> String { format!("arch_{}", chrono::Utc::now().timestamp_millis()) }

pub async fn propose(title: &str, description: &str, current: &str, proposed: &str, rationale: &str, improvement: f32, risks: Vec<String>) -> ArchProposal {
    let mut state = ARCH.write().await;
    let p = ArchProposal { id: gen_id(), title: title.into(), description: description.into(), current_design: current.into(), proposed_design: proposed.into(), rationale: rationale.into(), estimated_improvement: improvement, risks, status: ProposalStatus::Draft, sandbox_result: None, timestamp: Utc::now() };
    state.proposals.push(p.clone());
    state.total += 1;
    p
}

pub async fn get_proposals() -> Vec<ArchProposal> { ARCH.read().await.proposals.clone() }
pub async fn get_current() -> serde_json::Value {
    let s = ARCH.read().await;
    serde_json::json!({ "modules": s.current_module_count, "endpoints": s.current_endpoint_count, "proposals": s.proposals.len() })
}

pub async fn persist() { let s = ARCH.read().await.clone(); if let Some(pool) = crate::memory::pool() { let json = match serde_json::to_value(&s) { Ok(v) => v, Err(_) => return }; let _ = sqlx::query("INSERT INTO adaptive_arch_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await; } }
pub async fn restore() { if let Some(pool) = crate::memory::pool() { let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM adaptive_arch_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None); if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<AdaptiveArchState>(json) { let mut s = ARCH.write().await; *s = r; } } } }
pub async fn init_table() { if let Some(pool) = crate::memory::pool() { let _ = sqlx::query("CREATE TABLE IF NOT EXISTS adaptive_arch_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await; } }

#[cfg(test)]
mod tests { use super::*; #[test] fn test_default() { let s = AdaptiveArchState::default(); assert!(s.current_module_count > 0); } #[test] fn test_status_eq() { assert_eq!(ProposalStatus::Draft, ProposalStatus::Draft); } }
