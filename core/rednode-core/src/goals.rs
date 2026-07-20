// RedNode-OS — Goal Engine
//
// Long-term objective management. Instead of only completing individual tasks,
// RedNode maintains persistent goals that guide its behavior over time.
//
// Example:
//   Goal: "Build RedNodeSec"
//   Sub-goals: Instagram growth, Website, GitHub, Branding, Security labs
//   Every completed task is evaluated: "Did this advance any active goal?"
//
// Goals persist in PostgreSQL, survive restarts, and influence:
//   - Consciousness (what should happen next)
//   - Curiosity engine (what topics to explore)
//   - Predictive intent (proactive suggestions aligned with goals)
//
// API:
//   GET    /goals              — list all goals
//   GET    /goals/:id          — goal detail with sub-goals and progress
//   POST   /goals              — create a new goal
//   POST   /goals/:id/subgoal  — add sub-goal
//   POST   /goals/:id/contribute — record a contribution
//   DELETE /goals/:id          — deactivate a goal

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

static GOALS: once_cell::sync::Lazy<Arc<RwLock<Vec<UserGoal>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(Vec::new())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserGoal {
    pub id: String,
    pub title: String,
    pub description: String,
    pub sub_goals: Vec<SubGoal>,
    pub progress: f32,
    pub status: GoalStatus,
    pub created_at: DateTime<Utc>,
    pub target_date: Option<DateTime<Utc>>,
    pub contributions: Vec<Contribution>,
    pub tags: Vec<String>,
    /// IDs of goals that must complete before this goal can proceed
    pub depends_on: Vec<String>,
    /// IDs of goals that potentially conflict with this one
    pub conflicts_with: Vec<String>,
    /// Risk score for pursuing this goal (0.0 safe, 1.0 risky)
    pub risk_score: f32,
    /// Estimated completion date based on current progress rate
    pub estimated_completion: Option<DateTime<Utc>>,
    /// Context inherited from a parent goal
    pub inherited_context: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum GoalStatus {
    Active,
    Paused,
    Completed,
    Abandoned,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubGoal {
    pub id: String,
    pub title: String,
    pub description: String,
    pub progress: f32,
    pub status: GoalStatus,
    pub metrics: Vec<Metric>,
    pub tasks_linked: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub current: f64,
    pub target: f64,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contribution {
    pub task_description: String,
    pub sub_goal_id: Option<String>,
    pub impact: f32,
    pub timestamp: DateTime<Utc>,
    pub auto_detected: bool,
}

fn gen_id() -> String {
    format!("goal_{}", chrono::Utc::now().timestamp_millis())
}

// ─── Public API ───

/// Get all goals
pub async fn list() -> Vec<UserGoal> {
    GOALS.read().await.clone()
}

/// Get active goals only
pub async fn active() -> Vec<UserGoal> {
    GOALS.read().await.iter()
        .filter(|g| g.status == GoalStatus::Active)
        .cloned()
        .collect()
}

/// Get a specific goal by ID
pub async fn get(id: &str) -> Option<UserGoal> {
    GOALS.read().await.iter()
        .find(|g| g.id == id)
        .cloned()
}

/// Create a new goal
pub async fn create(title: &str, description: &str, tags: Vec<String>, target_date: Option<DateTime<Utc>>) -> UserGoal {
    let goal = UserGoal {
        id: gen_id(),
        title: title.into(),
        description: description.into(),
        sub_goals: Vec::new(),
        progress: 0.0,
        status: GoalStatus::Active,
        created_at: Utc::now(),
        target_date,
        contributions: Vec::new(),
        tags,
        depends_on: Vec::new(),
        conflicts_with: Vec::new(),
        risk_score: 0.0,
        estimated_completion: None,
        inherited_context: None,
    };

    let mut goals = GOALS.write().await;
    goals.push(goal.clone());

    // Persist
    persist_goals(&goals).await;

    // Emit to cognitive bus (replaces direct consciousness push)
    crate::cognitive_bus::emit(
        crate::cognitive_bus::CognitiveEventType::GoalUpdated,
        "goals",
        serde_json::json!({"action": "created", "goal_id": goal.id, "title": title}),
    ).await;

    crate::events::emit(serde_json::json!({
        "type": "goal_created",
        "goal_id": goal.id,
        "title": title,
        "ts": Utc::now().to_rfc3339(),
    }));

    tracing::info!(goal = title, "🎯 New goal created");
    goal
}

/// Add a sub-goal to an existing goal
pub async fn add_sub_goal(goal_id: &str, title: &str, description: &str) -> Option<SubGoal> {
    let mut goals = GOALS.write().await;
    let goal = goals.iter_mut().find(|g| g.id == goal_id)?;

    let sub = SubGoal {
        id: format!("sub_{}", chrono::Utc::now().timestamp_millis()),
        title: title.into(),
        description: description.into(),
        progress: 0.0,
        status: GoalStatus::Active,
        metrics: Vec::new(),
        tasks_linked: Vec::new(),
    };

    goal.sub_goals.push(sub.clone());
    persist_goals(&goals).await;

    tracing::info!(goal = goal_id, sub_goal = title, "🎯 Sub-goal added");
    Some(sub)
}

/// Record a contribution to a goal (manual or auto-detected)
pub async fn contribute(goal_id: &str, task_description: &str, sub_goal_id: Option<&str>, impact: f32, auto_detected: bool) {
    let mut goals = GOALS.write().await;

    if let Some(goal) = goals.iter_mut().find(|g| g.id == goal_id) {
        let contribution = Contribution {
            task_description: task_description.into(),
            sub_goal_id: sub_goal_id.map(String::from),
            impact: impact.min(1.0).max(0.0),
            timestamp: Utc::now(),
            auto_detected,
        };

        goal.contributions.push(contribution);

        // Update sub-goal progress if specified
        if let Some(sid) = sub_goal_id {
            if let Some(sub) = goal.sub_goals.iter_mut().find(|s| s.id == sid) {
                sub.progress = (sub.progress + impact * 0.1).min(1.0);
                if sub.progress >= 1.0 {
                    sub.status = GoalStatus::Completed;
                }
            }
        }

        // Recalculate overall goal progress
        if !goal.sub_goals.is_empty() {
            let total_progress: f32 = goal.sub_goals.iter().map(|s| s.progress).sum();
            goal.progress = total_progress / goal.sub_goals.len() as f32;
        } else {
            // No sub-goals: progress from contribution count
            goal.progress = (goal.contributions.len() as f32 * 0.05).min(1.0);
        }

        // Check if goal is complete
        if goal.progress >= 1.0 {
            goal.status = GoalStatus::Completed;
            tracing::info!(goal = goal.title, "🎯 Goal completed!");
            crate::cognitive_bus::emit(
                crate::cognitive_bus::CognitiveEventType::GoalCompleted,
                "goals",
                serde_json::json!({"goal_id": goal.id, "title": goal.title}),
            ).await;
            crate::events::emit(serde_json::json!({
                "type": "goal_completed",
                "goal_id": goal.id,
                "title": goal.title,
                "ts": Utc::now().to_rfc3339(),
            }));
        }

        persist_goals(&goals).await;
    }
}

/// Auto-detect if a completed task contributes to any active goal
/// Called by the coordinator after every task completion
pub async fn auto_detect_contribution(task_description: &str, agent: &str) {
    let goals = GOALS.read().await;
    let active_goals: Vec<_> = goals.iter()
        .filter(|g| g.status == GoalStatus::Active)
        .collect();

    if active_goals.is_empty() {
        return;
    }

    let task_lower = task_description.to_lowercase();

    for goal in &active_goals {
        // Check if task description matches goal tags or sub-goal titles
        let matches_goal = goal.tags.iter().any(|tag| task_lower.contains(&tag.to_lowercase()))
            || task_lower.contains(&goal.title.to_lowercase());

        let matching_sub = goal.sub_goals.iter()
            .find(|s| s.status == GoalStatus::Active && task_lower.contains(&s.title.to_lowercase()));

        if matches_goal || matching_sub.is_some() {
            let sub_id = matching_sub.map(|s| s.id.as_str());
            let goal_id = goal.id.clone();
            drop(goals); // Release read lock before calling contribute (which needs write lock)

            contribute(&goal_id, task_description, sub_id, 0.5, true).await;

            tracing::info!(
                task = task_description,
                goal = goal.title,
                "🎯 Auto-detected contribution to goal"
            );
            return;
        }
    }
}

/// Update goal status
pub async fn set_status(goal_id: &str, status: GoalStatus) {
    let mut goals = GOALS.write().await;
    if let Some(goal) = goals.iter_mut().find(|g| g.id == goal_id) {
        goal.status = status;
        persist_goals(&goals).await;
    }
}

/// Get count of active goals (for consciousness)
pub async fn active_count() -> usize {
    GOALS.read().await.iter()
        .filter(|g| g.status == GoalStatus::Active)
        .count()
}

/// Add a dependency: goal_id depends on dependency_id
pub async fn add_dependency(goal_id: &str, dependency_id: &str) {
    let mut goals = GOALS.write().await;
    if let Some(goal) = goals.iter_mut().find(|g| g.id == goal_id) {
        if !goal.depends_on.contains(&dependency_id.to_string()) {
            goal.depends_on.push(dependency_id.to_string());
        }
    }
}

/// Get goals whose dependencies are all completed (ready to work on)
pub async fn get_ready_goals() -> Vec<UserGoal> {
    let goals = GOALS.read().await;
    goals.iter()
        .filter(|g| g.status == GoalStatus::Active)
        .filter(|g| {
            g.depends_on.iter().all(|dep_id| {
                goals.iter().any(|dg| dg.id == *dep_id && dg.status == GoalStatus::Completed)
            }) || g.depends_on.is_empty()
        })
        .cloned()
        .collect()
}

/// Detect conflicts between active goals (goals with overlapping but contradicting tags)
pub async fn check_conflicts() -> Vec<(String, String, String)> {
    let goals = GOALS.read().await;
    let active: Vec<_> = goals.iter().filter(|g| g.status == GoalStatus::Active).collect();
    let mut conflicts = Vec::new();

    for i in 0..active.len() {
        for j in (i + 1)..active.len() {
            // Check explicit conflicts_with
            if active[i].conflicts_with.contains(&active[j].id)
                || active[j].conflicts_with.contains(&active[i].id)
            {
                conflicts.push((
                    active[i].id.clone(),
                    active[j].id.clone(),
                    "Explicitly marked as conflicting".into(),
                ));
            }
        }
    }
    conflicts
}

/// Update estimated completion based on current progress rate
pub async fn update_predictions() {
    let mut goals = GOALS.write().await;
    let now = Utc::now();

    for goal in goals.iter_mut() {
        if goal.status != GoalStatus::Active || goal.progress <= 0.0 {
            continue;
        }
        let elapsed_days = (now - goal.created_at).num_days().max(1) as f32;
        let progress_per_day = goal.progress / elapsed_days;
        if progress_per_day > 0.001 {
            let remaining = 1.0 - goal.progress;
            let days_remaining = (remaining / progress_per_day) as i64;
            goal.estimated_completion = Some(now + chrono::Duration::days(days_remaining));
        }
    }
}

/// Persist goals to PostgreSQL
async fn persist_goals(goals: &[UserGoal]) {
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(goals) {
            Ok(v) => v,
            Err(_) => return,
        };

        let _ = sqlx::query(
            "INSERT INTO goal_store (id, goals, updated_at) \
             VALUES (1, $1, NOW()) \
             ON CONFLICT (id) DO UPDATE SET goals = $1, updated_at = NOW()"
        )
        .bind(&json)
        .execute(pool)
        .await;
    }
}

/// Restore goals from PostgreSQL (called on boot)
pub async fn restore() {
    if let Some(pool) = crate::memory::pool() {
        let row: Option<(serde_json::Value,)> = sqlx::query_as(
            "SELECT goals FROM goal_store WHERE id = 1"
        )
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        if let Some((goals_json,)) = row {
            if let Ok(restored) = serde_json::from_value::<Vec<UserGoal>>(goals_json) {
                let mut goals = GOALS.write().await;
                *goals = restored;
                tracing::info!(
                    count = goals.len(),
                    active = goals.iter().filter(|g| g.status == GoalStatus::Active).count(),
                    "🎯 Goals restored from database"
                );
            }
        }
    }
}

/// Get goals for API response
pub async fn get_for_api() -> serde_json::Value {
    let goals = list().await;
    let active = goals.iter().filter(|g| g.status == GoalStatus::Active).count();
    let completed = goals.iter().filter(|g| g.status == GoalStatus::Completed).count();

    serde_json::json!({
        "total": goals.len(),
        "active": active,
        "completed": completed,
        "goals": goals,
    })
}

/// Create the goal_store table if it does not exist
pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS goal_store (\
                id INTEGER PRIMARY KEY, \
                goals JSONB NOT NULL, \
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
    fn test_goal_creation() {
        let goal = UserGoal {
            id: "test_1".into(),
            title: "Test Goal".into(),
            description: "A test goal".into(),
            sub_goals: vec![],
            progress: 0.0,
            status: GoalStatus::Active,
            created_at: Utc::now(),
            target_date: None,
            contributions: vec![],
            tags: vec!["test".into()],
            depends_on: vec![],
            conflicts_with: vec![],
            risk_score: 0.0,
            estimated_completion: None,
            inherited_context: None,
        };
        assert_eq!(goal.status, GoalStatus::Active);
        assert_eq!(goal.progress, 0.0);
        assert!(goal.depends_on.is_empty());
        assert_eq!(goal.risk_score, 0.0);
    }

    #[test]
    fn test_goal_serialization() {
        let goal = UserGoal {
            id: "test_2".into(),
            title: "Serialize Test".into(),
            description: "Test".into(),
            sub_goals: vec![SubGoal {
                id: "sub_1".into(),
                title: "Sub test".into(),
                description: "Sub".into(),
                progress: 0.5,
                status: GoalStatus::Active,
                metrics: vec![],
                tasks_linked: vec![],
            }],
            progress: 0.25,
            status: GoalStatus::Active,
            created_at: Utc::now(),
            target_date: None,
            contributions: vec![],
            tags: vec![],
            depends_on: vec![],
            conflicts_with: vec![],
            risk_score: 0.1,
            estimated_completion: None,
            inherited_context: None,
        };
        let json = serde_json::to_string(&goal).expect("serialize goal");
        assert!(json.contains("Serialize Test"));
        assert!(json.contains("Sub test"));
        assert!(json.contains("risk_score"));
    }
}
