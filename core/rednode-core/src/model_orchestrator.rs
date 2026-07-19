// RedNode-OS — Model Orchestrator
//
// Selects the best model for each task type: planning, reasoning,
// coding, vision, speech, embeddings, verification. Based on speed,
// cost, privacy, and quality requirements.
//
// API:
//   GET  /models             — available models and their ratings
//   POST /models/select      — select best model for a task type
//   POST /models/benchmark   — benchmark a model

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

static ORCH: once_cell::sync::Lazy<Arc<RwLock<OrchestratorState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(OrchestratorState::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelProfile {
    pub id: String, pub name: String, pub provider: String,
    pub capabilities: Vec<String>, pub quality_score: f32, pub speed_score: f32,
    pub cost_score: f32, pub privacy_score: f32, pub local: bool,
    pub parameters: Option<String>, pub last_benchmarked: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSelection { pub task_type: String, pub selected_model: String, pub reason: String, pub alternatives: Vec<String>, pub timestamp: DateTime<Utc> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrchestratorState {
    pub models: Vec<ModelProfile>,
    pub task_assignments: HashMap<String, String>,
    pub selections: Vec<ModelSelection>,
    pub total_selections: u64,
}

impl Default for OrchestratorState {
    fn default() -> Self {
        Self {
            models: vec![
                ModelProfile { id: "qwen25_14b".into(), name: "Qwen2.5 14B".into(), provider: "ollama".into(), capabilities: vec!["planning".into(), "reasoning".into(), "coding".into()], quality_score: 0.8, speed_score: 0.6, cost_score: 1.0, privacy_score: 1.0, local: true, parameters: Some("14B".into()), last_benchmarked: None },
                ModelProfile { id: "qwen25_7b".into(), name: "Qwen2.5 7B".into(), provider: "ollama".into(), capabilities: vec!["planning".into(), "reasoning".into()], quality_score: 0.6, speed_score: 0.8, cost_score: 1.0, privacy_score: 1.0, local: true, parameters: Some("7B".into()), last_benchmarked: None },
                ModelProfile { id: "nomic_embed".into(), name: "Nomic Embed".into(), provider: "ollama".into(), capabilities: vec!["embeddings".into()], quality_score: 0.7, speed_score: 0.9, cost_score: 1.0, privacy_score: 1.0, local: true, parameters: None, last_benchmarked: None },
            ],
            task_assignments: HashMap::from([("planning".into(), "qwen25_14b".into()), ("reasoning".into(), "qwen25_14b".into()), ("embeddings".into(), "nomic_embed".into())]),
            selections: Vec::new(), total_selections: 0,
        }
    }
}

/// Select the best model for a task type
pub async fn select(task_type: &str, prefer_speed: bool) -> Option<ModelProfile> {
    let mut state = ORCH.write().await;
    let candidates: Vec<&ModelProfile> = state.models.iter().filter(|m| m.capabilities.contains(&task_type.to_string()) && m.local).collect();
    if candidates.is_empty() { return None; }

    let best = if prefer_speed {
        candidates.iter().max_by(|a, b| a.speed_score.partial_cmp(&b.speed_score).unwrap_or(std::cmp::Ordering::Equal))
    } else {
        candidates.iter().max_by(|a, b| a.quality_score.partial_cmp(&b.quality_score).unwrap_or(std::cmp::Ordering::Equal))
    };

    let model = best?.clone().clone();
    let sel = ModelSelection { task_type: task_type.into(), selected_model: model.name.clone(), reason: if prefer_speed { "Speed optimized".into() } else { "Quality optimized".into() }, alternatives: candidates.iter().map(|m| m.name.clone()).collect(), timestamp: Utc::now() };
    state.selections.push(sel);
    state.total_selections += 1;
    state.task_assignments.insert(task_type.into(), model.id.clone());

    Some(model)
}

pub async fn get_models() -> Vec<ModelProfile> { ORCH.read().await.models.clone() }
pub async fn get_assignment(task_type: &str) -> Option<String> { ORCH.read().await.task_assignments.get(task_type).cloned() }

pub async fn persist() { let s = ORCH.read().await.clone(); if let Some(pool) = crate::memory::pool() { let json = match serde_json::to_value(&s) { Ok(v) => v, Err(_) => return }; let _ = sqlx::query("INSERT INTO model_orchestrator_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await; } }
pub async fn restore() { if let Some(pool) = crate::memory::pool() { let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM model_orchestrator_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None); if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<OrchestratorState>(json) { let mut s = ORCH.write().await; *s = r; } } } }
pub async fn init_table() { if let Some(pool) = crate::memory::pool() { let _ = sqlx::query("CREATE TABLE IF NOT EXISTS model_orchestrator_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await; } }

#[cfg(test)]
mod tests { use super::*; #[test] fn test_default() { let s = OrchestratorState::default(); assert!(!s.models.is_empty()); } }
