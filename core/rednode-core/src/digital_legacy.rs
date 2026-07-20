// RedNode-OS — Digital Legacy
//
// Allows future RedNode versions to inherit: Identity, Knowledge,
// Goals, Skills, Experiences. Ensures continuity across upgrades.
//
// API:
//   POST /legacy/export   — export legacy package
//   POST /legacy/import   — import from a legacy package
//   GET  /legacy/status   — legacy compatibility status

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

static LEGACY: once_cell::sync::Lazy<Arc<RwLock<LegacyState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(LegacyState::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyPackage {
    pub id: String, pub version: String, pub created_at: DateTime<Utc>,
    pub identity: serde_json::Value, pub constitution: serde_json::Value,
    pub goals: serde_json::Value, pub capabilities: serde_json::Value,
    pub episodic_highlights: serde_json::Value, pub procedures: serde_json::Value,
    pub personality: serde_json::Value, pub trust_scores: serde_json::Value,
    pub ethical_values: serde_json::Value, pub cognitive_metrics: serde_json::Value,
    pub checksum: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyState { pub exports: Vec<LegacyExport>, pub imports: Vec<LegacyImport>, pub total_exports: u64, pub total_imports: u64 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyExport { pub id: String, pub version: String, pub timestamp: DateTime<Utc>, pub size_bytes: u64 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyImport { pub id: String, pub from_version: String, pub timestamp: DateTime<Utc>, pub items_imported: u32, pub success: bool }

impl Default for LegacyState { fn default() -> Self { Self { exports: Vec::new(), imports: Vec::new(), total_exports: 0, total_imports: 0 } } }

fn gen_id() -> String { format!("leg_{}", chrono::Utc::now().timestamp_millis()) }

/// Export a legacy package containing all inheritable state
pub async fn export_legacy() -> LegacyPackage {
    let identity = serde_json::to_value(crate::identity::get().await).unwrap_or_default();
    let constitution = serde_json::to_value(crate::constitution::get_articles().await).unwrap_or_default();
    let goals = crate::goals::get_for_api().await;
    let capabilities = serde_json::to_value(crate::capability_registry::list().await).unwrap_or_default();
    let episodes = serde_json::to_value(crate::episodic_memory::get_recent_episodes(50).await).unwrap_or_default();
    let procedures = serde_json::to_value(crate::episodic_memory::get_procedures().await).unwrap_or_default();
    let personality = serde_json::to_value(crate::personality::get().await).unwrap_or_default();
    let trust = serde_json::to_value(crate::trust::get_all().await).unwrap_or_default();
    let ethics = serde_json::to_value(crate::ethics::get_values().await).unwrap_or_default();
    let metrics = serde_json::to_value(crate::cognitive_metrics::get().await).unwrap_or_default();

    let content = serde_json::json!({ "identity": identity, "goals": goals, "capabilities": capabilities });
    let checksum = format!("{:x}", sha2::Sha256::digest(content.to_string().as_bytes()));

    let pkg = LegacyPackage { id: gen_id(), version: "0.39.0".into(), created_at: Utc::now(), identity, constitution, goals, capabilities, episodic_highlights: episodes, procedures, personality, trust_scores: trust, ethical_values: ethics, cognitive_metrics: metrics, checksum };

    let mut state = LEGACY.write().await;
    let size = serde_json::to_string(&pkg).unwrap_or_default().len() as u64;
    state.exports.push(LegacyExport { id: pkg.id.clone(), version: pkg.version.clone(), timestamp: Utc::now(), size_bytes: size });
    state.total_exports += 1;

    tracing::info!(version = %pkg.version, "Legacy package exported");
    pkg
}

use sha2::Digest;

pub async fn get_status() -> serde_json::Value {
    let s = LEGACY.read().await;
    serde_json::json!({ "total_exports": s.total_exports, "total_imports": s.total_imports, "last_export": s.exports.last(), "last_import": s.imports.last() })
}

pub async fn persist() { let s = LEGACY.read().await.clone(); if let Some(pool) = crate::memory::pool() { let json = match serde_json::to_value(&s) { Ok(v) => v, Err(_) => return }; let _ = sqlx::query("INSERT INTO digital_legacy_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await; } }
pub async fn restore() { if let Some(pool) = crate::memory::pool() { let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM digital_legacy_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None); if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<LegacyState>(json) { let mut s = LEGACY.write().await; *s = r; } } } }
pub async fn init_table() { if let Some(pool) = crate::memory::pool() { let _ = sqlx::query("CREATE TABLE IF NOT EXISTS digital_legacy_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await; } }

#[cfg(test)]
mod tests { use super::*; #[test] fn test_default() { let s = LegacyState::default(); assert_eq!(s.total_exports, 0); } }
