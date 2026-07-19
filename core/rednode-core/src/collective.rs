// RedNode-OS — Collective Intelligence
//
// Multiple RedNode instances collaborating as one distributed intelligence:
//   Laptop <-> Server <-> Raspberry Pi <-> NAS
//
// Each instance is autonomous with partial knowledge.
// Together they behave as one intelligence.
//
// Features:
//   - Peer discovery (mDNS/manual registration)
//   - Knowledge sharing (selective sync of memory/learnings)
//   - Task delegation (route tasks to the best-equipped peer)
//   - Consensus decisions (quorum-based for critical actions)
//   - Conflict resolution (when peers disagree)
//   - State synchronization (world model, goals, discoveries)
//
// API:
//   GET  /collective         — collective status
//   GET  /collective/peers   — known peer instances
//   POST /collective/peers   — register a peer
//   POST /collective/sync    — trigger sync with peers
//   POST /collective/delegate — delegate a task to best peer

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

static COLLECTIVE: once_cell::sync::Lazy<Arc<RwLock<CollectiveState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(CollectiveState::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInstance {
    pub id: String, pub name: String, pub url: String, pub role: PeerRole,
    pub capabilities: Vec<String>, pub status: PeerStatus,
    pub last_seen: DateTime<Utc>, pub last_sync: Option<DateTime<Utc>>,
    pub trust_score: f32, pub version: String,
    /// What this peer specializes in (subset of capabilities it's best at)
    pub specializations: Vec<String>,
}

/// Policy controlling what knowledge to share vs keep local
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FederationPolicy {
    /// Share episodic memories with peers?
    pub share_episodes: bool,
    /// Share distilled knowledge documents?
    pub share_knowledge: bool,
    /// Share goal progress?
    pub share_goals: bool,
    /// Share threat intelligence?
    pub share_threats: bool,
    /// Share capability registry?
    pub share_capabilities: bool,
    /// Categories of knowledge to NEVER share
    pub deny_categories: Vec<String>,
}

impl Default for FederationPolicy {
    fn default() -> Self {
        Self {
            share_episodes: false,
            share_knowledge: true,
            share_goals: false,
            share_threats: true,
            share_capabilities: true,
            deny_categories: vec!["personal".into(), "credentials".into()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PeerRole { Primary, Secondary, Edge, Storage, Compute }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PeerStatus { Online, Offline, Syncing, Degraded, Unknown }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncRecord { pub peer_id: String, pub timestamp: DateTime<Utc>, pub items_sent: u32, pub items_received: u32, pub duration_ms: u64, pub success: bool }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationRecord { pub id: String, pub task: String, pub from_peer: String, pub to_peer: String, pub reason: String, pub status: String, pub timestamp: DateTime<Utc> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectiveState {
    pub this_instance_id: String, pub peers: Vec<PeerInstance>,
    pub sync_history: Vec<SyncRecord>, pub delegations: Vec<DelegationRecord>,
    pub total_syncs: u64, pub total_delegations: u64,
    /// Policy controlling knowledge sharing with peers
    pub federation_policy: FederationPolicy,
}

impl Default for CollectiveState {
    fn default() -> Self {
        Self {
            this_instance_id: format!("rednode_{}", gethostname::gethostname().to_string_lossy()),
            peers: Vec::new(), sync_history: Vec::new(), delegations: Vec::new(),
            total_syncs: 0, total_delegations: 0,
            federation_policy: FederationPolicy::default(),
        }
    }
}

fn gen_id() -> String { format!("peer_{}", chrono::Utc::now().timestamp_millis()) }

pub async fn register_peer(name: &str, url: &str, role: PeerRole, capabilities: Vec<String>, version: &str) -> PeerInstance {
    let mut state = COLLECTIVE.write().await;
    let peer = PeerInstance { id: gen_id(), name: name.into(), url: url.into(), role, capabilities, status: PeerStatus::Unknown, last_seen: Utc::now(), last_sync: None, trust_score: 0.5, version: version.into(), specializations: Vec::new() };
    state.peers.push(peer.clone());
    tracing::info!(name = name, url = url, "Peer registered");
    peer
}

pub async fn remove_peer(id: &str) -> bool {
    let mut state = COLLECTIVE.write().await;
    let before = state.peers.len();
    state.peers.retain(|p| p.id != id);
    state.peers.len() < before
}

pub async fn update_peer_status(id: &str, status: PeerStatus) {
    let mut state = COLLECTIVE.write().await;
    if let Some(peer) = state.peers.iter_mut().find(|p| p.id == id) {
        peer.status = status; peer.last_seen = Utc::now();
    }
}

/// Find the best peer for a given capability
pub async fn find_best_peer(capability: &str) -> Option<PeerInstance> {
    let state = COLLECTIVE.read().await;
    state.peers.iter()
        .filter(|p| p.status == PeerStatus::Online && p.capabilities.iter().any(|c| c == capability))
        .max_by(|a, b| a.trust_score.partial_cmp(&b.trust_score).unwrap_or(std::cmp::Ordering::Equal))
        .cloned()
}

pub async fn record_sync(peer_id: &str, items_sent: u32, items_received: u32, duration_ms: u64, success: bool) {
    let mut state = COLLECTIVE.write().await;
    state.sync_history.push(SyncRecord { peer_id: peer_id.into(), timestamp: Utc::now(), items_sent, items_received, duration_ms, success });
    if state.sync_history.len() > 100 { state.sync_history.remove(0); }
    state.total_syncs += 1;
    if let Some(peer) = state.peers.iter_mut().find(|p| p.id == peer_id) {
        peer.last_sync = Some(Utc::now());
        if success { peer.trust_score = (peer.trust_score + 0.02).min(1.0); }
        else { peer.trust_score = (peer.trust_score - 0.05).max(0.0); }
    }
}

pub async fn record_delegation(task: &str, to_peer: &str, reason: &str) {
    let mut state = COLLECTIVE.write().await;
    state.delegations.push(DelegationRecord { id: gen_id(), task: task.into(), from_peer: state.this_instance_id.clone(), to_peer: to_peer.into(), reason: reason.into(), status: "delegated".into(), timestamp: Utc::now() });
    if state.delegations.len() > 100 { state.delegations.remove(0); }
    state.total_delegations += 1;
}

pub async fn get_peers() -> Vec<PeerInstance> { COLLECTIVE.read().await.peers.clone() }
pub async fn get_status() -> serde_json::Value {
    let s = COLLECTIVE.read().await;
    serde_json::json!({ "instance_id": s.this_instance_id, "peer_count": s.peers.len(), "online_peers": s.peers.iter().filter(|p| p.status == PeerStatus::Online).count(), "total_syncs": s.total_syncs, "total_delegations": s.total_delegations })
}

pub async fn persist() { let s = COLLECTIVE.read().await.clone(); if let Some(pool) = crate::memory::pool() { let json = match serde_json::to_value(&s) { Ok(v) => v, Err(_) => return }; let _ = sqlx::query("INSERT INTO collective_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await; } }
pub async fn restore() { if let Some(pool) = crate::memory::pool() { let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM collective_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None); if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<CollectiveState>(json) { let mut s = COLLECTIVE.write().await; *s = r; } } } }
pub async fn init_table() { if let Some(pool) = crate::memory::pool() { let _ = sqlx::query("CREATE TABLE IF NOT EXISTS collective_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await; } }

#[cfg(test)]
mod tests { use super::*; #[test] fn test_default() { let s = CollectiveState::default(); assert!(s.peers.is_empty()); } #[test] fn test_role_eq() { assert_eq!(PeerRole::Primary, PeerRole::Primary); } }
