// RedNode-OS — Digital Twin
//
// A live simulation of the infrastructure based on the world model.
// Enables "what if" analysis without touching real systems:
//
//   - "What happens if I take TrueNAS offline?"
//   - "What if I upgrade pfSense to version X?"
//   - "Can I safely restart the NATS server right now?"
//   - "What's the blast radius of a power outage on VLAN 30?"
//
// The digital twin maintains a shadow copy of the world model and
// runs simulated scenarios against it. Results include:
//   - Services affected
//   - Estimated downtime
//   - Recovery sequence
//   - Risk assessment
//
// Integration:
//   - world_model.rs provides the real-time infrastructure state
//   - governance.rs validates proposed changes
//   - consciousness reads twin results for decision-making
//
// API:
//   POST /twin/simulate    — run a simulation scenario
//   GET  /twin/history     — past simulation results
//   POST /twin/validate    — validate a change before execution

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

static TWIN: once_cell::sync::Lazy<Arc<RwLock<TwinState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(TwinState::default())));

/// A simulation scenario
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scenario {
    pub id: String,
    pub name: String,
    pub description: String,
    pub actions: Vec<SimulatedAction>,
    pub created_at: DateTime<Utc>,
}

/// An action to simulate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SimulatedAction {
    /// Take a machine offline
    MachineOffline { machine_id: String },
    /// Take a service offline
    ServiceOffline { service_id: String },
    /// Remove a VLAN
    VlanRemove { vlan_id: String },
    /// Network partition (split two VLANs)
    NetworkPartition { vlan_a: String, vlan_b: String },
    /// Power outage on a VLAN
    PowerOutage { vlan_id: String },
    /// Upgrade a service
    ServiceUpgrade { service_id: String, new_version: String },
    /// Add a new machine
    MachineAdd { name: String, role: String },
    /// Custom scenario
    Custom { description: String },
}

