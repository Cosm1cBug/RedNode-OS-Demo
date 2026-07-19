// RedNode-OS — Cognitive Bus
//
// Event-driven backbone connecting all cognitive modules. Instead of
// modules calling each other directly (tight coupling), they emit
// typed cognitive events that any module can subscribe to.
//
// This enables:
//   - Loose coupling (modules don't need to know about each other)
//   - Event replay (audit trail of all cognitive activity)
//   - Plugin integration (plugins subscribe to events they care about)
//   - Debugging (trace any decision through its event chain)
//
// The cognitive bus sits alongside the existing events.rs WebSocket bus.
// events.rs is for external consumers (dashboard, clients).
// cognitive_bus.rs is for internal module-to-module communication.
//
// API:
//   GET /cognitive-bus/recent — recent cognitive events
//   GET /cognitive-bus/stats  — event statistics

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

static BUS: once_cell::sync::Lazy<Arc<RwLock<CognitiveBusState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(CognitiveBusState::default())));

static SENDER: once_cell::sync::Lazy<broadcast::Sender<CognitiveEvent>> =
    once_cell::sync::Lazy::new(|| {
        let (tx, _) = broadcast::channel(256);
        tx
    });

/// A typed cognitive event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveEvent {
    pub id: String,
    pub event_type: CognitiveEventType,
    pub source_module: String,
    pub data: serde_json::Value,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CognitiveEventType {
    ObservationCreated,
    GoalUpdated,
    GoalCompleted,
    MemoryStored,
    MemoryRecalled,
    ThreatDetected,
    ThreatMitigated,
    ReflectionCompleted,
    SimulationFinished,
    DecisionMade,
    TaskStarted,
    TaskCompleted,
    TaskFailed,
    AttentionShifted,
    TrustChanged,
    EpisodeRecorded,
    DreamCycleComplete,
    PolicyViolation,
    CapabilityVerified,
    PlanCreated,
    DebateCompleted,
    EvolutionProposed,
    ConfigChanged,
}

/// Statistics for event flow
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveBusState {
    pub recent_events: VecDeque<CognitiveEvent>,
    pub event_counts: HashMap<String, u64>,
    pub total_events: u64,
    pub events_per_minute: f32,
    pub last_event_at: Option<DateTime<Utc>>,
}

impl Default for CognitiveBusState {
    fn default() -> Self {
        Self {
            recent_events: VecDeque::with_capacity(200),
            event_counts: HashMap::new(),
            total_events: 0,
            events_per_minute: 0.0,
            last_event_at: None,
        }
    }
}

fn gen_id() -> String {
    format!("cev_{}", chrono::Utc::now().timestamp_millis())
}

// ─── Public API ───

/// Emit a cognitive event to the bus
pub async fn emit(event_type: CognitiveEventType, source_module: &str, data: serde_json::Value) {
    let event = CognitiveEvent {
        id: gen_id(),
        event_type: event_type.clone(),
        source_module: source_module.into(),
        data,
        timestamp: Utc::now(),
    };

    // Store in history
    let mut state = BUS.write().await;
    if state.recent_events.len() >= 200 {
        state.recent_events.pop_front();
    }
    state.recent_events.push_back(event.clone());
    state.total_events += 1;
    state.last_event_at = Some(Utc::now());

    let type_key = format!("{:?}", event_type);
    *state.event_counts.entry(type_key).or_insert(0) += 1;

    drop(state);

    // Broadcast to subscribers (non-blocking — if no subscribers, event is still recorded)
    let _ = SENDER.send(event);
}

/// Subscribe to cognitive events
pub fn subscribe() -> broadcast::Receiver<CognitiveEvent> {
    SENDER.subscribe()
}

/// Get recent events
pub async fn get_recent(limit: usize) -> Vec<CognitiveEvent> {
    BUS.read().await.recent_events.iter().rev().take(limit).cloned().collect()
}

/// Get recent events filtered by type
pub async fn get_by_type(event_type: &CognitiveEventType, limit: usize) -> Vec<CognitiveEvent> {
    BUS.read().await.recent_events.iter()
        .rev()
        .filter(|e| &e.event_type == event_type)
        .take(limit)
        .cloned()
        .collect()
}

