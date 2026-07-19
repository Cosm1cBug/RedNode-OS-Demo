// RedNode-OS — Perception Layer
//
// Normalizes inputs from all sources into a unified observation format
// before feeding them to the attention engine. Raw inputs from vision,
// speech, logs, files, sensors, APIs, and network are converted into
// standardized Observation structs.
//
// Pipeline: Raw Input → Perception → Observation → Attention → Consciousness
//
// API:
//   POST /perception/observe — submit a raw observation
//   GET  /perception/recent  — recent observations
//   GET  /perception/stats   — observation statistics

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

static PERCEPTION: once_cell::sync::Lazy<Arc<RwLock<PerceptionState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(PerceptionState::default())));

/// A normalized observation from any input source
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Observation {
    pub id: String,
    pub modality: Modality,
    pub source: String,
    pub content: String,
    pub structured_data: Option<serde_json::Value>,
    pub confidence: f32,
    pub timestamp: DateTime<Utc>,
    pub processed: bool,
    pub forwarded_to_attention: bool,
}

/// Input modality — what kind of sensory input this is
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum Modality {
    Vision,
    Speech,
    LogEntry,
    FileChange,
    SensorReading,
    ApiResponse,
    NetworkEvent,
    UserInput,
    AgentReport,
    SystemMetric,
}

/// The perception state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerceptionState {
    pub recent: VecDeque<Observation>,
    pub modality_counts: HashMap<String, u64>,
    pub total_observations: u64,
    pub total_forwarded: u64,
}

impl Default for PerceptionState {
    fn default() -> Self {
        Self {
            recent: VecDeque::with_capacity(200),
            modality_counts: HashMap::new(),
            total_observations: 0,
            total_forwarded: 0,
        }
    }
}

fn gen_id() -> String { format!("obs_{}", chrono::Utc::now().timestamp_millis()) }

// ─── Public API ───

/// Submit a raw observation and normalize it
pub async fn observe(
    modality: Modality,
    source: &str,
    content: &str,
    structured_data: Option<serde_json::Value>,
    confidence: f32,
) -> Observation {
    let obs = Observation {
        id: gen_id(),
        modality: modality.clone(),
        source: source.into(),
        content: content.into(),
        structured_data,
        confidence: confidence.clamp(0.0, 1.0),
        timestamp: Utc::now(),
        processed: false,
        forwarded_to_attention: false,
    };

    let mut state = PERCEPTION.write().await;
    if state.recent.len() >= 200 { state.recent.pop_front(); }
    state.recent.push_back(obs.clone());
    state.total_observations += 1;
    let modality_key = format!("{:?}", modality);
    *state.modality_counts.entry(modality_key).or_insert(0) += 1;

    obs
}

/// Process an observation: normalize, extract signals, forward to attention
pub async fn process_and_forward(obs: &Observation) -> bool {
    // Map modality to attention signal category
    let category = match obs.modality {
        Modality::Vision => crate::attention::SignalCategory::SecurityAlert,
        Modality::Speech => crate::attention::SignalCategory::UserInput,
        Modality::LogEntry => crate::attention::SignalCategory::SystemEvent,
        Modality::FileChange => crate::attention::SignalCategory::SystemEvent,
        Modality::SensorReading => crate::attention::SignalCategory::HealthAlert,
        Modality::ApiResponse => crate::attention::SignalCategory::AgentReport,
        Modality::NetworkEvent => crate::attention::SignalCategory::SecurityAlert,
        Modality::UserInput => crate::attention::SignalCategory::UserInput,
        Modality::AgentReport => crate::attention::SignalCategory::AgentReport,
        Modality::SystemMetric => crate::attention::SignalCategory::SystemEvent,
    };

    // Forward to attention engine
    crate::attention::submit_signal(&obs.source, category, &obs.content, obs.structured_data.clone()).await;

    // Emit cognitive bus event
    crate::cognitive_bus::emit(
        crate::cognitive_bus::CognitiveEventType::ObservationCreated,
        "perception",
        serde_json::json!({
            "id": obs.id,
            "modality": format!("{:?}", obs.modality),
            "source": obs.source,
        }),
    ).await;

    // Mark as forwarded
    let mut state = PERCEPTION.write().await;
    if let Some(stored) = state.recent.iter_mut().find(|o| o.id == obs.id) {
        stored.processed = true;
        stored.forwarded_to_attention = true;
    }
    state.total_forwarded += 1;

    true
}

/// Get recent observations
pub async fn get_recent(limit: usize) -> Vec<Observation> {
    PERCEPTION.read().await.recent.iter().rev().take(limit).cloned().collect()
}

/// Get statistics
pub async fn get_stats() -> serde_json::Value {
    let state = PERCEPTION.read().await;
    serde_json::json!({
        "total_observations": state.total_observations,
        "total_forwarded": state.total_forwarded,
        "modality_counts": state.modality_counts,
        "recent_count": state.recent.len(),
    })
}

// ─── Persistence ───

pub async fn persist() {
    let state = PERCEPTION.read().await.clone();
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(&state) {
            Ok(v) => v,
            Err(e) => { tracing::warn!("Failed to serialize perception: {}", e); return; }
        };
        let _ = sqlx::query(
            "INSERT INTO perception_store (id, state, updated_at) \
             VALUES (1, $1, NOW()) \
             ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()"
        ).bind(&json).execute(pool).await;
    }
}

pub async fn restore() {
    if let Some(pool) = crate::memory::pool() {
        let row: Option<(serde_json::Value,)> = sqlx::query_as(
            "SELECT state FROM perception_store WHERE id = 1"
        ).fetch_optional(pool).await.unwrap_or(None);
        if let Some((json,)) = row {
            if let Ok(restored) = serde_json::from_value::<PerceptionState>(json) {
                let mut state = PERCEPTION.write().await;
                *state = restored;
                tracing::info!(total = state.total_observations, "Perception layer restored");
            }
        }
    }
}

pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS perception_store (\
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
        let state = PerceptionState::default();
        assert!(state.recent.is_empty());
        assert_eq!(state.total_observations, 0);
    }

    #[test]
    fn test_modality_eq() {
        assert_eq!(Modality::Vision, Modality::Vision);
        assert_ne!(Modality::Vision, Modality::Speech);
    }

    #[test]
    fn test_observation_serialization() {
        let obs = Observation {
            id: "test".into(), modality: Modality::LogEntry, source: "syslog".into(),
            content: "disk full".into(), structured_data: None, confidence: 0.9,
            timestamp: Utc::now(), processed: false, forwarded_to_attention: false,
        };
        let json = serde_json::to_string(&obs).expect("serialize observation");
        assert!(json.contains("LogEntry"));
        assert!(json.contains("disk full"));
    }
}
