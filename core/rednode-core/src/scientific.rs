// RedNode-OS — Scientific Method Engine
//
// Structured research: Observe → Hypothesis → Experiment → Measure → Analyze → Conclude → Store
// Especially valuable for cybersecurity research, performance tuning, and autonomous experimentation.
//
// API:
//   POST /science/experiment — create an experiment
//   GET  /science/experiments — list experiments
//   GET  /science/experiment/:id — experiment detail
//   POST /science/experiment/:id/result — record result

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

static SCIENCE: once_cell::sync::Lazy<Arc<RwLock<ScienceState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(ScienceState::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Experiment {
    pub id: String, pub title: String, pub domain: String,
    pub observation: String, pub hypothesis: String,
    pub methodology: Vec<String>, pub variables: Vec<Variable>,
    pub status: ExperimentStatus, pub results: Vec<ExperimentResult>,
    pub conclusion: Option<String>, pub knowledge_stored: bool,
    pub created_at: DateTime<Utc>, pub completed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variable { pub name: String, pub variable_type: VarType, pub value: String }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum VarType { Independent, Dependent, Controlled }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ExperimentStatus { Planned, Running, Measuring, Analyzing, Concluded, Failed }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExperimentResult { pub measurement: String, pub value: serde_json::Value, pub timestamp: DateTime<Utc>, pub notes: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScienceState { pub experiments: Vec<Experiment>, pub total: u64 }

impl Default for ScienceState { fn default() -> Self { Self { experiments: Vec::new(), total: 0 } } }

fn gen_id() -> String { format!("exp_{}", chrono::Utc::now().timestamp_millis()) }

pub async fn create_experiment(title: &str, domain: &str, observation: &str, hypothesis: &str, methodology: Vec<String>, variables: Vec<Variable>) -> Experiment {
    let exp = Experiment { id: gen_id(), title: title.into(), domain: domain.into(), observation: observation.into(), hypothesis: hypothesis.into(), methodology, variables, status: ExperimentStatus::Planned, results: Vec::new(), conclusion: None, knowledge_stored: false, created_at: Utc::now(), completed_at: None };
    let mut state = SCIENCE.write().await;
    state.experiments.push(exp.clone()); state.total += 1;
    tracing::info!(title = title, "Experiment created");
    exp
}

pub async fn record_result(experiment_id: &str, measurement: &str, value: serde_json::Value, notes: &str) -> bool {
    let mut state = SCIENCE.write().await;
    if let Some(exp) = state.experiments.iter_mut().find(|e| e.id == experiment_id) {
        exp.results.push(ExperimentResult { measurement: measurement.into(), value, timestamp: Utc::now(), notes: notes.into() });
        if exp.status == ExperimentStatus::Planned { exp.status = ExperimentStatus::Running; }
        return true;
    }
    false
}

pub async fn conclude(experiment_id: &str, conclusion: &str) -> bool {
    let mut state = SCIENCE.write().await;
    if let Some(exp) = state.experiments.iter_mut().find(|e| e.id == experiment_id) {
        exp.conclusion = Some(conclusion.into());
        exp.status = ExperimentStatus::Concluded;
        exp.completed_at = Some(Utc::now());
        return true;
    }
    false
}

pub async fn list_experiments() -> Vec<Experiment> { SCIENCE.read().await.experiments.clone() }
pub async fn get_experiment(id: &str) -> Option<Experiment> { SCIENCE.read().await.experiments.iter().find(|e| e.id == id).cloned() }

pub async fn persist() { let s = SCIENCE.read().await.clone(); if let Some(pool) = crate::memory::pool() { let json = match serde_json::to_value(&s) { Ok(v) => v, Err(_) => return }; let _ = sqlx::query("INSERT INTO science_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await; } }
pub async fn restore() { if let Some(pool) = crate::memory::pool() { let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM science_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None); if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<ScienceState>(json) { let mut s = SCIENCE.write().await; *s = r; } } } }
pub async fn init_table() { if let Some(pool) = crate::memory::pool() { let _ = sqlx::query("CREATE TABLE IF NOT EXISTS science_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await; } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_default() { let s = ScienceState::default(); assert_eq!(s.total, 0); }
    #[test] fn test_status_eq() { assert_eq!(ExperimentStatus::Planned, ExperimentStatus::Planned); }
}
