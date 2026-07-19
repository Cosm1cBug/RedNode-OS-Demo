// RedNode-OS — Cognitive Load Manager
//
// Schedules INTELLIGENCE rather than processes. Decides:
//   - Which agents should run vs pause
//   - Which tasks get priority vs defer
//   - Which goals receive attention now
//   - How to allocate cognitive resources (LLM calls, CPU, memory)
//
// Different from the Economy (which tracks cost). Cognitive Load
// manages the CAPACITY of the system to think — preventing
// overload that degrades decision quality.
//
// API:
//   GET  /cognitive-load       — current load state
//   POST /cognitive-load/pause — pause a task/agent
//   POST /cognitive-load/resume — resume a paused task/agent

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

static COGLOAD: once_cell::sync::Lazy<Arc<RwLock<CognitiveLoadState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(CognitiveLoadState::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveLoadState {
    /// Current overall cognitive load (0.0 = idle, 1.0 = overloaded)
    pub load_level: f32,
    /// Active cognitive tasks consuming attention
    pub active_slots: Vec<CognitiveSlot>,
    /// Maximum concurrent cognitive tasks
    pub max_slots: u32,
    /// Paused tasks waiting for capacity
    pub paused: Vec<PausedTask>,
    /// Agent load tracking
    pub agent_load: HashMap<String, AgentLoad>,
    /// Load thresholds
    pub thresholds: LoadThresholds,
    pub total_throttled: u64,
    pub total_paused: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CognitiveSlot {
    pub id: String,
    pub task_description: String,
    pub agent: String,
    pub cognitive_cost: f32,
    pub started_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PausedTask {
    pub id: String,
    pub task_description: String,
    pub agent: String,
    pub reason: String,
    pub paused_at: DateTime<Utc>,
    pub priority: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentLoad {
    pub active_tasks: u32,
    pub max_concurrent: u32,
    pub paused: bool,
    pub last_task_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadThresholds {
    /// Above this load level, new LLM calls are deferred
    pub llm_throttle: f32,
    /// Above this, low-priority tasks are paused
    pub pause_low_priority: f32,
    /// Above this, only security/critical tasks proceed
    pub critical_only: f32,
}

impl Default for CognitiveLoadState {
    fn default() -> Self {
        Self {
            load_level: 0.0,
            active_slots: Vec::new(),
            max_slots: 8,
            paused: Vec::new(),
            agent_load: HashMap::new(),
            thresholds: LoadThresholds {
                llm_throttle: 0.7,
                pause_low_priority: 0.8,
                critical_only: 0.95,
            },
            total_throttled: 0,
            total_paused: 0,
        }
    }
}

fn gen_id() -> String { format!("cog_{}", chrono::Utc::now().timestamp_millis()) }

// ─── Public API ───

/// Check if there is capacity for a new cognitive task
pub async fn can_accept(priority: f32, is_security: bool) -> bool {
    let state = COGLOAD.read().await;
    if is_security { return true; } // Security always proceeds
    if state.load_level >= state.thresholds.critical_only { return false; }
    if state.load_level >= state.thresholds.pause_low_priority && priority < 0.5 { return false; }
    state.active_slots.len() < state.max_slots as usize
}

/// Allocate a cognitive slot for a task
pub async fn allocate(task_description: &str, agent: &str, cognitive_cost: f32) -> Option<String> {
    let mut state = COGLOAD.write().await;
    if state.active_slots.len() >= state.max_slots as usize {
        // Pause the task instead
        let id = gen_id();
        state.paused.push(PausedTask {
            id: id.clone(),
            task_description: task_description.into(),
            agent: agent.into(),
            reason: "Cognitive capacity exceeded".into(),
            paused_at: Utc::now(),
            priority: 0.5,
        });
        state.total_paused += 1;
        return None;
    }

    let id = gen_id();
    state.active_slots.push(CognitiveSlot {
        id: id.clone(),
        task_description: task_description.into(),
        agent: agent.into(),
        cognitive_cost: cognitive_cost.clamp(0.0, 1.0),
        started_at: Utc::now(),
    });

    // Update agent load
    let al = state.agent_load.entry(agent.into()).or_insert(AgentLoad {
        active_tasks: 0, max_concurrent: 3, paused: false, last_task_at: None,
    });
    al.active_tasks += 1;
    al.last_task_at = Some(Utc::now());

    recalculate_load(&mut state);
    Some(id)
}

/// Release a cognitive slot (task completed/failed)
pub async fn release(slot_id: &str) {
    let mut state = COGLOAD.write().await;
    if let Some(pos) = state.active_slots.iter().position(|s| s.id == slot_id) {
        let slot = state.active_slots.remove(pos);
        if let Some(al) = state.agent_load.get_mut(&slot.agent) {
            al.active_tasks = al.active_tasks.saturating_sub(1);
        }
    }
    recalculate_load(&mut state);

    // Resume highest-priority paused task if capacity available
    if state.active_slots.len() < state.max_slots as usize && !state.paused.is_empty() {
        state.paused.sort_by(|a, b| b.priority.partial_cmp(&a.priority).unwrap_or(std::cmp::Ordering::Equal));
        let resumed = state.paused.remove(0);
        state.active_slots.push(CognitiveSlot {
            id: resumed.id,
            task_description: resumed.task_description,
            agent: resumed.agent,
            cognitive_cost: 0.5,
            started_at: Utc::now(),
        });
    }
}

/// Manually pause an agent
pub async fn pause_agent(agent: &str) {
    let mut state = COGLOAD.write().await;
    let al = state.agent_load.entry(agent.into()).or_insert(AgentLoad {
        active_tasks: 0, max_concurrent: 3, paused: false, last_task_at: None,
    });
    al.paused = true;
}

/// Resume a paused agent
pub async fn resume_agent(agent: &str) {
    let mut state = COGLOAD.write().await;
    if let Some(al) = state.agent_load.get_mut(agent) {
        al.paused = false;
    }
}

/// Should LLM calls be throttled?
pub async fn should_throttle_llm() -> bool {
    let state = COGLOAD.read().await;
    state.load_level >= state.thresholds.llm_throttle
}

fn recalculate_load(state: &mut CognitiveLoadState) {
    let slot_load = state.active_slots.len() as f32 / state.max_slots as f32;
    let cost_load = state.active_slots.iter().map(|s| s.cognitive_cost).sum::<f32>() / state.max_slots as f32;
    state.load_level = (slot_load * 0.6 + cost_load * 0.4).clamp(0.0, 1.0);
}

pub async fn get_state() -> CognitiveLoadState { COGLOAD.read().await.clone() }

pub async fn get_stats() -> serde_json::Value {
    let s = COGLOAD.read().await;
    serde_json::json!({
        "load_level": s.load_level,
        "active_slots": s.active_slots.len(),
        "max_slots": s.max_slots,
        "paused_tasks": s.paused.len(),
        "total_throttled": s.total_throttled,
        "total_paused": s.total_paused,
    })
}

pub async fn tick() {
    let mut state = COGLOAD.write().await;
    // Timeout slots older than 5 minutes
    let cutoff = Utc::now() - chrono::Duration::minutes(5);
    state.active_slots.retain(|s| s.started_at > cutoff);
    recalculate_load(&mut state);
}

pub async fn persist() {
    let s = COGLOAD.read().await.clone();
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(&s) { Ok(v) => v, Err(_) => return };
        let _ = sqlx::query("INSERT INTO cognitive_load_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await;
    }
}
pub async fn restore() {
    if let Some(pool) = crate::memory::pool() {
        let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM cognitive_load_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None);
        if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<CognitiveLoadState>(json) { let mut s = COGLOAD.write().await; *s = r; } }
    }
}
pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query("CREATE TABLE IF NOT EXISTS cognitive_load_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_default() { let s = CognitiveLoadState::default(); assert_eq!(s.load_level, 0.0); assert_eq!(s.max_slots, 8); }
    #[test]
    fn test_thresholds() { let t = LoadThresholds { llm_throttle: 0.7, pause_low_priority: 0.8, critical_only: 0.95 }; assert!(t.llm_throttle < t.critical_only); }
}
