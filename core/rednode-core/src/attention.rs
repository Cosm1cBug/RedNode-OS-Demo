// RedNode-OS — Attention Engine
//
// Decides what deserves focus RIGHT NOW. Without this, consciousness
// becomes overloaded — every event gets equal weight, leading to
// thrashing and poor decisions.
//
// Every incoming event, task, notification, and discovery is scored:
//   - Urgency (how time-sensitive?)
//   - Importance (how impactful?)
//   - Goal relevance (does it advance active goals?)
//   - Security impact (is there a threat component?)
//   - Confidence (how certain are we about the signal?)
//   - Novelty (is this new information?)
//
// The attention engine produces a ranked priority queue that
// consciousness uses to decide what to focus on next.
//
// API:
//   GET  /attention           — current attention state
//   GET  /attention/queue     — priority queue
//   POST /attention/signal    — submit a signal for scoring
//   POST /attention/focus     — manually set focus override

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BinaryHeap;
use std::sync::Arc;
use tokio::sync::RwLock;

static ATTENTION: once_cell::sync::Lazy<Arc<RwLock<AttentionState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(AttentionState::default())));

/// An incoming signal that needs attention scoring
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Signal {
    pub id: String,
    pub source: String,
    pub category: SignalCategory,
    pub description: String,
    pub raw_data: Option<serde_json::Value>,
    pub received_at: DateTime<Utc>,
    pub scores: AttentionScores,
    pub composite_priority: f32,
    pub processed: bool,
    pub action_taken: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SignalCategory {
    SecurityAlert,
    TaskRequest,
    SystemEvent,
    GoalProgress,
    Discovery,
    Notification,
    HealthAlert,
    UserInput,
    ScheduledEvent,
    AgentReport,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionScores {
    pub urgency: f32,
    pub importance: f32,
    pub goal_relevance: f32,
    pub security_impact: f32,
    pub confidence: f32,
    pub novelty: f32,
}

impl AttentionScores {
    /// Compute weighted composite score
    pub fn composite(&self) -> f32 {
        self.urgency * 0.25
        + self.importance * 0.20
        + self.goal_relevance * 0.15
        + self.security_impact * 0.20
        + self.confidence * 0.10
        + self.novelty * 0.10
    }
}

/// Current focus state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FocusTarget {
    pub signal_id: Option<String>,
    pub description: String,
    pub priority: f32,
    pub since: DateTime<Utc>,
    pub manual_override: bool,
}

/// The attention engine state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttentionState {
    pub current_focus: FocusTarget,
    pub queue: Vec<Signal>,
    pub recent_processed: Vec<Signal>,
    pub total_signals: u64,
    pub total_focus_changes: u64,
    pub attention_capacity: u32,
    pub filter_threshold: f32,
}

impl Default for AttentionState {
    fn default() -> Self {
        Self {
            current_focus: FocusTarget {
                signal_id: None,
                description: "Monitoring — awaiting signals".into(),
                priority: 0.0,
                since: Utc::now(),
                manual_override: false,
            },
            queue: Vec::new(),
            recent_processed: Vec::new(),
            total_signals: 0,
            total_focus_changes: 0,
            attention_capacity: 10,
            filter_threshold: 0.2,
        }
    }
}

fn gen_id() -> String { format!("sig_{}", chrono::Utc::now().timestamp_millis()) }

// ─── Scoring Logic ───

/// Score a signal based on its category and context
pub fn score_signal(category: &SignalCategory, description: &str, source: &str) -> AttentionScores {
    let desc_lower = description.to_lowercase();

    let urgency = match category {
        SignalCategory::SecurityAlert => 0.9,
        SignalCategory::HealthAlert => 0.8,
        SignalCategory::UserInput => 0.7,
        SignalCategory::TaskRequest => 0.6,
        SignalCategory::ScheduledEvent => 0.5,
        SignalCategory::AgentReport => 0.3,
        SignalCategory::Notification => 0.3,
        SignalCategory::GoalProgress => 0.2,
        SignalCategory::Discovery => 0.2,
        SignalCategory::SystemEvent => 0.4,
    };

    let importance = if desc_lower.contains("critical") || desc_lower.contains("failure") || desc_lower.contains("breach") {
        0.95
    } else if desc_lower.contains("warning") || desc_lower.contains("error") || desc_lower.contains("degraded") {
        0.7
    } else if desc_lower.contains("success") || desc_lower.contains("completed") {
        0.4
    } else {
        0.5
    };

    // Goal relevance — checked against active goals
    let goal_relevance = if desc_lower.contains("goal") || desc_lower.contains("objective") { 0.8 } else { 0.3 };

    let security_impact = match category {
        SignalCategory::SecurityAlert => 0.95,
        _ => if desc_lower.contains("security") || desc_lower.contains("threat") || desc_lower.contains("vuln") { 0.7 } else { 0.1 },
    };

    let confidence = match source {
        "user" => 1.0,
        "sensor" | "system" => 0.9,
        "agent" => 0.8,
        "llm" => 0.6,
        _ => 0.5,
    };

    let novelty = 0.5; // Would be computed from episodic memory similarity

    AttentionScores { urgency, importance, goal_relevance, security_impact, confidence, novelty }
}

// ─── Public API ───

