// RedNode-OS — Mental Models
//
// Internal theories about how things work. Not just facts (memory) or
// skills (procedures), but BELIEFS — hypotheses with supporting and
// contradicting evidence, predictions that can be validated, and a
// revision history showing how understanding evolved.
//
// Flow: Observation → Pattern → Theory → Prediction → Validation → Revision
//
// A mental model continuously evolves as new evidence arrives.
// Models with validated predictions gain confidence.
// Models with failed predictions get revised or deprecated.
//
// API:
//   GET  /mental-models         — all models
//   GET  /mental-models/:id     — specific model
//   POST /mental-models         — create a model
//   POST /mental-models/:id/evidence — add evidence
//   POST /mental-models/:id/predict  — record a prediction
//   POST /mental-models/:id/validate — validate a prediction

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

static MODELS: once_cell::sync::Lazy<Arc<RwLock<MentalModelState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(MentalModelState::default())));

/// A mental model — an internal theory about how something works
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentalModel {
    pub id: String,
    pub title: String,
    pub domain: String,
    pub belief: String,
    pub supporting_evidence: Vec<Evidence>,
    pub contradicting_evidence: Vec<Evidence>,
    pub confidence: f32,
    pub predictions: Vec<Prediction>,
    pub revision_history: Vec<Revision>,
    pub status: ModelStatus,
    pub created_at: DateTime<Utc>,
    pub last_validated: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    pub description: String,
    pub source: String,
    pub strength: f32,
    pub supports: bool,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    pub id: String,
    pub description: String,
    pub predicted_outcome: String,
    pub predicted_confidence: f32,
    pub actual_outcome: Option<String>,
    pub was_correct: Option<bool>,
    pub predicted_at: DateTime<Utc>,
    pub validated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Revision {
    pub old_belief: String,
    pub new_belief: String,
    pub reason: String,
    pub evidence_id: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ModelStatus {
    Hypothetical,
    Supported,
    Established,
    Challenged,
    Revised,
    Deprecated,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MentalModelState {
    pub models: Vec<MentalModel>,
    pub total_created: u64,
    pub total_predictions: u64,
    pub total_validated: u64,
    pub prediction_accuracy: f32,
}

impl Default for MentalModelState {
    fn default() -> Self {
        Self {
            models: Vec::new(),
            total_created: 0,
            total_predictions: 0,
            total_validated: 0,
            prediction_accuracy: 0.0,
        }
    }
}

fn gen_id(prefix: &str) -> String {
    format!("{}_{}", prefix, chrono::Utc::now().timestamp_millis())
}

// ─── Public API ───

/// Create a new mental model (theory/hypothesis)
pub async fn create(title: &str, domain: &str, belief: &str) -> MentalModel {
    let model = MentalModel {
        id: gen_id("model"),
        title: title.into(),
        domain: domain.into(),
        belief: belief.into(),
        supporting_evidence: Vec::new(),
        contradicting_evidence: Vec::new(),
        confidence: 0.3,
        predictions: Vec::new(),
        revision_history: Vec::new(),
        status: ModelStatus::Hypothetical,
        created_at: Utc::now(),
        last_validated: None,
    };

    let mut state = MODELS.write().await;
    state.models.push(model.clone());
    state.total_created += 1;

    tracing::info!(title = title, domain = domain, "Mental model created");

    crate::cognitive_bus::emit(
        crate::cognitive_bus::CognitiveEventType::MemoryStored,
        "mental_models",
        serde_json::json!({"action": "model_created", "title": title, "domain": domain}),
    ).await;

    model
}

/// Add evidence to a model (supporting or contradicting)
pub async fn add_evidence(model_id: &str, description: &str, source: &str, strength: f32, supports: bool) -> bool {
    let mut state = MODELS.write().await;
    let model = match state.models.iter_mut().find(|m| m.id == model_id) {
        Some(m) => m,
        None => return false,
    };

    let evidence = Evidence {
        description: description.into(),
        source: source.into(),
        strength: strength.clamp(0.0, 1.0),
        supports,
        timestamp: Utc::now(),
    };

    if supports {
        model.supporting_evidence.push(evidence);
        model.confidence = (model.confidence + strength * 0.1).min(0.99);
        if model.confidence > 0.7 && model.status == ModelStatus::Hypothetical {
            model.status = ModelStatus::Supported;
        }
        if model.confidence > 0.9 {
            model.status = ModelStatus::Established;
        }
    } else {
        model.contradicting_evidence.push(evidence);
        model.confidence = (model.confidence - strength * 0.15).max(0.01);
        if model.status == ModelStatus::Established || model.status == ModelStatus::Supported {
            model.status = ModelStatus::Challenged;
        }
    }

    true
}

/// Record a prediction based on a model
pub async fn predict(model_id: &str, description: &str, predicted_outcome: &str) -> Option<String> {
    let mut state = MODELS.write().await;
    let model = state.models.iter_mut().find(|m| m.id == model_id)?;

    let pred_id = gen_id("pred");
    model.predictions.push(Prediction {
        id: pred_id.clone(),
        description: description.into(),
        predicted_outcome: predicted_outcome.into(),
        predicted_confidence: model.confidence,
        actual_outcome: None,
        was_correct: None,
        predicted_at: Utc::now(),
        validated_at: None,
    });

    state.total_predictions += 1;
    Some(pred_id)
}

/// Validate a prediction — compare predicted vs actual outcome
pub async fn validate_prediction(model_id: &str, prediction_id: &str, actual_outcome: &str, was_correct: bool) -> bool {
    let mut state = MODELS.write().await;
    let model = match state.models.iter_mut().find(|m| m.id == model_id) {
        Some(m) => m,
        None => return false,
    };

    let pred = match model.predictions.iter_mut().find(|p| p.id == prediction_id) {
        Some(p) => p,
        None => return false,
    };

    pred.actual_outcome = Some(actual_outcome.into());
    pred.was_correct = Some(was_correct);
    pred.validated_at = Some(Utc::now());

    // Adjust model confidence based on prediction accuracy
    if was_correct {
        model.confidence = (model.confidence + 0.05).min(0.99);
    } else {
        model.confidence = (model.confidence - 0.1).max(0.01);
        if model.status == ModelStatus::Established {
            model.status = ModelStatus::Challenged;
        }
    }

    model.last_validated = Some(Utc::now());
    state.total_validated += 1;

    // Update global prediction accuracy
    let all_validated: Vec<_> = state.models.iter()
        .flat_map(|m| m.predictions.iter())
        .filter(|p| p.was_correct.is_some())
        .collect();
    if !all_validated.is_empty() {
        let correct = all_validated.iter().filter(|p| p.was_correct == Some(true)).count();
        state.prediction_accuracy = correct as f32 / all_validated.len() as f32;
    }

    true
}

/// Revise a model's belief
pub async fn revise(model_id: &str, new_belief: &str, reason: &str) -> bool {
    let mut state = MODELS.write().await;
    let model = match state.models.iter_mut().find(|m| m.id == model_id) {
        Some(m) => m,
        None => return false,
    };

    model.revision_history.push(Revision {
        old_belief: model.belief.clone(),
        new_belief: new_belief.into(),
        reason: reason.into(),
        evidence_id: None,
        timestamp: Utc::now(),
    });

    model.belief = new_belief.into();
    model.status = ModelStatus::Revised;
    model.confidence = 0.4; // Reset confidence after revision

    true
}

/// Get all models
pub async fn list() -> Vec<MentalModel> {
    MODELS.read().await.models.clone()
}

/// Get a specific model
pub async fn get(id: &str) -> Option<MentalModel> {
    MODELS.read().await.models.iter().find(|m| m.id == id).cloned()
}

/// Get models in a domain
pub async fn get_by_domain(domain: &str) -> Vec<MentalModel> {
    MODELS.read().await.models.iter()
        .filter(|m| m.domain == domain && m.status != ModelStatus::Deprecated)
        .cloned()
        .collect()
}

/// Get stats
pub async fn get_stats() -> serde_json::Value {
    let state = MODELS.read().await;
    serde_json::json!({
        "total_models": state.models.len(),
        "total_predictions": state.total_predictions,
        "total_validated": state.total_validated,
        "prediction_accuracy": state.prediction_accuracy,
        "by_status": {
            "hypothetical": state.models.iter().filter(|m| m.status == ModelStatus::Hypothetical).count(),
            "supported": state.models.iter().filter(|m| m.status == ModelStatus::Supported).count(),
            "established": state.models.iter().filter(|m| m.status == ModelStatus::Established).count(),
            "challenged": state.models.iter().filter(|m| m.status == ModelStatus::Challenged).count(),
            "revised": state.models.iter().filter(|m| m.status == ModelStatus::Revised).count(),
        },
    })
}

// ─── Persistence ───

pub async fn persist() {
    let state = MODELS.read().await.clone();
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(&state) {
            Ok(v) => v,
            Err(e) => { tracing::warn!("Failed to serialize mental models: {}", e); return; }
        };
        let _ = sqlx::query(
            "INSERT INTO mental_models_store (id, state, updated_at) \
             VALUES (1, $1, NOW()) \
             ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()"
        ).bind(&json).execute(pool).await;
    }
}

pub async fn restore() {
    if let Some(pool) = crate::memory::pool() {
        let row: Option<(serde_json::Value,)> = sqlx::query_as(
            "SELECT state FROM mental_models_store WHERE id = 1"
        ).fetch_optional(pool).await.unwrap_or(None);
        if let Some((json,)) = row {
            if let Ok(restored) = serde_json::from_value::<MentalModelState>(json) {
                let mut state = MODELS.write().await;
                *state = restored;
                tracing::info!(
                    models = state.models.len(),
                    accuracy = state.prediction_accuracy,
                    "Mental models restored"
                );
            }
        }
    }
}

pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS mental_models_store (\
                id INTEGER PRIMARY KEY, \
                state JSONB NOT NULL, \
                updated_at TIMESTAMPTZ DEFAULT NOW()\
            )"
        ).execute(pool).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let state = MentalModelState::default();
        assert!(state.models.is_empty());
        assert_eq!(state.prediction_accuracy, 0.0);
    }

    #[test]
    fn test_model_status_eq() {
        assert_eq!(ModelStatus::Hypothetical, ModelStatus::Hypothetical);
        assert_ne!(ModelStatus::Hypothetical, ModelStatus::Established);
    }

    #[test]
    fn test_model_serialization() {
        let model = MentalModel {
            id: "test".into(), title: "Test Theory".into(), domain: "testing".into(),
            belief: "Tests should pass".into(), supporting_evidence: vec![], contradicting_evidence: vec![],
            confidence: 0.5, predictions: vec![], revision_history: vec![],
            status: ModelStatus::Hypothetical, created_at: Utc::now(), last_validated: None,
        };
        let json = serde_json::to_string(&model).expect("serialize mental model");
        assert!(json.contains("Test Theory"));
        assert!(json.contains("Hypothetical"));
    }
}