/// The result of a simulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub id: String,
    pub scenario: Scenario,
    pub affected_services: Vec<AffectedService>,
    pub affected_machines: Vec<String>,
    pub estimated_downtime_minutes: u32,
    pub recovery_sequence: Vec<RecoveryStep>,
    pub risk_level: SimRiskLevel,
    pub summary: String,
    pub ran_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedService {
    pub id: String,
    pub name: String,
    pub impact: String,
    pub can_failover: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecoveryStep {
    pub order: u32,
    pub description: String,
    pub entity_id: String,
    pub estimated_minutes: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SimRiskLevel {
    Safe,
    Low,
    Medium,
    High,
    Critical,
}

/// The twin state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwinState {
    pub simulation_history: VecDeque<SimulationResult>,
    pub total_simulations: u64,
}

impl Default for TwinState {
    fn default() -> Self {
        Self {
            simulation_history: VecDeque::with_capacity(50),
            total_simulations: 0,
        }
    }
}

fn gen_id(prefix: &str) -> String {
    format!("{}_{}", prefix, chrono::Utc::now().timestamp_millis())
}

// ─── Public API ───

/// Run a simulation scenario against the current world model
pub async fn simulate(name: &str, description: &str, actions: Vec<SimulatedAction>) -> SimulationResult {
    let world = crate::world_model::get_world().await;

    let scenario = Scenario {
        id: gen_id("scenario"),
        name: name.into(),
        description: description.into(),
        actions: actions.clone(),
        created_at: Utc::now(),
    };

    let mut affected_services = Vec::new();
    let mut affected_machines = HashSet::new();
    let mut recovery_steps = Vec::new();
    let mut total_downtime = 0u32;

    for action in &actions {
        match action {
            SimulatedAction::MachineOffline { machine_id } => {
                affected_machines.insert(machine_id.clone());

                // Find all services running on this machine
                for service in &world.services {
                    if service.host_machine_id.as_deref() == Some(machine_id) {
                        affected_services.push(AffectedService {
                            id: service.id.clone(),
                            name: service.name.clone(),
                            impact: "Service will be unavailable".into(),
                            can_failover: false,
                        });
                    }
                }

                // Find all containers on this machine
                for container in &world.containers {
                    if container.host_machine_id == *machine_id {
                        affected_services.push(AffectedService {
                            id: container.id.clone(),
                            name: container.name.clone(),
                            impact: "Container will stop".into(),
                            can_failover: false,
                        });
                    }
                }

                // Walk dependency graph (reverse edges)
                let impact = crate::world_model::impact_analysis(machine_id).await;
                for dep in &impact.directly_affected {
                    if !affected_machines.contains(&dep.id) {
                        affected_services.push(AffectedService {
                            id: dep.id.clone(),
                            name: dep.name.clone(),
                            impact: format!("Affected via {} dependency", dep.relation),
                            can_failover: false,
                        });
                    }
                }
                for dep in &impact.transitively_affected {
                    if !affected_machines.contains(&dep.id) {
                        affected_services.push(AffectedService {
                            id: dep.id.clone(),
                            name: dep.name.clone(),
                            impact: format!("Transitively affected (depth {})", dep.depth),
                            can_failover: false,
                        });
                    }
                }

                total_downtime += 15; // Estimate 15 min per machine restart

                recovery_steps.push(RecoveryStep {
                    order: recovery_steps.len() as u32 + 1,
                    description: format!("Bring machine '{}' back online", machine_id),
                    entity_id: machine_id.clone(),
                    estimated_minutes: 5,
                });
            }

            SimulatedAction::ServiceOffline { service_id } => {
                if let Some(svc) = world.services.iter().find(|s| s.id == *service_id) {
                    affected_services.push(AffectedService {
                        id: svc.id.clone(),
                        name: svc.name.clone(),
                        impact: "Service explicitly taken offline".into(),
                        can_failover: false,
                    });
                    total_downtime += 5;
                }

                let impact = crate::world_model::impact_analysis(service_id).await;
                for dep in &impact.directly_affected {
                    affected_services.push(AffectedService {
                        id: dep.id.clone(),
                        name: dep.name.clone(),
                        impact: format!("Depends on offline service"),
                        can_failover: false,
                    });
                }
            }

            SimulatedAction::PowerOutage { vlan_id } => {
                // All machines on this VLAN go down
                if let Some(vlan) = world.vlans.iter().find(|v| v.id == *vlan_id) {
                    for mid in &vlan.machine_ids {
                        affected_machines.insert(mid.clone());
                    }
                    total_downtime += 30;

                    recovery_steps.push(RecoveryStep {
                        order: recovery_steps.len() as u32 + 1,
                        description: format!("Restore power to VLAN {}", vlan.name),
                        entity_id: vlan_id.clone(),
                        estimated_minutes: 10,
                    });
                }
            }

            SimulatedAction::VlanRemove { vlan_id } => {
                if let Some(vlan) = world.vlans.iter().find(|v| v.id == *vlan_id) {
                    for mid in &vlan.machine_ids {
                        affected_machines.insert(mid.clone());
                    }
                    total_downtime += 60;
                }
            }

            SimulatedAction::NetworkPartition { vlan_a, vlan_b } => {
                // Services that span both VLANs will be affected
                total_downtime += 10;
                affected_services.push(AffectedService {
                    id: format!("partition_{}_{}", vlan_a, vlan_b),
                    name: format!("Cross-VLAN traffic {}<->{}", vlan_a, vlan_b),
                    impact: "Network partition — cross-VLAN communication lost".into(),
                    can_failover: false,
                });
            }

            SimulatedAction::ServiceUpgrade { service_id, new_version } => {
                if let Some(svc) = world.services.iter().find(|s| s.id == *service_id) {
                    affected_services.push(AffectedService {
                        id: svc.id.clone(),
                        name: svc.name.clone(),
                        impact: format!("Upgrade to {} — brief restart required", new_version),
                        can_failover: false,
                    });
                    total_downtime += 3;
                }
            }

            SimulatedAction::MachineAdd { name, role } => {
                affected_services.push(AffectedService {
                    id: "new_machine".into(),
                    name: name.clone(),
                    impact: format!("New {} added — no disruption expected", role),
                    can_failover: false,
                });
            }

            SimulatedAction::Custom { description } => {
                affected_services.push(AffectedService {
                    id: "custom".into(),
                    name: "Custom scenario".into(),
                    impact: description.clone(),
                    can_failover: false,
                });
                total_downtime += 10;
            }
        }
    }

    // Add service restart steps to recovery
    for svc in &affected_services {
        if svc.impact.contains("unavailable") || svc.impact.contains("stop") {
            recovery_steps.push(RecoveryStep {
                order: recovery_steps.len() as u32 + 1,
                description: format!("Restart service '{}'", svc.name),
                entity_id: svc.id.clone(),
                estimated_minutes: 2,
            });
        }
    }

    // Risk assessment
    let risk_level = match affected_services.len() {
        0 => SimRiskLevel::Safe,
        1..=2 => SimRiskLevel::Low,
        3..=5 => SimRiskLevel::Medium,
        6..=10 => SimRiskLevel::High,
        _ => SimRiskLevel::Critical,
    };

    let summary = format!(
        "Simulation '{}': {} services affected, {} machines impacted, ~{}min downtime, {:?} risk",
        name,
        affected_services.len(),
        affected_machines.len(),
        total_downtime,
        risk_level,
    );

    let result = SimulationResult {
        id: gen_id("sim"),
        scenario,
        affected_services,
        affected_machines: affected_machines.into_iter().collect(),
        estimated_downtime_minutes: total_downtime,
        recovery_sequence: recovery_steps,
        risk_level,
        summary: summary.clone(),
        ran_at: Utc::now(),
    };

    // Store result
    let mut state = TWIN.write().await;
    if state.simulation_history.len() >= 50 {
        state.simulation_history.pop_front();
    }
    state.simulation_history.push_back(result.clone());
    state.total_simulations += 1;

    tracing::info!(summary = %summary, "Digital twin simulation complete");

    crate::events::emit(serde_json::json!({
        "type": "simulation_complete",
        "name": name,
        "risk": format!("{:?}", result.risk_level),
        "affected": result.affected_services.len(),
        "ts": Utc::now().to_rfc3339(),
    }));

    result
}

/// Validate a proposed change — shorthand for simulate + risk check
pub async fn validate_change(action: SimulatedAction) -> SimulationResult {
    let desc = format!("{:?}", action);
    simulate("change_validation", &desc, vec![action]).await
}

/// Get simulation history
pub async fn get_history(limit: usize) -> Vec<SimulationResult> {
    let state = TWIN.read().await;
    state.simulation_history.iter().rev().take(limit).cloned().collect()
}

/// Get stats
pub async fn get_stats() -> serde_json::Value {
    let state = TWIN.read().await;
    serde_json::json!({
        "total_simulations": state.total_simulations,
        "history_count": state.simulation_history.len(),
    })
}

// ─── Persistence ───

pub async fn persist() {
    let state = TWIN.read().await.clone();
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(&state) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Failed to serialize twin state: {}", e);
                return;
            }
        };

        let _ = sqlx::query(
            "INSERT INTO twin_store (id, state, updated_at) \
             VALUES (1, $1, NOW()) \
             ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()"
        )
        .bind(&json)
        .execute(pool)
        .await;
    }
}

