// RedNode-OS — Internal Economy
//
// Every task gets a resource cost estimate. The planner uses cost data to
// choose the cheapest path that meets quality requirements. Budget limits
// prevent runaway resource consumption.
//
// Resources tracked:
//   - CPU time (milliseconds)
//   - RAM usage (megabytes)
//   - GPU time (milliseconds, if applicable)
//   - API calls (count — especially LLM invocations)
//   - Energy estimate (watt-hours)
//   - Disk I/O (megabytes read/written)
//
// Budget system:
//   - Daily budgets per resource type
//   - Per-task cost caps (reject tasks that exceed cost threshold)
//   - Dashboard shows spending over time
//
// Integration:
//   - Planner reads economy to choose cheapest viable plan
//   - Coordinator logs actual costs after execution
//   - Dashboard shows resource spending trends
//
// API:
//   GET  /economy           — current budget status
//   GET  /economy/history   — spending history
//   POST /economy/budget    — set budget limits

use chrono::{DateTime, Utc, Datelike};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

static ECONOMY: once_cell::sync::Lazy<Arc<RwLock<EconomyState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(EconomyState::default())));

/// Cost estimate for a single task
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCost {
    pub task_id: String,
    pub tool: String,
    pub agent: String,
    pub cpu_ms: u64,
    pub ram_mb: u64,
    pub gpu_ms: u64,
    pub api_calls: u32,
    pub energy_wh: f64,
    pub disk_io_mb: f64,
    pub timestamp: DateTime<Utc>,
}

/// Budget limits for a time period
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub daily_cpu_ms: u64,
    pub daily_ram_mb_peak: u64,
    pub daily_gpu_ms: u64,
    pub daily_api_calls: u32,
    pub daily_energy_wh: f64,
    pub per_task_cpu_ms_max: u64,
    pub per_task_api_calls_max: u32,
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            daily_cpu_ms: 3_600_000,      // 1 hour of CPU
            daily_ram_mb_peak: 8_192,      // 8 GB
            daily_gpu_ms: 7_200_000,       // 2 hours of GPU
            daily_api_calls: 500,          // 500 LLM calls
            daily_energy_wh: 1_000.0,      // 1 kWh
            per_task_cpu_ms_max: 120_000,  // 2 minutes per task
            per_task_api_calls_max: 20,    // 20 LLM calls per task
        }
    }
}

/// Daily spending summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySpend {
    pub date: String,
    pub cpu_ms: u64,
    pub ram_mb_peak: u64,
    pub gpu_ms: u64,
    pub api_calls: u32,
    pub energy_wh: f64,
    pub disk_io_mb: f64,
    pub task_count: u64,
}

/// The economy state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EconomyState {
    pub budget: Budget,
    pub today: DailySpend,
    pub history: VecDeque<DailySpend>,
    pub recent_costs: VecDeque<TaskCost>,
}

impl Default for EconomyState {
    fn default() -> Self {
        Self {
            budget: Budget::default(),
            today: DailySpend {
                date: Utc::now().format("%Y-%m-%d").to_string(),
                cpu_ms: 0,
                ram_mb_peak: 0,
                gpu_ms: 0,
                api_calls: 0,
                energy_wh: 0.0,
                disk_io_mb: 0.0,
                task_count: 0,
            },
            history: VecDeque::with_capacity(90),
            recent_costs: VecDeque::with_capacity(100),
        }
    }
}

// ─── Public API ───

/// Record the cost of a completed task
pub async fn record_cost(
    task_id: &str,
    tool: &str,
    agent: &str,
    cpu_ms: u64,
    ram_mb: u64,
    gpu_ms: u64,
    api_calls: u32,
) {
    let mut state = ECONOMY.write().await;
    let now = Utc::now();

    // Estimate energy from CPU+GPU time (rough: 100W TDP system)
    let energy_wh = (cpu_ms as f64 + gpu_ms as f64) / 3_600_000.0 * 100.0;

    let cost = TaskCost {
        task_id: task_id.into(),
        tool: tool.into(),
        agent: agent.into(),
        cpu_ms,
        ram_mb,
        gpu_ms,
        api_calls,
        energy_wh,
        disk_io_mb: 0.0,
        timestamp: now,
    };

    // Keep last 100 costs
    if state.recent_costs.len() >= 100 {
        state.recent_costs.pop_front();
    }
    state.recent_costs.push_back(cost);

    // Roll over day if needed
    let today_str = now.format("%Y-%m-%d").to_string();
    if state.today.date != today_str {
        // Archive yesterday
        if state.history.len() >= 90 {
            state.history.pop_front();
        }
        state.history.push_back(state.today.clone());
        state.today = DailySpend {
            date: today_str,
            cpu_ms: 0,
            ram_mb_peak: 0,
            gpu_ms: 0,
            api_calls: 0,
            energy_wh: 0.0,
            disk_io_mb: 0.0,
            task_count: 0,
        };
    }

    // Update today's spend
    state.today.cpu_ms += cpu_ms;
    state.today.gpu_ms += gpu_ms;
    state.today.api_calls += api_calls;
    state.today.energy_wh += energy_wh;
    state.today.task_count += 1;
    if ram_mb > state.today.ram_mb_peak {
        state.today.ram_mb_peak = ram_mb;
    }
}

