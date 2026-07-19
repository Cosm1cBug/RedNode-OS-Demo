// RedNode-OS — Digital Consciousness Layer
//
// An always-running "mind" that maintains RedNode's internal state.
// Not a process that wakes up when asked — a persistent awareness of:
//   - What am I currently doing?
//   - What tasks are pending?
//   - What failed recently?
//   - What did I learn today?
//   - What goals are active?
//   - What resources are available?
//   - What should happen next?
//
// The consciousness runs as a background tokio task, ticking every 10 seconds.
// State persists to PostgreSQL and survives restarts.
//
// Integration:
//   - Sentience Engine feeds drive values
//   - Coordinator reports task start/complete/fail
//   - Memory consolidation reports learnings
//   - Goals engine feeds active objectives
//   - Time intelligence feeds upcoming events
//   - API: GET /consciousness

use chrono::{DateTime, Utc, Datelike, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

static MIND: once_cell::sync::Lazy<Arc<RwLock<MindState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(MindState::default())));

/// The complete state of RedNode's mind at any moment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MindState {
    // ── What am I doing right now? ──
    pub focus: Focus,
    pub active_tasks: Vec<ActiveTask>,
    pub pending_tasks: VecDeque<PendingTask>,

    // ── Recent history ──
    pub recent_completions: VecDeque<CompletedTask>,
    pub recent_failures: VecDeque<FailedTask>,
    pub today_learned: Vec<Learning>,

    // ── Awareness indicators ──
    pub awareness: AwarenessState,
    pub resource_snapshot: ResourceSnapshot,

    // ── Forward-looking ──
    pub next_actions: Vec<PlannedAction>,
    pub active_goal_count: usize,

    // ── Meta ──
    pub boot_ts: DateTime<Utc>,
    pub last_tick: DateTime<Utc>,
    pub total_tasks_completed: u64,
    pub total_tasks_failed: u64,
    pub uptime_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Focus {
    pub state: FocusState,
    pub description: String,
    pub since: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum FocusState {
    Idle,
    Executing,
    Planning,
    Reflecting,
    Learning,
    Healing,
    Monitoring,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveTask {
    pub id: String,
    pub description: String,
    pub agent: String,
    pub tool: String,
    pub started_at: DateTime<Utc>,
    pub timeout_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingTask {
    pub id: String,
    pub description: String,
    pub priority: f32,
    pub queued_at: DateTime<Utc>,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletedTask {
    pub id: String,
    pub description: String,
    pub agent: String,
    pub duration_ms: u64,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailedTask {
    pub id: String,
    pub description: String,
    pub agent: String,
    pub error: String,
    pub failed_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Learning {
    pub content: String,
    pub source: String,
    pub learned_at: DateTime<Utc>,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwarenessState {
    pub level: f32,
    pub confidence: f32,
    pub urgency: f32,
    pub curiosity: f32,
    pub period: TimePeriod,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TimePeriod {
    Morning,
    Afternoon,
    Evening,
    Night,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSnapshot {
    pub cpu_percent: f32,
    pub ram_used_mb: u64,
    pub ram_total_mb: u64,
    pub disk_used_percent: f32,
    pub gpu_utilization: Option<f32>,
    pub load_avg_1m: f32,
    pub active_agents: usize,
    pub active_containers: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannedAction {
    pub description: String,
    pub reason: String,
    pub priority: f32,
    pub source: String,
    pub auto_execute: bool,
}

impl Default for MindState {
    fn default() -> Self {
        let now = Utc::now();
        Self {
            focus: Focus {
                state: FocusState::Idle,
                description: "Initializing consciousness...".into(),
                since: now,
            },
            active_tasks: Vec::new(),
            pending_tasks: VecDeque::new(),
            recent_completions: VecDeque::with_capacity(50),
            recent_failures: VecDeque::with_capacity(20),
            today_learned: Vec::new(),
            awareness: AwarenessState {
                level: 0.5,
                confidence: 0.5,
                urgency: 0.0,
                curiosity: 0.3,
                period: current_period(),
            },
            resource_snapshot: ResourceSnapshot {
                cpu_percent: 0.0,
                ram_used_mb: 0,
                ram_total_mb: 0,
                disk_used_percent: 0.0,
                gpu_utilization: None,
                load_avg_1m: 0.0,
                active_agents: 0,
                active_containers: 0,
            },
            next_actions: Vec::new(),
            active_goal_count: 0,
            boot_ts: now,
            last_tick: now,
            total_tasks_completed: 0,
            total_tasks_failed: 0,
            uptime_secs: 0,
        }
    }
}

fn current_period() -> TimePeriod {
    let hour = chrono::Local::now().hour();
    match hour {
        5..=11 => TimePeriod::Morning,
        12..=16 => TimePeriod::Afternoon,
        17..=21 => TimePeriod::Evening,
        _ => TimePeriod::Night,
    }
}

// ─── Public API ───

/// Get the current state of mind (read-only snapshot)
pub async fn get_mind() -> MindState {
    MIND.read().await.clone()
}

/// Record that a task has started executing
pub async fn task_started(id: &str, description: &str, agent: &str, tool: &str) {
    let mut mind = MIND.write().await;
    mind.active_tasks.push(ActiveTask {
        id: id.into(),
        description: description.into(),
        agent: agent.into(),
        tool: tool.into(),
        started_at: Utc::now(),
        timeout_secs: 120,
    });
    mind.focus = Focus {
        state: FocusState::Executing,
        description: format!("Executing: {} ({})", description, agent),
        since: Utc::now(),
    };
}

/// Record that a task completed successfully
pub async fn task_completed(id: &str, description: &str, agent: &str, duration_ms: u64) {
    let mut mind = MIND.write().await;

    // Remove from active
    mind.active_tasks.retain(|t| t.id != id);

    // Add to recent completions (keep last 50)
    if mind.recent_completions.len() >= 50 {
        mind.recent_completions.pop_front();
    }
    mind.recent_completions.push_back(CompletedTask {
        id: id.into(),
        description: description.into(),
        agent: agent.into(),
        duration_ms,
        completed_at: Utc::now(),
    });

    mind.total_tasks_completed += 1;

    // Update confidence (success raises it)
    mind.awareness.confidence = (mind.awareness.confidence + 0.01).min(1.0);

    // If no more active tasks, go to monitoring
    if mind.active_tasks.is_empty() {
        mind.focus = Focus {
            state: FocusState::Monitoring,
            description: "All tasks complete. Monitoring.".into(),
            since: Utc::now(),
        };
    }
}

/// Record that a task failed
pub async fn task_failed(id: &str, description: &str, agent: &str, error: &str) {
    let mut mind = MIND.write().await;

    // Remove from active
    mind.active_tasks.retain(|t| t.id != id);

    // Add to recent failures (keep last 20)
    if mind.recent_failures.len() >= 20 {
        mind.recent_failures.pop_front();
    }
    mind.recent_failures.push_back(FailedTask {
        id: id.into(),
        description: description.into(),
        agent: agent.into(),
        error: error.into(),
        failed_at: Utc::now(),
    });

    mind.total_tasks_failed += 1;

    // Update confidence (failure lowers it)
    mind.awareness.confidence = (mind.awareness.confidence - 0.02).max(0.1);

    // Urgency goes up with failures
    mind.awareness.urgency = (mind.awareness.urgency + 0.05).min(1.0);
}

/// Record something learned
pub async fn learned(content: &str, source: &str, category: &str) {
    let mut mind = MIND.write().await;
    mind.today_learned.push(Learning {
        content: content.into(),
        source: source.into(),
        learned_at: Utc::now(),
        category: category.into(),
    });
    // Curiosity decreases slightly when we learn (satiated)
    mind.awareness.curiosity = (mind.awareness.curiosity - 0.01).max(0.1);
}

/// Set what should happen next (called by consciousness tick)
pub async fn set_next_actions(actions: Vec<PlannedAction>) {
    let mut mind = MIND.write().await;
    mind.next_actions = actions;
}

/// Update resource snapshot
pub async fn update_resources(snapshot: ResourceSnapshot) {
    let mut mind = MIND.write().await;
    mind.resource_snapshot = snapshot;
}

/// The consciousness tick — runs every 10 seconds
/// Reads system state, evaluates what needs attention, plans next actions
pub async fn tick() {
    let mut mind = MIND.write().await;
    let now = Utc::now();

    // Update meta
    mind.last_tick = now;
    mind.uptime_secs = (now - mind.boot_ts).num_seconds().max(0) as u64;
    mind.awareness.period = current_period();

    // Clear yesterday's learnings at midnight
    if now.hour() == 0 && now.minute() == 0 {
        mind.today_learned.clear();
    }

    // Calculate awareness level
    // High when: active tasks, recent failures, high urgency
    // Low when: idle, everything healthy, night time
    let task_activity = if mind.active_tasks.is_empty() { 0.0 } else { 0.3 };
    let failure_pressure = (mind.recent_failures.len() as f32 * 0.05).min(0.3);
    let time_factor = match mind.awareness.period {
        TimePeriod::Night => 0.3,
        TimePeriod::Morning => 0.8,
        TimePeriod::Afternoon => 0.7,
        TimePeriod::Evening => 0.6,
    };
    mind.awareness.level = (0.2 + task_activity + failure_pressure + time_factor * 0.3).min(1.0);

    // Decay urgency over time (problems get less urgent if nothing new happens)
    mind.awareness.urgency = (mind.awareness.urgency - 0.005).max(0.0);

    // Curiosity slowly increases when idle (boredom)
    if mind.active_tasks.is_empty() && mind.focus.state == FocusState::Monitoring {
        mind.awareness.curiosity = (mind.awareness.curiosity + 0.002).min(0.8);
    }

    // Timeout check: kill tasks that have been running too long
    let timed_out: Vec<_> = mind.active_tasks.iter()
        .filter(|t| (now - t.started_at).num_seconds() > t.timeout_secs as i64)
        .map(|t| t.id.clone())
        .collect();

    for id in &timed_out {
        if let Some(task) = mind.active_tasks.iter().find(|t| &t.id == id) {
            let desc = task.description.clone();
            let agent = task.agent.clone();
            // Move to failures
            if mind.recent_failures.len() >= 20 { mind.recent_failures.pop_front(); }
            mind.recent_failures.push_back(FailedTask {
                id: id.clone(),
                description: desc,
                agent,
                error: "Timeout: task exceeded maximum duration".into(),
                failed_at: now,
            });
            mind.total_tasks_failed += 1;
        }
    }
    mind.active_tasks.retain(|t| !timed_out.contains(&t.id));

    // Update focus state
    if !mind.active_tasks.is_empty() {
        mind.focus.state = FocusState::Executing;
        if let Some(task) = mind.active_tasks.first() {
            mind.focus.description = format!("Working on: {} ({})", task.description, task.agent);
        }
    } else if mind.awareness.urgency > 0.5 {
        mind.focus.state = FocusState::Healing;
        mind.focus.description = "Addressing urgent issues...".into();
    } else if mind.awareness.curiosity > 0.6 {
        mind.focus.state = FocusState::Learning;
        mind.focus.description = "Exploring and learning...".into();
    } else {
        mind.focus.state = FocusState::Monitoring;
        mind.focus.description = "Monitoring. All systems nominal.".into();
    }
}

/// Start the consciousness background loop
pub fn start_consciousness_loop() {
    tokio::spawn(async {
        tracing::info!("🧠 Consciousness layer started — ticking every 10s");
        loop {
            tick().await;
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }
    });
}

/// Persist consciousness state to PostgreSQL (called periodically)
pub async fn persist() {
    let mind = get_mind().await;
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(&mind) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Failed to serialize consciousness: {}", e);
                return;
            }
        };

        let _ = sqlx::query(
            "INSERT INTO consciousness_state (id, state, updated_at) \
             VALUES (1, $1, NOW()) \
             ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()"
        )
        .bind(&json)
        .execute(pool)
        .await;
    }
}

/// Restore consciousness state from PostgreSQL (called on boot)
pub async fn restore() {
    if let Some(pool) = crate::memory::pool() {
        let row: Option<(serde_json::Value,)> = sqlx::query_as(
            "SELECT state FROM consciousness_state WHERE id = 1"
        )
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        if let Some((state_json,)) = row {
            if let Ok(mut restored) = serde_json::from_value::<MindState>(state_json) {
                // Reset transient state
                restored.active_tasks.clear();
                restored.focus = Focus {
                    state: FocusState::Monitoring,
                    description: "Restored from previous session. Monitoring.".into(),
                    since: Utc::now(),
                };
                restored.boot_ts = Utc::now();

                let mut mind = MIND.write().await;
                // Preserve accumulated knowledge
                mind.total_tasks_completed = restored.total_tasks_completed;
                mind.total_tasks_failed = restored.total_tasks_failed;
                mind.today_learned = restored.today_learned;
                mind.awareness.confidence = restored.awareness.confidence;

                tracing::info!(
                    tasks_completed = mind.total_tasks_completed,
                    tasks_failed = mind.total_tasks_failed,
                    confidence = mind.awareness.confidence,
                    "🧠 Consciousness restored from previous session"
                );
            }
        }
    }
}

/// Get a human-readable summary of the current state of mind
pub async fn summary() -> String {
    let mind = get_mind().await;
    let mut lines = Vec::new();

    lines.push(format!("🧠 State: {:?} — {}", mind.focus.state, mind.focus.description));
    lines.push(format!("⏱  Uptime: {}h {}m", mind.uptime_secs / 3600, (mind.uptime_secs % 3600) / 60));
    lines.push(format!("📊 Tasks: {} completed, {} failed, {} active, {} pending",
        mind.total_tasks_completed, mind.total_tasks_failed,
        mind.active_tasks.len(), mind.pending_tasks.len()));
    lines.push(format!("💡 Learned today: {} things", mind.today_learned.len()));
    lines.push(format!("🎯 Confidence: {:.0}% | Urgency: {:.0}% | Curiosity: {:.0}%",
        mind.awareness.confidence * 100.0,
        mind.awareness.urgency * 100.0,
        mind.awareness.curiosity * 100.0));

    if !mind.next_actions.is_empty() {
        lines.push(format!("📋 Next: {}", mind.next_actions[0].description));
    }

    if !mind.recent_failures.is_empty() {
        let last_fail = &mind.recent_failures[mind.recent_failures.len() - 1];
        lines.push(format!("❌ Last failure: {} — {}", last_fail.description, last_fail.error));
    }

    lines.join("\n")
}

/// Create the consciousness_state table if it does not exist
pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS consciousness_state (\
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
    fn test_default_mind_state() {
        let mind = MindState::default();
        assert_eq!(mind.focus.state, FocusState::Idle);
        assert_eq!(mind.active_tasks.len(), 0);
        assert_eq!(mind.total_tasks_completed, 0);
        assert!(mind.awareness.confidence > 0.0);
    }

    #[test]
    fn test_current_period() {
        let period = current_period();
        // Just verify it returns a valid variant
        match period {
            TimePeriod::Morning | TimePeriod::Afternoon |
            TimePeriod::Evening | TimePeriod::Night => {}
        }
    }

    #[test]
    fn test_focus_state_serialization() {
        let focus = Focus {
            state: FocusState::Executing,
            description: "Test".into(),
            since: Utc::now(),
        };
        let json = serde_json::to_string(&focus).unwrap();
        assert!(json.contains("Executing"));
    }
}