pub async fn restore() {
    if let Some(pool) = crate::memory::pool() {
        let row: Option<(serde_json::Value,)> = sqlx::query_as(
            "SELECT state FROM twin_store WHERE id = 1"
        )
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        if let Some((json,)) = row {
            if let Ok(restored) = serde_json::from_value::<TwinState>(json) {
                let mut state = TWIN.write().await;
                *state = restored;
                tracing::info!(
                    simulations = state.total_simulations,
                    "Digital twin restored"
                );
            }
        }
    }
}

pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS twin_store (\
                id INTEGER PRIMARY KEY, \
                state JSONB NOT NULL, \
                updated_at TIMESTAMPTZ DEFAULT NOW()\
            )"
        )
        .execute(pool)
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let state = TwinState::default();
        assert!(state.simulation_history.is_empty());
        assert_eq!(state.total_simulations, 0);
    }

    #[test]
    fn test_risk_level_eq() {
        assert_eq!(SimRiskLevel::Safe, SimRiskLevel::Safe);
        assert_ne!(SimRiskLevel::Safe, SimRiskLevel::Critical);
    }

    #[test]
    fn test_simulated_action_serialization() {
        let action = SimulatedAction::MachineOffline { machine_id: "m_1".into() };
        let json = serde_json::to_string(&action).unwrap();
        assert!(json.contains("MachineOffline"));
        assert!(json.contains("m_1"));
    }

    #[test]
    fn test_recovery_step_serialization() {
        let step = RecoveryStep {
            order: 1,
            description: "Restart NAS".into(),
            entity_id: "m_nas".into(),
            estimated_minutes: 5,
        };
        let json = serde_json::to_string(&step).unwrap();
        assert!(json.contains("Restart NAS"));
    }
}