/// Check if a task would exceed budget limits
pub async fn check_budget(estimated_cpu_ms: u64, estimated_api_calls: u32) -> BudgetCheck {
    let state = ECONOMY.read().await;

    let cpu_remaining = state.budget.daily_cpu_ms.saturating_sub(state.today.cpu_ms);
    let api_remaining = state.budget.daily_api_calls.saturating_sub(state.today.api_calls);

    let per_task_ok = estimated_cpu_ms <= state.budget.per_task_cpu_ms_max
        && estimated_api_calls <= state.budget.per_task_api_calls_max;

    let daily_ok = estimated_cpu_ms <= cpu_remaining
        && estimated_api_calls <= api_remaining as u32;

    BudgetCheck {
        allowed: per_task_ok && daily_ok,
        per_task_ok,
        daily_ok,
        cpu_remaining,
        api_remaining: api_remaining as u32,
        reason: if !per_task_ok {
            "Task exceeds per-task budget limit".into()
        } else if !daily_ok {
            "Task would exceed daily budget".into()
        } else {
            "Within budget".into()
        },
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BudgetCheck {
    pub allowed: bool,
    pub per_task_ok: bool,
    pub daily_ok: bool,
    pub cpu_remaining: u64,
    pub api_remaining: u32,
    pub reason: String,
}

/// Update budget limits
pub async fn set_budget(new_budget: Budget) {
    let mut state = ECONOMY.write().await;
    state.budget = new_budget;
    tracing::info!("Budget limits updated");
}

/// Get current budget status
pub async fn get_status() -> serde_json::Value {
    let state = ECONOMY.read().await;
    let cpu_pct = if state.budget.daily_cpu_ms > 0 {
        state.today.cpu_ms as f64 / state.budget.daily_cpu_ms as f64 * 100.0
    } else {
        0.0
    };
    let api_pct = if state.budget.daily_api_calls > 0 {
        state.today.api_calls as f64 / state.budget.daily_api_calls as f64 * 100.0
    } else {
        0.0
    };

    serde_json::json!({
        "budget": state.budget,
        "today": state.today,
        "cpu_usage_pct": cpu_pct,
        "api_usage_pct": api_pct,
        "tasks_today": state.today.task_count,
    })
}

/// Get spending history
pub async fn get_history() -> Vec<DailySpend> {
    ECONOMY.read().await.history.iter().cloned().collect()
}

/// Background tick — roll over day if needed
pub async fn tick() {
    let mut state = ECONOMY.write().await;
    let today_str = Utc::now().format("%Y-%m-%d").to_string();
    if state.today.date != today_str {
        if state.history.len() >= 90 {
            state.history.pop_front();
        }
        state.history.push_back(state.today.clone());
        state.today = DailySpend {
            date: today_str,
            cpu_ms: 0,
            ram_mb_peak: 0,
            gpu_ms: 0,
            api_calls: 0,
            energy_wh: 0.0,
            disk_io_mb: 0.0,
            task_count: 0,
        };
    }
}

// ─── Persistence ───

pub async fn persist() {
    let state = ECONOMY.read().await.clone();
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(&state) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Failed to serialize economy: {}", e);
                return;
            }
        };

        let _ = sqlx::query(
            "INSERT INTO economy_store (id, state, updated_at) \
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
            "SELECT state FROM economy_store WHERE id = 1"
        )
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        if let Some((json,)) = row {
            if let Ok(restored) = serde_json::from_value::<EconomyState>(json) {
                let mut state = ECONOMY.write().await;
                *state = restored;
                tracing::info!(
                    history_days = state.history.len(),
                    "Economy state restored"
                );
            }
        }
    }
}

pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS economy_store (\
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
    fn test_default_budget() {
        let b = Budget::default();
        assert!(b.daily_cpu_ms > 0);
        assert!(b.daily_api_calls > 0);
    }

    #[test]
    fn test_daily_spend_serialization() {
        let ds = DailySpend {
            date: "2026-07-19".into(),
            cpu_ms: 1000,
            ram_mb_peak: 4096,
            gpu_ms: 0,
            api_calls: 50,
            energy_wh: 5.0,
            disk_io_mb: 100.0,
            task_count: 10,
        };
        let json = serde_json::to_string(&ds).unwrap();
        assert!(json.contains("2026-07-19"));
    }

    #[test]
    fn test_task_cost_serialization() {
        let tc = TaskCost {
            task_id: "t1".into(),
            tool: "shell.run_safe".into(),
            agent: "system-agent".into(),
            cpu_ms: 500,
            ram_mb: 128,
            gpu_ms: 0,
            api_calls: 1,
            energy_wh: 0.01,
            disk_io_mb: 2.0,
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&tc).unwrap();
        assert!(json.contains("shell.run_safe"));
    }
}