/// Submit a new signal for attention scoring
pub async fn submit_signal(
    source: &str,
    category: SignalCategory,
    description: &str,
    raw_data: Option<serde_json::Value>,
) -> Signal {
    let scores = score_signal(&category, description, source);
    let composite = scores.composite();

    let signal = Signal {
        id: gen_id(),
        source: source.into(),
        category,
        description: description.into(),
        raw_data,
        received_at: Utc::now(),
        scores,
        composite_priority: composite,
        processed: false,
        action_taken: None,
    };

    let mut state = ATTENTION.write().await;
    state.total_signals += 1;

    // Filter out low-priority noise
    if composite < state.filter_threshold {
        return signal;
    }

    // Insert into priority queue (sorted by composite, descending)
    state.queue.push(signal.clone());
    state.queue.sort_by(|a, b| b.composite_priority.partial_cmp(&a.composite_priority).unwrap_or(std::cmp::Ordering::Equal));

    // Trim to capacity
    while state.queue.len() > state.attention_capacity as usize {
        if let Some(removed) = state.queue.pop() {
            state.recent_processed.push(removed);
        }
    }

    // Check if we should change focus
    if !state.current_focus.manual_override && composite > state.current_focus.priority {
        state.current_focus = FocusTarget {
            signal_id: Some(signal.id.clone()),
            description: signal.description.clone(),
            priority: composite,
            since: Utc::now(),
            manual_override: false,
        };
        state.total_focus_changes += 1;
    }

    // Keep recent_processed bounded
    if state.recent_processed.len() > 100 {
        state.recent_processed.drain(0..50);
    }

    signal
}

/// Get the next highest-priority signal from the queue
pub async fn next_signal() -> Option<Signal> {
    let mut state = ATTENTION.write().await;
    if state.queue.is_empty() {
        return None;
    }
    let mut signal = state.queue.remove(0);
    signal.processed = true;
    state.recent_processed.push(signal.clone());
    Some(signal)
}

/// Manually override focus
pub async fn set_focus(description: &str, priority: f32) {
    let mut state = ATTENTION.write().await;
    state.current_focus = FocusTarget {
        signal_id: None,
        description: description.into(),
        priority,
        since: Utc::now(),
        manual_override: true,
    };
    state.total_focus_changes += 1;
}

/// Release manual focus override
pub async fn release_focus() {
    let mut state = ATTENTION.write().await;
    state.current_focus.manual_override = false;
    // Re-evaluate from queue
    if let Some(top) = state.queue.first() {
        state.current_focus = FocusTarget {
            signal_id: Some(top.id.clone()),
            description: top.description.clone(),
            priority: top.composite_priority,
            since: Utc::now(),
            manual_override: false,
        };
    } else {
        state.current_focus.description = "Monitoring — awaiting signals".into();
        state.current_focus.priority = 0.0;
    }
}

/// Get current attention state
pub async fn get_state() -> AttentionState {
    ATTENTION.read().await.clone()
}

/// Get the priority queue
pub async fn get_queue() -> Vec<Signal> {
    ATTENTION.read().await.queue.clone()
}

/// Background tick — decay old signals, update focus
pub async fn tick() {
    let mut state = ATTENTION.write().await;
    let now = Utc::now();

    // Decay priority of aged signals (older than 1 hour lose 10% priority per tick)
    for signal in &mut state.queue {
        let age_hours = (now - signal.received_at).num_hours() as f32;
        if age_hours > 1.0 {
            signal.composite_priority *= 0.99;
        }
    }

    // Remove signals below threshold
    state.queue.retain(|s| s.composite_priority >= state.filter_threshold);

    // Update focus if current focus signal was removed
    if let Some(ref focus_id) = state.current_focus.signal_id {
        if !state.queue.iter().any(|s| s.id == *focus_id) && !state.current_focus.manual_override {
            if let Some(top) = state.queue.first() {
                state.current_focus = FocusTarget {
                    signal_id: Some(top.id.clone()),
                    description: top.description.clone(),
                    priority: top.composite_priority,
                    since: Utc::now(),
                    manual_override: false,
                };
            } else {
                state.current_focus.description = "Monitoring — awaiting signals".into();
                state.current_focus.priority = 0.0;
                state.current_focus.signal_id = None;
            }
        }
    }
}

pub async fn get_stats() -> serde_json::Value {
    let s = ATTENTION.read().await;
    serde_json::json!({
        "queue_size": s.queue.len(),
        "total_signals": s.total_signals,
        "total_focus_changes": s.total_focus_changes,
        "current_focus": s.current_focus.description,
        "current_priority": s.current_focus.priority,
    })
}

// ─── Persistence ───
pub async fn persist() {
    let s = ATTENTION.read().await.clone();
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(&s) { Ok(v) => v, Err(_) => return };
        let _ = sqlx::query("INSERT INTO attention_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await;
    }
}
pub async fn restore() {
    if let Some(pool) = crate::memory::pool() {
        let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM attention_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None);
        if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<AttentionState>(json) { let mut s = ATTENTION.write().await; *s = r; } }
    }
}
pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query("CREATE TABLE IF NOT EXISTS attention_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_score_composite() {
        let scores = AttentionScores { urgency: 0.9, importance: 0.8, goal_relevance: 0.5, security_impact: 0.9, confidence: 0.8, novelty: 0.6 };
        let c = scores.composite();
        assert!(c > 0.5 && c < 1.0);
    }
    #[test]
    fn test_score_security_alert() {
        let scores = score_signal(&SignalCategory::SecurityAlert, "Critical breach detected", "system");
        assert!(scores.urgency > 0.8);
        assert!(scores.security_impact > 0.9);
    }
    #[test]
    fn test_default_state() {
        let s = AttentionState::default();
        assert!(s.queue.is_empty());
        assert!(!s.current_focus.manual_override);
    }
}
