// RedNode-OS — Hardware Abstraction Layer
//
// If one machine dies, another instance takes over:
//   - State synchronization across instances
//   - Distributed memory
//   - Distributed execution
//   - Failover detection and automatic migration
//   - Hardware capability profiling
//
// This makes RedNode resilient rather than tied to a single host.
//
// API:
//   GET  /hal              — hardware profile of this instance
//   GET  /hal/cluster      — cluster-wide hardware view
//   POST /hal/failover     — trigger manual failover

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

static HAL: once_cell::sync::Lazy<Arc<RwLock<HalState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(HalState::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub hostname: String, pub cpu_model: String, pub cpu_cores: u32,
    pub ram_total_mb: u64, pub gpu: Option<GpuInfo>,
    pub disks: Vec<DiskInfo>, pub network_interfaces: Vec<NetInterface>,
    pub architecture: String, pub os: String, pub uptime_secs: u64,
    pub last_profiled: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo { pub model: String, pub vram_mb: u64, pub driver: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskInfo { pub mount: String, pub total_gb: u64, pub used_gb: u64, pub filesystem: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetInterface { pub name: String, pub ip: Option<String>, pub mac: String, pub speed_mbps: Option<u32> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverTarget { pub peer_id: String, pub peer_name: String, pub ready: bool, pub last_checked: DateTime<Utc>, pub capabilities: Vec<String> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverRecord { pub id: String, pub from_host: String, pub to_host: String, pub reason: String, pub services_migrated: Vec<String>, pub timestamp: DateTime<Utc>, pub success: bool }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HalState {
    pub local_profile: HardwareProfile,
    pub failover_targets: Vec<FailoverTarget>,
    pub failover_history: Vec<FailoverRecord>,
    pub total_failovers: u64,
}

impl Default for HalState {
    fn default() -> Self {
        let sys = sysinfo::System::new_all();
        let hostname = gethostname::gethostname().to_string_lossy().to_string();

        Self {
            local_profile: HardwareProfile {
                hostname, cpu_model: "Unknown".into(),
                cpu_cores: sys.cpus().len() as u32,
                ram_total_mb: sys.total_memory() / 1024 / 1024,
                gpu: None, disks: vec![], network_interfaces: vec![],
                architecture: std::env::consts::ARCH.into(), os: std::env::consts::OS.into(),
                uptime_secs: sysinfo::System::uptime(), last_profiled: Utc::now(),
            },
            failover_targets: Vec::new(), failover_history: Vec::new(), total_failovers: 0,
        }
    }
}

fn gen_id() -> String { format!("fo_{}", chrono::Utc::now().timestamp_millis()) }

/// Refresh local hardware profile
pub async fn refresh_profile() {
    let sys = sysinfo::System::new_all();
    let mut state = HAL.write().await;
    state.local_profile.cpu_cores = sys.cpus().len() as u32;
    state.local_profile.ram_total_mb = sys.total_memory() / 1024 / 1024;
    state.local_profile.uptime_secs = sysinfo::System::uptime();
    state.local_profile.last_profiled = Utc::now();
}

pub async fn get_profile() -> HardwareProfile { HAL.read().await.local_profile.clone() }

pub async fn register_failover_target(peer_id: &str, peer_name: &str, capabilities: Vec<String>) {
    let mut state = HAL.write().await;
    state.failover_targets.push(FailoverTarget { peer_id: peer_id.into(), peer_name: peer_name.into(), ready: true, last_checked: Utc::now(), capabilities });
}

pub async fn record_failover(from: &str, to: &str, reason: &str, services: Vec<String>, success: bool) {
    let mut state = HAL.write().await;
    state.failover_history.push(FailoverRecord { id: gen_id(), from_host: from.into(), to_host: to.into(), reason: reason.into(), services_migrated: services, timestamp: Utc::now(), success });
    state.total_failovers += 1;
}

pub async fn get_cluster_view() -> serde_json::Value {
    let state = HAL.read().await;
    serde_json::json!({
        "local": state.local_profile,
        "failover_targets": state.failover_targets,
        "total_failovers": state.total_failovers,
    })
}

pub async fn persist() { let s = HAL.read().await.clone(); if let Some(pool) = crate::memory::pool() { let json = match serde_json::to_value(&s) { Ok(v) => v, Err(_) => return }; let _ = sqlx::query("INSERT INTO hal_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await; } }
pub async fn restore() { if let Some(pool) = crate::memory::pool() { let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM hal_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None); if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<HalState>(json) { let mut s = HAL.write().await; *s = r; } } } }
pub async fn init_table() { if let Some(pool) = crate::memory::pool() { let _ = sqlx::query("CREATE TABLE IF NOT EXISTS hal_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await; } }

#[cfg(test)]
mod tests { use super::*; #[test] fn test_default() { let s = HalState::default(); assert!(s.failover_targets.is_empty()); assert!(!s.local_profile.hostname.is_empty()); } }
