// RedNode-OS — Verification Engine
//
// Independently validates that completed tasks actually succeeded
// before marking them complete. Prevents false positives.
//
// API:
//   POST /verify/:task_id  — verify a task result
//   GET  /verify/history   — verification history

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

static VERIFY: once_cell::sync::Lazy<Arc<RwLock<VerificationState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(VerificationState::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub id: String, pub task_id: String, pub tool: String, pub agent: String,
    pub claimed_success: bool, pub verified_success: bool, pub method: VerificationMethod,
    pub evidence: Vec<String>, pub discrepancy: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VerificationMethod { OutputCheck, StateCheck, ReExecution, PeerReview, Manual }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationState { pub results: VecDeque<VerificationResult>, pub total: u64, pub discrepancies: u64, pub false_positive_rate: f32 }

impl Default for VerificationState { fn default() -> Self { Self { results: VecDeque::with_capacity(200), total: 0, discrepancies: 0, false_positive_rate: 0.0 } } }

fn gen_id() -> String { format!("ver_{}", chrono::Utc::now().timestamp_millis()) }

/// Verify a task result
pub async fn verify(task_id: &str, tool: &str, agent: &str, claimed_success: bool, output: &serde_json::Value) -> VerificationResult {
    let mut state = VERIFY.write().await;

    // Basic output verification
    let has_error = output.get("error").is_some();
    let has_ok = output.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    let verified = if claimed_success { has_ok && !has_error } else { !has_ok || has_error };

    let discrepancy = if claimed_success != verified { Some(format!("Claimed {} but verification shows {}", if claimed_success { "success" } else { "failure" }, if verified { "success" } else { "failure" })) } else { None };

    let result = VerificationResult { id: gen_id(), task_id: task_id.into(), tool: tool.into(), agent: agent.into(), claimed_success, verified_success: verified, method: VerificationMethod::OutputCheck, evidence: vec![format!("has_ok={}, has_error={}", has_ok, has_error)], discrepancy: discrepancy.clone(), timestamp: Utc::now() };

    if state.results.len() >= 200 { state.results.pop_front(); }
    state.results.push_back(result.clone());
    state.total += 1;
    if discrepancy.is_some() { state.discrepancies += 1; }
    state.false_positive_rate = state.discrepancies as f32 / state.total as f32;

    result
}

pub async fn get_history(limit: usize) -> Vec<VerificationResult> { VERIFY.read().await.results.iter().rev().take(limit).cloned().collect() }
pub async fn get_stats() -> serde_json::Value { let s = VERIFY.read().await; serde_json::json!({ "total": s.total, "discrepancies": s.discrepancies, "false_positive_rate": s.false_positive_rate }) }

pub async fn persist() { let s = VERIFY.read().await.clone(); if let Some(pool) = crate::memory::pool() { let json = match serde_json::to_value(&s) { Ok(v) => v, Err(_) => return }; let _ = sqlx::query("INSERT INTO verification_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await; } }
pub async fn restore() { if let Some(pool) = crate::memory::pool() { let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM verification_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None); if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<VerificationState>(json) { let mut s = VERIFY.write().await; *s = r; } } } }
pub async fn init_table() { if let Some(pool) = crate::memory::pool() { let _ = sqlx::query("CREATE TABLE IF NOT EXISTS verification_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await; } }

#[cfg(test)]
mod tests { use super::*; #[test] fn test_default() { let s = VerificationState::default(); assert_eq!(s.total, 0); } #[test] fn test_method_eq() { assert_eq!(VerificationMethod::OutputCheck, VerificationMethod::OutputCheck); } }
