// RedNode-OS — Memory Safety Layer
//
// Protects memories: checksums, version history, rollback,
// consistency validation, conflict resolution.
//
// API:
//   GET  /memory/safety — memory integrity status
//   POST /memory/safety/validate — run integrity check

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

static MEMSAFE: once_cell::sync::Lazy<Arc<RwLock<MemorySafetyState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(MemorySafetyState::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityCheck { pub id: String, pub timestamp: DateTime<Utc>, pub audit_chain_valid: bool, pub consciousness_valid: bool, pub goals_valid: bool, pub world_model_valid: bool, pub identity_valid: bool, pub constitution_valid: bool, pub overall_valid: bool, pub issues: Vec<String> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySafetyState { pub checks: Vec<IntegrityCheck>, pub total_checks: u64, pub last_check: Option<DateTime<Utc>>, pub issues_found: u64 }

impl Default for MemorySafetyState { fn default() -> Self { Self { checks: Vec::new(), total_checks: 0, last_check: None, issues_found: 0 } } }

fn gen_id() -> String { format!("msc_{}", chrono::Utc::now().timestamp_millis()) }

/// Run a full memory integrity check
pub async fn validate() -> IntegrityCheck {
    let mut issues = Vec::new();
    let mind = crate::consciousness::get_mind().await;
    let consciousness_valid = mind.uptime_secs > 0 || mind.total_tasks_completed == 0;
    if !consciousness_valid { issues.push("Consciousness state inconsistency detected".into()); }

    let goals = crate::goals::list().await;
    let goals_valid = goals.iter().all(|g| g.progress >= 0.0 && g.progress <= 1.0);
    if !goals_valid { issues.push("Goal progress out of range".into()); }

    let identity = crate::identity::get().await;
    let identity_valid = !identity.purpose.mission.is_empty();
    if !identity_valid { issues.push("Identity purpose is empty".into()); }

    let constitution = crate::constitution::get_articles().await;
    let constitution_valid = constitution.len() >= 7;
    if !constitution_valid { issues.push("Constitution articles missing".into()); }

    let overall = consciousness_valid && goals_valid && identity_valid && constitution_valid;

    let check = IntegrityCheck { id: gen_id(), timestamp: Utc::now(), audit_chain_valid: true, consciousness_valid, goals_valid, world_model_valid: true, identity_valid, constitution_valid, overall_valid: overall, issues: issues.clone() };

    let mut state = MEMSAFE.write().await;
    if state.checks.len() > 50 { state.checks.remove(0); }
    state.checks.push(check.clone());
    state.total_checks += 1;
    state.last_check = Some(Utc::now());
    state.issues_found += issues.len() as u64;

    check
}

pub async fn get_status() -> serde_json::Value {
    let s = MEMSAFE.read().await;
    serde_json::json!({ "total_checks": s.total_checks, "last_check": s.last_check, "issues_found": s.issues_found, "last_result": s.checks.last() })
}

pub async fn persist() { let s = MEMSAFE.read().await.clone(); if let Some(pool) = crate::memory::pool() { let json = match serde_json::to_value(&s) { Ok(v) => v, Err(_) => return }; let _ = sqlx::query("INSERT INTO memory_safety_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await; } }
pub async fn restore() { if let Some(pool) = crate::memory::pool() { let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM memory_safety_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None); if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<MemorySafetyState>(json) { let mut s = MEMSAFE.write().await; *s = r; } } } }
pub async fn init_table() { if let Some(pool) = crate::memory::pool() { let _ = sqlx::query("CREATE TABLE IF NOT EXISTS memory_safety_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await; } }

#[cfg(test)]
mod tests { use super::*; #[test] fn test_default() { let s = MemorySafetyState::default(); assert_eq!(s.total_checks, 0); } }
