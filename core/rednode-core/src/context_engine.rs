// RedNode-OS — Context Engine
//
// Activates only the information RELEVANT to the current task.
// Memory stores information. Context SELECTS which information
// should be active for the current operation.
//
// API:
//   GET  /context            — current active context
//   POST /context/activate   — manually activate context
//   POST /context/clear      — clear active context

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

static CONTEXT: once_cell::sync::Lazy<Arc<RwLock<ContextState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(ContextState::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveContext {
    pub task_intent: Option<String>,
    pub relevant_goals: Vec<String>,
    pub relevant_episodes: Vec<String>,
    pub relevant_procedures: Vec<String>,
    pub relevant_world_entities: Vec<String>,
    pub relevant_knowledge: Vec<String>,
    pub agent_capabilities: Vec<String>,
    pub trust_context: std::collections::HashMap<String, f32>,
    pub time_context: TimeContext,
    pub activated_at: DateTime<Utc>,
    pub ttl_secs: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeContext {
    pub is_work_hours: bool,
    pub period: String,
    pub upcoming_deadlines: u32,
    pub overdue_tasks: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextState {
    pub active: ActiveContext,
    pub total_activations: u64,
    pub avg_context_items: f32,
}

impl Default for ContextState {
    fn default() -> Self {
        Self {
            active: ActiveContext {
                task_intent: None, relevant_goals: vec![], relevant_episodes: vec![],
                relevant_procedures: vec![], relevant_world_entities: vec![],
                relevant_knowledge: vec![], agent_capabilities: vec![],
                trust_context: std::collections::HashMap::new(),
                time_context: TimeContext { is_work_hours: false, period: "unknown".into(), upcoming_deadlines: 0, overdue_tasks: 0 },
                activated_at: Utc::now(), ttl_secs: 300,
            },
            total_activations: 0, avg_context_items: 0.0,
        }
    }
}

/// Build context for a given intent — pulls relevant info from all memory systems
pub async fn activate_for_intent(intent: &str) -> ActiveContext {
    let mut state = CONTEXT.write().await;

    // Gather relevant goals
    let goals = crate::goals::active().await;
    let relevant_goals: Vec<String> = goals.iter()
        .filter(|g| g.tags.iter().any(|t| intent.to_lowercase().contains(&t.to_lowercase())) || intent.to_lowercase().contains(&g.title.to_lowercase()))
        .map(|g| format!("{}: {}", g.title, g.progress))
        .collect();

    // Gather relevant episodes
    let episodes = crate::episodic_memory::search_episodes(intent, 5).await;
    let relevant_episodes: Vec<String> = episodes.iter().map(|e| format!("[{}] {}", e.category, e.title)).take(5).collect();

    // Gather relevant procedures
    let procedures = crate::episodic_memory::search_procedures(intent).await;
    let relevant_procedures: Vec<String> = procedures.iter().map(|p| p.name.clone()).take(5).collect();

    // Time context
    let time = crate::time_intel::get_awareness().await;
    let time_ctx = TimeContext {
        is_work_hours: time.is_work_hours,
        period: format!("{:?}", time.current_period),
        upcoming_deadlines: time.active_deadlines.len() as u32,
        overdue_tasks: time.overdue_patterns.len() as u32,
    };

    let ctx = ActiveContext {
        task_intent: Some(intent.into()),
        relevant_goals, relevant_episodes, relevant_procedures,
        relevant_world_entities: vec![], relevant_knowledge: vec![],
        agent_capabilities: vec![], trust_context: std::collections::HashMap::new(),
        time_context: time_ctx, activated_at: Utc::now(), ttl_secs: 300,
    };

    let total_items = ctx.relevant_goals.len() + ctx.relevant_episodes.len() + ctx.relevant_procedures.len();
    state.active = ctx.clone();
    state.total_activations += 1;
    let n = state.total_activations as f32;
    state.avg_context_items = (state.avg_context_items * (n - 1.0) + total_items as f32) / n;

    ctx
}

pub async fn get_active() -> ActiveContext { CONTEXT.read().await.active.clone() }

pub async fn clear() {
    let mut state = CONTEXT.write().await;
    state.active = ActiveContext {
        task_intent: None, relevant_goals: vec![], relevant_episodes: vec![],
        relevant_procedures: vec![], relevant_world_entities: vec![],
        relevant_knowledge: vec![], agent_capabilities: vec![],
        trust_context: std::collections::HashMap::new(),
        time_context: TimeContext { is_work_hours: false, period: "unknown".into(), upcoming_deadlines: 0, overdue_tasks: 0 },
        activated_at: Utc::now(), ttl_secs: 300,
    };
}

pub async fn get_stats() -> serde_json::Value {
    let s = CONTEXT.read().await;
    serde_json::json!({ "total_activations": s.total_activations, "avg_context_items": s.avg_context_items, "current_intent": s.active.task_intent })
}

pub async fn persist() { let s = CONTEXT.read().await.clone(); if let Some(pool) = crate::memory::pool() { let json = match serde_json::to_value(&s) { Ok(v) => v, Err(_) => return }; let _ = sqlx::query("INSERT INTO context_engine_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await; } }
pub async fn restore() { if let Some(pool) = crate::memory::pool() { let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM context_engine_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None); if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<ContextState>(json) { let mut s = CONTEXT.write().await; *s = r; } } } }
pub async fn init_table() { if let Some(pool) = crate::memory::pool() { let _ = sqlx::query("CREATE TABLE IF NOT EXISTS context_engine_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await; } }

#[cfg(test)]
mod tests { use super::*; #[test] fn test_default() { let s = ContextState::default(); assert!(s.active.task_intent.is_none()); } }
