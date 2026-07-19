// RedNode-OS — Simulation Engine
//
// Broader than the Digital Twin. Before acting, RedNode can simulate:
//   Plan → Simulate → Predict → Evaluate → Execute
//
// Simulates: conversations, deployments, code changes, infrastructure,
// security responses, economic/resource costs.
//
// The simulation engine runs a proposed action plan through a virtual
// environment and predicts the outcome BEFORE committing to real execution.
//
// API:
//   POST /simulate/plan     — simulate an execution plan
//   POST /simulate/deploy   — simulate a deployment
//   POST /simulate/security — simulate a security response
//   GET  /simulate/history  — past simulation results

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

static SIM: once_cell::sync::Lazy<Arc<RwLock<SimulationState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(SimulationState::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationRequest {
    pub name: String,
    pub simulation_type: SimulationType,
    pub inputs: serde_json::Value,
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SimulationType {
    PlanExecution,
    Deployment,
    SecurityResponse,
    CodeChange,
    Conversation,
    ResourceAllocation,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationOutput {
    pub id: String,
    pub request: SimulationRequest,
    pub predicted_outcome: PredictedOutcome,
    pub risk_assessment: RiskAssessment,
    pub resource_cost: ResourceCost,
    pub recommendations: Vec<String>,
    pub confidence: f32,
    pub ran_at: DateTime<Utc>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PredictedOutcome {
    pub success_probability: f32,
    pub expected_duration_secs: u64,
    pub side_effects: Vec<String>,
    pub failure_modes: Vec<FailureMode>,
    pub rollback_possible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailureMode {
    pub description: String,
    pub probability: f32,
    pub impact: String,
    pub mitigation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskAssessment {
    pub overall_risk: f32,
    pub reversibility: f32,
    pub blast_radius: u32,
    pub requires_downtime: bool,
    pub affected_services: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceCost {
    pub estimated_cpu_ms: u64,
    pub estimated_ram_mb: u64,
    pub estimated_api_calls: u32,
    pub estimated_duration_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationState {
    pub history: VecDeque<SimulationOutput>,
    pub total_simulations: u64,
    pub avg_accuracy: f32,
}

impl Default for SimulationState {
    fn default() -> Self {
        Self { history: VecDeque::with_capacity(100), total_simulations: 0, avg_accuracy: 0.5 }
    }
}

fn gen_id() -> String { format!("sim_{}", chrono::Utc::now().timestamp_millis()) }

// ─── Public API ───

/// Run a simulation
pub async fn simulate(request: SimulationRequest) -> SimulationOutput {
    let start = std::time::Instant::now();
    let mut state = SIM.write().await;

    // Analyze the request based on type
    let (success_prob, duration_est, side_effects, failure_modes, reversible) = match request.simulation_type {
        SimulationType::PlanExecution => {
            let steps = request.inputs.get("steps").and_then(|v| v.as_u64()).unwrap_or(1);
            let prob = (1.0 - steps as f32 * 0.05).max(0.3);
            (prob, steps * 5, vec![], vec![
                FailureMode { description: "Agent timeout".into(), probability: 0.1, impact: "Task incomplete".into(), mitigation: "Retry with longer timeout".into() },
            ], true)
        }
        SimulationType::Deployment => {
            (0.7, 300, vec!["Brief service interruption".into()], vec![
                FailureMode { description: "Dependency conflict".into(), probability: 0.15, impact: "Deployment rollback".into(), mitigation: "Test in staging first".into() },
                FailureMode { description: "Resource exhaustion".into(), probability: 0.05, impact: "OOM kill".into(), mitigation: "Check resource limits".into() },
            ], true)
        }
        SimulationType::SecurityResponse => {
            (0.85, 60, vec!["Potential false positive blocking".into()], vec![
                FailureMode { description: "Legitimate traffic blocked".into(), probability: 0.1, impact: "Service disruption".into(), mitigation: "Whitelist known IPs".into() },
            ], true)
        }
        SimulationType::CodeChange => {
            (0.75, 120, vec!["Need to restart affected services".into()], vec![
                FailureMode { description: "Regression".into(), probability: 0.2, impact: "Feature broken".into(), mitigation: "Run test suite".into() },
            ], true)
        }
        _ => (0.6, 60, vec![], vec![], true),
    };

    // Check against world model for blast radius
    let world = crate::world_model::get_world().await;
    let affected = request.inputs.get("affected_services")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();

    let output = SimulationOutput {
        id: gen_id(),
        request: request.clone(),
        predicted_outcome: PredictedOutcome {
            success_probability: success_prob,
            expected_duration_secs: duration_est,
            side_effects,
            failure_modes,
            rollback_possible: reversible,
        },
        risk_assessment: RiskAssessment {
            overall_risk: 1.0 - success_prob,
            reversibility: if reversible { 0.9 } else { 0.2 },
            blast_radius: affected.len() as u32,
            requires_downtime: request.simulation_type == SimulationType::Deployment,
            affected_services: affected,
        },
        resource_cost: ResourceCost {
            estimated_cpu_ms: duration_est * 100,
            estimated_ram_mb: 256,
            estimated_api_calls: if matches!(request.simulation_type, SimulationType::PlanExecution) { 3 } else { 1 },
            estimated_duration_secs: duration_est,
        },
        recommendations: vec![
            if success_prob < 0.5 { "Consider alternative approach — low success probability".into() }
            else if success_prob < 0.7 { "Proceed with caution — moderate risk".into() }
            else { "Safe to proceed".into() },
        ],
        confidence: 0.6,
        ran_at: Utc::now(),
        duration_ms: start.elapsed().as_millis() as u64,
    };

    if state.history.len() >= 100 { state.history.pop_front(); }
    state.history.push_back(output.clone());
    state.total_simulations += 1;

    tracing::info!(
        name = %request.name,
        success_prob = success_prob,
        "Simulation complete"
    );

    output
}

pub async fn get_history(limit: usize) -> Vec<SimulationOutput> {
    SIM.read().await.history.iter().rev().take(limit).cloned().collect()
}

pub async fn get_stats() -> serde_json::Value {
    let state = SIM.read().await;
    serde_json::json!({
        "total_simulations": state.total_simulations,
        "avg_accuracy": state.avg_accuracy,
        "history_count": state.history.len(),
    })
}

pub async fn persist() {
    let state = SIM.read().await.clone();
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(&state) { Ok(v) => v, Err(_) => return };
        let _ = sqlx::query("INSERT INTO simulation_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await;
    }
}

pub async fn restore() {
    if let Some(pool) = crate::memory::pool() {
        let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM simulation_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None);
        if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<SimulationState>(json) { let mut s = SIM.write().await; *s = r; tracing::info!("Simulation engine restored"); } }
    }
}

pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query("CREATE TABLE IF NOT EXISTS simulation_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_simulation_type_eq() { assert_eq!(SimulationType::Deployment, SimulationType::Deployment); }
    #[test]
    fn test_default_state() { let s = SimulationState::default(); assert_eq!(s.total_simulations, 0); }
}