/// Get event statistics
pub async fn get_stats() -> serde_json::Value {
    let state = BUS.read().await;
    serde_json::json!({
        "total_events": state.total_events,
        "event_counts": state.event_counts,
        "recent_count": state.recent_events.len(),
        "last_event_at": state.last_event_at,
        "events_per_minute": state.events_per_minute,
    })
}

/// Background tick — compute events per minute
pub async fn tick() {
    let mut state = BUS.write().await;
    let now = Utc::now();
    // Count events in last 60 seconds
    let one_min_ago = now - chrono::Duration::seconds(60);
    let recent_count = state.recent_events.iter()
        .filter(|e| e.timestamp > one_min_ago)
        .count();
    state.events_per_minute = recent_count as f32;
}

// ─── Convenience Emitters ───
// These are called by other modules instead of direct crate:: calls

pub async fn emit_task_completed(task_id: &str, agent: &str, tool: &str, duration_ms: u64, success: bool) {
    let etype = if success { CognitiveEventType::TaskCompleted } else { CognitiveEventType::TaskFailed };
    emit(etype, "coordinator", serde_json::json!({
        "task_id": task_id, "agent": agent, "tool": tool, "duration_ms": duration_ms,
    })).await;
}

pub async fn emit_goal_updated(goal_id: &str, title: &str, progress: f32) {
    emit(CognitiveEventType::GoalUpdated, "goals", serde_json::json!({
        "goal_id": goal_id, "title": title, "progress": progress,
    })).await;
}

pub async fn emit_threat_detected(level: &str, category: &str, description: &str) {
    emit(CognitiveEventType::ThreatDetected, "immune", serde_json::json!({
        "level": level, "category": category, "description": description,
    })).await;
}

pub async fn emit_decision_made(action: &str, confidence: f32, reason: &str) {
    emit(CognitiveEventType::DecisionMade, "consciousness", serde_json::json!({
        "action": action, "confidence": confidence, "reason": reason,
    })).await;
}

pub async fn emit_memory_stored(memory_type: &str, id: &str, summary: &str) {
    emit(CognitiveEventType::MemoryStored, "memory", serde_json::json!({
        "type": memory_type, "id": id, "summary": summary,
    })).await;
}

// ─── Persistence ───

pub async fn persist() {
    let state = BUS.read().await.clone();
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(&state) {
            Ok(v) => v,
            Err(e) => { tracing::warn!("Failed to serialize cognitive bus: {}", e); return; }
        };
        let _ = sqlx::query(
            "INSERT INTO cognitive_bus_store (id, state, updated_at) \
             VALUES (1, $1, NOW()) \
             ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()"
        ).bind(&json).execute(pool).await;
    }
}

pub async fn restore() {
    if let Some(pool) = crate::memory::pool() {
        let row: Option<(serde_json::Value,)> = sqlx::query_as(
            "SELECT state FROM cognitive_bus_store WHERE id = 1"
        ).fetch_optional(pool).await.unwrap_or(None);
        if let Some((json,)) = row {
            if let Ok(restored) = serde_json::from_value::<CognitiveBusState>(json) {
                let mut state = BUS.write().await;
                *state = restored;
                tracing::info!(
                    total = state.total_events,
                    "Cognitive bus state restored"
                );
            }
        }
    }
}

pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS cognitive_bus_store (\
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
        let state = CognitiveBusState::default();
        assert!(state.recent_events.is_empty());
        assert_eq!(state.total_events, 0);
    }

    #[test]
    fn test_event_type_eq() {
        assert_eq!(CognitiveEventType::TaskCompleted, CognitiveEventType::TaskCompleted);
        assert_ne!(CognitiveEventType::TaskCompleted, CognitiveEventType::TaskFailed);
    }

    #[test]
    fn test_event_serialization() {
        let event = CognitiveEvent {
            id: "test".into(),
            event_type: CognitiveEventType::GoalUpdated,
            source_module: "goals".into(),
            data: serde_json::json!({"goal_id": "g1"}),
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&event).expect("serialize cognitive event");
        assert!(json.contains("GoalUpdated"));
    }
}
