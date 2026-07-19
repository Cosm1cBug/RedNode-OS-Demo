// RedNode-OS — Capability Registry
//
// Instead of asking "Can I do this?", RedNode KNOWS.
// Each capability records: version, confidence, dependencies,
// benchmark score, owner agent, last verified, required permissions.
//
// Prevents stale or unreliable abilities from being used.
//
// API:
//   GET  /capabilities           — all registered capabilities
//   GET  /capabilities/:id       — specific capability
//   POST /capabilities/verify/:id — trigger verification
//   GET  /capabilities/search    — search capabilities

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

static REGISTRY: once_cell::sync::Lazy<Arc<RwLock<CapabilityRegistry>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(CapabilityRegistry::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Capability {
    pub id: String, pub name: String, pub domain: String, pub version: String,
    pub confidence: f32, pub benchmark_score: Option<f32>,
    pub owner_agent: String, pub dependencies: Vec<String>,
    pub required_permissions: Vec<String>, pub verified: bool,
    pub last_verified: Option<DateTime<Utc>>, pub last_used: Option<DateTime<Utc>>,
    pub use_count: u64, pub success_count: u64, pub failure_count: u64,
    pub stale: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityRegistry { pub capabilities: Vec<Capability>, pub total_registered: u64 }

impl Default for CapabilityRegistry { fn default() -> Self { Self { capabilities: Vec::new(), total_registered: 0 } } }

fn gen_id() -> String { format!("cap_{}", chrono::Utc::now().timestamp_millis()) }

pub async fn register(name: &str, domain: &str, version: &str, agent: &str, deps: Vec<String>, perms: Vec<String>) -> Capability {
    let mut reg = REGISTRY.write().await;
    // Update if exists
    if let Some(existing) = reg.capabilities.iter_mut().find(|c| c.name == name && c.domain == domain) {
        existing.version = version.into();
        existing.owner_agent = agent.into();
        existing.dependencies = deps;
        existing.stale = false;
        return existing.clone();
    }
    let cap = Capability { id: gen_id(), name: name.into(), domain: domain.into(), version: version.into(), confidence: 0.5, benchmark_score: None, owner_agent: agent.into(), dependencies: deps, required_permissions: perms, verified: false, last_verified: None, last_used: None, use_count: 0, success_count: 0, failure_count: 0, stale: false };
    reg.capabilities.push(cap.clone()); reg.total_registered += 1;
    cap
}

pub async fn record_use(cap_id: &str, success: bool) {
    let mut reg = REGISTRY.write().await;
    if let Some(cap) = reg.capabilities.iter_mut().find(|c| c.id == cap_id) {
        cap.use_count += 1;
        if success { cap.success_count += 1; } else { cap.failure_count += 1; }
        cap.last_used = Some(Utc::now());
        cap.confidence = cap.success_count as f32 / cap.use_count as f32;
    }
}

pub async fn mark_verified(cap_id: &str, benchmark: Option<f32>) {
    let mut reg = REGISTRY.write().await;
    if let Some(cap) = reg.capabilities.iter_mut().find(|c| c.id == cap_id) {
        cap.verified = true; cap.last_verified = Some(Utc::now()); cap.benchmark_score = benchmark; cap.stale = false;
    }
}

/// Mark capabilities as stale if not verified in 30 days
pub async fn tick() {
    let mut reg = REGISTRY.write().await;
    let cutoff = Utc::now() - chrono::Duration::days(30);
    for cap in &mut reg.capabilities {
        if cap.last_verified.map_or(true, |v| v < cutoff) { cap.stale = true; }
    }
}

pub async fn search(query: &str) -> Vec<Capability> {
    let reg = REGISTRY.read().await;
    let q = query.to_lowercase();
    reg.capabilities.iter().filter(|c| c.name.to_lowercase().contains(&q) || c.domain.to_lowercase().contains(&q)).cloned().collect()
}

pub async fn list() -> Vec<Capability> { REGISTRY.read().await.capabilities.clone() }
pub async fn get(id: &str) -> Option<Capability> { REGISTRY.read().await.capabilities.iter().find(|c| c.id == id).cloned() }
pub async fn can_do(name: &str) -> bool { REGISTRY.read().await.capabilities.iter().any(|c| c.name == name && c.confidence > 0.3 && !c.stale) }

pub async fn persist() { let s = REGISTRY.read().await.clone(); if let Some(pool) = crate::memory::pool() { let json = match serde_json::to_value(&s) { Ok(v) => v, Err(_) => return }; let _ = sqlx::query("INSERT INTO capability_registry_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await; } }
pub async fn restore() { if let Some(pool) = crate::memory::pool() { let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM capability_registry_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None); if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<CapabilityRegistry>(json) { let mut s = REGISTRY.write().await; *s = r; } } } }
pub async fn init_table() { if let Some(pool) = crate::memory::pool() { let _ = sqlx::query("CREATE TABLE IF NOT EXISTS capability_registry_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await; } }

#[cfg(test)]
mod tests { use super::*; #[test] fn test_default() { let r = CapabilityRegistry::default(); assert!(r.capabilities.is_empty()); } }
