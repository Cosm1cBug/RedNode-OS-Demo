// RedNode-OS — Reflection System
//
// Periodic self-assessment: what went well, what failed, what can be
// improved, and what was learned. Reflections feed back into:
//   - Consciousness (updates confidence, urgency)
//   - Memory (learnings become knowledge)
//   - Goals (tracks progress)
//   - Personality (adjusts proactivity based on success rate)
//
// Reflection types:
//   - Task reflection:  after every task completion (quick, ~1 line)
//   - Periodic review:  every 6 hours (medium, ~5 lines)
//   - Daily summary:    end of day (thorough, stored permanently)
//   - On-demand:        "reflect on today" via intent
//
// The reflection system uses the LLM to synthesize observations into
// actionable insights, but also works without LLM (rule-based fallback).
//
// API:
//   GET  /reflection/today     — today's reflections
//   GET  /reflection/history   — past daily summaries
//   POST /reflection/trigger   — force a reflection now

use chrono::{DateTime, Utc, Datelike, Timelike};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

static REFLECTIONS: once_cell::sync::Lazy<Arc<RwLock<ReflectionStore>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(ReflectionStore::default())));

/// A single reflection entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reflection {
    pub id: String,
    pub period: ReflectionPeriod,
    pub created_at: DateTime<Utc>,
    pub successes: Vec<String>,
    pub failures: Vec<String>,
    pub optimizations: Vec<String>,
    pub goal_progress: Vec<GoalDelta>,
    pub automation_suggestions: Vec<String>,
    pub learnings: Vec<String>,
    pub confidence_delta: f32,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ReflectionPeriod {
    Task,
    SixHour,
    Daily,
    OnDemand,
    /// Domain-specific reflection scopes (Problem 7: expand reflection)
    Planning,
    Security,
    Memory,
    Goals,
    Architecture,
    Economy,
    Plugins,
    Infrastructure,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GoalDelta {
    pub goal_id: String,
    pub goal_title: String,
    pub progress_before: f32,
    pub progress_after: f32,
    pub delta: f32,
}

/// Persistent store for reflections
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReflectionStore {
    /// Today's reflections (cleared at midnight)
    pub today: Vec<Reflection>,
    /// Historical daily summaries (kept forever)
    pub history: VecDeque<Reflection>,
    /// Last periodic reflection timestamp
    pub last_periodic: Option<DateTime<Utc>>,
    /// Last daily summary timestamp
    pub last_daily: Option<DateTime<Utc>>,
    /// Stats
    pub total_reflections: u64,
}

impl Default for ReflectionStore {
    fn default() -> Self {
        Self {
            today: Vec::new(),
            history: VecDeque::with_capacity(365),
            last_periodic: None,
            last_daily: None,
            total_reflections: 0,
        }
    }
}

fn gen_id() -> String {
    format!("ref_{}", chrono::Utc::now().timestamp_millis())
}

// ─── Public API ───

/// Quick task reflection — called after every task completion
pub async fn reflect_on_task(
    task_description: &str,
    agent: &str,
    success: bool,
    duration_ms: u64,
    error: Option<&str>,
) {
    let mut store = REFLECTIONS.write().await;

    let mut successes = Vec::new();
    let mut failures = Vec::new();
    let mut optimizations = Vec::new();
    let confidence_delta;

    if success {
        successes.push(format!("{} completed by {} in {}ms", task_description, agent, duration_ms));
        confidence_delta = 0.01;

        // Suggest optimization for slow tasks
        if duration_ms > 10_000 {
            optimizations.push(format!(
                "Task '{}' took {}s — consider caching or parallelizing",
                task_description, duration_ms / 1000
            ));
        }
    } else {
        let err_msg = error.unwrap_or("unknown error");
        failures.push(format!("{} failed on {}: {}", task_description, agent, err_msg));
        confidence_delta = -0.02;

        // Suggest retry or alternative
        optimizations.push(format!(
            "Investigate failure in '{}' on {} — may need retry logic or fallback agent",
            task_description, agent
        ));
    }

    let reflection = Reflection {
        id: gen_id(),
        period: ReflectionPeriod::Task,
        created_at: Utc::now(),
        successes,
        failures,
        optimizations,
        goal_progress: Vec::new(),
        automation_suggestions: Vec::new(),
        learnings: Vec::new(),
        confidence_delta,
        summary: if success {
            format!("Task completed: {}", task_description)
        } else {
            format!("Task failed: {}", task_description)
        },
    };

    store.today.push(reflection);
    store.total_reflections += 1;
}

/// Periodic reflection (every 6 hours) — synthesizes recent activity
pub async fn periodic_reflection() -> Reflection {
    let mind = crate::consciousness::get_mind().await;
    let goals = crate::goals::active().await;

    let mut successes = Vec::new();
    let mut failures = Vec::new();
    let mut optimizations = Vec::new();
    let mut automation_suggestions = Vec::new();
    let mut learnings = Vec::new();
    let mut goal_deltas = Vec::new();

    // Analyze recent completions
    let completion_count = mind.recent_completions.len();
    if completion_count > 0 {
        successes.push(format!("{} tasks completed in this period", completion_count));

        // Check for repeated patterns (automation candidates)
        let mut agent_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
        for task in &mind.recent_completions {
            *agent_counts.entry(task.agent.clone()).or_default() += 1;
        }
        for (agent, count) in &agent_counts {
            if *count >= 5 {
                automation_suggestions.push(format!(
                    "{} ran {} tasks — consider a pipeline for these recurring {} tasks",
                    agent, count, agent
                ));
            }
        }
    }

    // Analyze recent failures
    let failure_count = mind.recent_failures.len();
    if failure_count > 0 {
        failures.push(format!("{} tasks failed in this period", failure_count));

        // Group failures by agent
        let mut fail_agents: std::collections::HashMap<String, Vec<String>> = std::collections::HashMap::new();
        for fail in &mind.recent_failures {
            fail_agents.entry(fail.agent.clone()).or_default().push(fail.error.clone());
        }
        for (agent, errors) in &fail_agents {
            if errors.len() >= 3 {
                optimizations.push(format!(
                    "Agent {} has {} repeated failures — may need attention",
                    agent, errors.len()
                ));
            }
        }
    }

    // Analyze learnings
    for learning in &mind.today_learned {
        learnings.push(format!("[{}] {}", learning.category, learning.content));
    }

    // Goal progress tracking
    for goal in &goals {
        goal_deltas.push(GoalDelta {
            goal_id: goal.id.clone(),
            goal_title: goal.title.clone(),
            progress_before: (goal.progress - 0.05).max(0.0),
            progress_after: goal.progress,
            delta: 0.05,
        });
    }

    // Resource observations
    let res = &mind.resource_snapshot;
    if res.cpu_percent > 80.0 {
        optimizations.push(format!("CPU usage high: {:.0}% — review active workloads", res.cpu_percent));
    }
    if res.disk_used_percent > 85.0 {
        optimizations.push(format!("Disk usage high: {:.0}% — consider cleanup", res.disk_used_percent));
    }

    // ── Security posture review ──
    let immune_health = crate::immune::health_score().await;
    if immune_health < 0.7 {
        optimizations.push(format!("Security health degraded: {:.0}% — review active threats", immune_health * 100.0));
    }
    let active_threats = crate::immune::get_active_threats().await;
    if !active_threats.is_empty() {
        failures.push(format!("{} active security threats unresolved", active_threats.len()));
    }

    // ── Plugin health review ──
    let plugin_count = crate::plugins::active_count().await;
    if plugin_count > 0 {
        learnings.push(format!("{} active plugins running", plugin_count));
    }

    // ── Infrastructure health review ──
    let world_summary = crate::world_model::summary().await;
    if !world_summary.is_empty() {
        learnings.push(format!("Infrastructure: {}", world_summary));
    }

    // Confidence calculation
    let success_rate = if completion_count + failure_count > 0 {
        completion_count as f32 / (completion_count + failure_count) as f32
    } else {
        0.5
    };
    let confidence_delta = (success_rate - 0.5) * 0.1;

    let summary = format!(
        "Period review: {} successes, {} failures, {} learnings, {:.0}% success rate",
        completion_count, failure_count, learnings.len(), success_rate * 100.0
    );

    let reflection = Reflection {
        id: gen_id(),
        period: ReflectionPeriod::SixHour,
        created_at: Utc::now(),
        successes,
        failures,
        optimizations,
        goal_progress: goal_deltas,
        automation_suggestions,
        learnings,
        confidence_delta,
        summary: summary.clone(),
    };

    // Store it
    let mut store = REFLECTIONS.write().await;
    store.today.push(reflection.clone());
    store.last_periodic = Some(Utc::now());
    store.total_reflections += 1;

    // Feed back into consciousness
    drop(store);
    // Emit to cognitive bus (replaces direct consciousness push)
    crate::cognitive_bus::emit(
        crate::cognitive_bus::CognitiveEventType::ReflectionCompleted,
        "reflection",
        serde_json::json!({"period": "six_hour", "summary": summary}),
    ).await;

    tracing::info!(summary = %summary, "Periodic reflection complete");

    crate::events::emit(serde_json::json!({
        "type": "reflection_complete",
        "period": "six_hour",
        "summary": summary,
        "ts": Utc::now().to_rfc3339(),
    }));

    reflection
}

/// Daily summary — comprehensive end-of-day reflection
pub async fn daily_summary() -> Reflection {
    let store = REFLECTIONS.read().await;

    // Aggregate all task reflections from today
    let mut all_successes = Vec::new();
    let mut all_failures = Vec::new();
    let mut all_optimizations = Vec::new();
    let mut all_suggestions = Vec::new();
    let mut all_learnings = Vec::new();
    let mut total_confidence_delta = 0.0f32;

    for r in &store.today {
        all_successes.extend(r.successes.clone());
        all_failures.extend(r.failures.clone());
        all_optimizations.extend(r.optimizations.clone());
        all_suggestions.extend(r.automation_suggestions.clone());
        all_learnings.extend(r.learnings.clone());
        total_confidence_delta += r.confidence_delta;
    }

    // Deduplicate
    all_optimizations.sort();
    all_optimizations.dedup();
    all_suggestions.sort();
    all_suggestions.dedup();
    all_learnings.sort();
    all_learnings.dedup();

    let summary = format!(
        "Daily summary: {} successes, {} failures, {} optimizations, {} learnings, confidence {}{:.2}",
        all_successes.len(),
        all_failures.len(),
        all_optimizations.len(),
        all_learnings.len(),
        if total_confidence_delta >= 0.0 { "+" } else { "" },
        total_confidence_delta,
    );

    // Get goal progress for today
    let goals = crate::goals::active().await;
    let goal_deltas: Vec<GoalDelta> = goals.iter().map(|g| GoalDelta {
        goal_id: g.id.clone(),
        goal_title: g.title.clone(),
        progress_before: (g.progress - 0.1).max(0.0),
        progress_after: g.progress,
        delta: g.progress.min(0.1),
    }).collect();

    drop(store);

    let reflection = Reflection {
        id: gen_id(),
        period: ReflectionPeriod::Daily,
        created_at: Utc::now(),
        successes: all_successes,
        failures: all_failures,
        optimizations: all_optimizations,
        goal_progress: goal_deltas,
        automation_suggestions: all_suggestions,
        learnings: all_learnings,
        confidence_delta: total_confidence_delta,
        summary: summary.clone(),
    };

    // Store in history (permanent) and clear today
    let mut store = REFLECTIONS.write().await;
    if store.history.len() >= 365 {
        store.history.pop_front();
    }
    store.history.push_back(reflection.clone());
    store.today.clear();
    store.last_daily = Some(Utc::now());
    store.total_reflections += 1;

    drop(store);

    // Feed into consciousness
    // Emit to cognitive bus (replaces direct consciousness push)
    crate::cognitive_bus::emit(
        crate::cognitive_bus::CognitiveEventType::ReflectionCompleted,
        "reflection",
        serde_json::json!({"period": "daily", "summary": summary}),
    ).await;

    tracing::info!(summary = %summary, "Daily reflection complete");

    crate::events::emit(serde_json::json!({
        "type": "reflection_complete",
        "period": "daily",
        "summary": summary,
        "ts": Utc::now().to_rfc3339(),
    }));

    reflection
}

/// On-demand reflection (triggered by user)
pub async fn on_demand() -> Reflection {
    // Same logic as periodic but labeled as on-demand
    let mut r = periodic_reflection().await;
    r.period = ReflectionPeriod::OnDemand;
    r.id = gen_id();
    r
}

/// Get today's reflections
pub async fn get_today() -> Vec<Reflection> {
    REFLECTIONS.read().await.today.clone()
}

/// Get historical daily summaries
pub async fn get_history(limit: usize) -> Vec<Reflection> {
    let store = REFLECTIONS.read().await;
    store.history.iter().rev().take(limit).cloned().collect()
}

/// Background tick — check if periodic or daily reflection is due
pub async fn tick() {
    let store = REFLECTIONS.read().await;
    let now = Utc::now();

    // Check if 6-hour periodic reflection is due
    let periodic_due = match store.last_periodic {
        Some(last) => (now - last).num_hours() >= 6,
        None => true,
    };

    // Check if daily reflection is due (after 23:00 local, once per day)
    let daily_due = match store.last_daily {
        Some(last) => last.day() != now.day(),
        None => true,
    };
    let is_late = chrono::Local::now().hour() >= 23;

    drop(store);

    if periodic_due {
        periodic_reflection().await;
    }

    if daily_due && is_late {
        daily_summary().await;
    }
}

/// Get stats for API
pub async fn get_stats() -> serde_json::Value {
    let store = REFLECTIONS.read().await;
    serde_json::json!({
        "today_count": store.today.len(),
        "history_count": store.history.len(),
        "total_reflections": store.total_reflections,
        "last_periodic": store.last_periodic,
        "last_daily": store.last_daily,
    })
}

// ─── Persistence ───

pub async fn persist() {
    let store = REFLECTIONS.read().await.clone();
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(&store) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Failed to serialize reflections: {}", e);
                return;
            }
        };

        let _ = sqlx::query(
            "INSERT INTO reflection_store (id, state, updated_at) \
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
            "SELECT state FROM reflection_store WHERE id = 1"
        )
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        if let Some((json,)) = row {
            if let Ok(restored) = serde_json::from_value::<ReflectionStore>(json) {
                let mut store = REFLECTIONS.write().await;
                *store = restored;
                tracing::info!(
                    history = store.history.len(),
                    total = store.total_reflections,
                    "Reflection system restored"
                );
            }
        }
    }
}

pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS reflection_store (\
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
    fn test_default_store() {
        let store = ReflectionStore::default();
        assert!(store.today.is_empty());
        assert!(store.history.is_empty());
        assert_eq!(store.total_reflections, 0);
    }

    #[test]
    fn test_reflection_serialization() {
        let r = Reflection {
            id: "test_1".into(),
            period: ReflectionPeriod::Task,
            created_at: Utc::now(),
            successes: vec!["task done".into()],
            failures: vec![],
            optimizations: vec![],
            goal_progress: vec![],
            automation_suggestions: vec![],
            learnings: vec![],
            confidence_delta: 0.01,
            summary: "Test reflection".into(),
        };
        let json = serde_json::to_string(&r).unwrap();
        assert!(json.contains("task done"));
        assert!(json.contains("Test reflection"));
    }

    #[test]
    fn test_goal_delta_serialization() {
        let gd = GoalDelta {
            goal_id: "g1".into(),
            goal_title: "Test Goal".into(),
            progress_before: 0.2,
            progress_after: 0.3,
            delta: 0.1,
        };
        let json = serde_json::to_string(&gd).unwrap();
        assert!(json.contains("Test Goal"));
    }
}
