// RedNode-OS — Meta-Reasoning Engine
//
// Evaluates HOW RedNode thinks, not just WHAT it thinks.
// After every planning cycle, meta-reasoning asks:
//   - Did I choose the best planning strategy?
//   - Should I have used a different model?
//   - Was this tool selection optimal?
//   - Is there a better reasoning algorithm for this class of problem?
//   - Am I over-planning or under-planning?
//
// Meta-reasoning creates a feedback loop that improves the reasoning
// process itself. Over time, RedNode gets better at choosing HOW to
// think, not just better at thinking.
//
// Integration:
//   - Planner reports planning decisions → meta-reasoning evaluates
//   - Coordinator reports execution outcomes → meta-reasoning correlates
//   - Reflection system provides success/failure data
//   - Distillation system stores meta-reasoning insights
//
// API:
//   GET  /meta/analysis     — recent meta-reasoning analyses
//   GET  /meta/strategies   — known reasoning strategies and their scores
//   GET  /meta/suggestions  — current improvement suggestions
//   POST /meta/evaluate     — trigger evaluation of a specific decision

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;

static META: once_cell::sync::Lazy<Arc<RwLock<MetaReasoningState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(MetaReasoningState::default())));

/// A reasoning strategy that RedNode can employ
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningStrategy {
    pub id: String,
    pub name: String,
    pub description: String,
    pub applicable_to: Vec<String>,
    pub success_rate: f32,
    pub avg_duration_ms: u64,
    pub times_used: u64,
    pub times_succeeded: u64,
    pub last_used: Option<DateTime<Utc>>,
}

/// A single meta-reasoning analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaAnalysis {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub intent: String,
    pub strategy_used: String,
    pub plan_steps: u32,
    pub execution_ms: u64,
    pub success: bool,
    pub assessment: Assessment,
    pub suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Assessment {
    /// Was the strategy appropriate for this intent? (0-1)
    pub strategy_fitness: f32,
    /// Was the plan size appropriate? (0-1, 0.5 = just right)
    pub plan_efficiency: f32,
    /// Was tool selection optimal? (0-1)
    pub tool_selection: f32,
    /// Was the model appropriate? (0-1)
    pub model_fitness: f32,
    /// Overall reasoning quality score (0-1)
    pub overall: f32,
}

/// Improvement suggestion from meta-reasoning
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Suggestion {
    pub id: String,
    pub category: String,
    pub description: String,
    pub confidence: f32,
    pub created_at: DateTime<Utc>,
    pub applied: bool,
}

/// The meta-reasoning state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetaReasoningState {
    pub strategies: Vec<ReasoningStrategy>,
    pub analyses: VecDeque<MetaAnalysis>,
    pub suggestions: Vec<Suggestion>,
    pub total_analyses: u64,
    pub avg_reasoning_quality: f32,
}

impl Default for MetaReasoningState {
    fn default() -> Self {
        Self {
            strategies: default_strategies(),
            analyses: VecDeque::with_capacity(100),
            suggestions: Vec::new(),
            total_analyses: 0,
            avg_reasoning_quality: 0.5,
        }
    }
}

fn default_strategies() -> Vec<ReasoningStrategy> {
    vec![
        ReasoningStrategy {
            id: "strat_llm_plan".into(), name: "LLM Planning".into(),
            description: "Use LLM to decompose intent into tool steps".into(),
            applicable_to: vec!["general".into(), "complex".into()],
            success_rate: 0.7, avg_duration_ms: 3000, times_used: 0, times_succeeded: 0, last_used: None,
        },
        ReasoningStrategy {
            id: "strat_keyword".into(), name: "Keyword Fallback".into(),
            description: "Pattern-match intent to tools without LLM".into(),
            applicable_to: vec!["simple".into(), "known_pattern".into()],
            success_rate: 0.6, avg_duration_ms: 50, times_used: 0, times_succeeded: 0, last_used: None,
        },
        ReasoningStrategy {
            id: "strat_goap".into(), name: "GOAP Planning".into(),
            description: "Goal-oriented action planning for multi-step goals".into(),
            applicable_to: vec!["goal_oriented".into(), "multi_step".into()],
            success_rate: 0.65, avg_duration_ms: 500, times_used: 0, times_succeeded: 0, last_used: None,
        },
        ReasoningStrategy {
            id: "strat_pipeline".into(), name: "Pipeline Execution".into(),
            description: "Use pre-defined pipeline for known workflows".into(),
            applicable_to: vec!["recurring".into(), "workflow".into()],
            success_rate: 0.9, avg_duration_ms: 100, times_used: 0, times_succeeded: 0, last_used: None,
        },
        ReasoningStrategy {
            id: "strat_debate".into(), name: "Multi-Agent Debate".into(),
            description: "Multiple perspectives evaluate before deciding".into(),
            applicable_to: vec!["high_risk".into(), "ambiguous".into()],
            success_rate: 0.8, avg_duration_ms: 8000, times_used: 0, times_succeeded: 0, last_used: None,
        },
        ReasoningStrategy {
            id: "strat_tree_search".into(), name: "Tree Search".into(),
            description: "Systematic exploration of action space branching".into(),
            applicable_to: vec!["exploration".into(), "multi_path".into()],
            success_rate: 0.6, avg_duration_ms: 2000, times_used: 0, times_succeeded: 0, last_used: None,
        },
        ReasoningStrategy {
            id: "strat_constraint".into(), name: "Constraint Solver".into(),
            description: "Solve under resource/time/dependency constraints".into(),
            applicable_to: vec!["resource_allocation".into(), "scheduling".into()],
            success_rate: 0.7, avg_duration_ms: 1000, times_used: 0, times_succeeded: 0, last_used: None,
        },
        ReasoningStrategy {
            id: "strat_monte_carlo".into(), name: "Monte Carlo Sampling".into(),
            description: "Random sampling to explore large solution spaces".into(),
            applicable_to: vec!["exploration".into(), "optimization".into()],
            success_rate: 0.55, avg_duration_ms: 4000, times_used: 0, times_succeeded: 0, last_used: None,
        },
        ReasoningStrategy {
            id: "strat_hybrid".into(), name: "Hybrid Auto-Select".into(),
            description: "Automatically select best strategy based on intent class and past performance".into(),
            applicable_to: vec!["general".into(), "complex".into(), "simple".into(), "high_risk".into()],
            success_rate: 0.75, avg_duration_ms: 1500, times_used: 0, times_succeeded: 0, last_used: None,
        },
    ]
}

fn gen_id(prefix: &str) -> String {
    format!("{}_{}", prefix, chrono::Utc::now().timestamp_millis())
}

// ─── Public API ───

/// Record a planning decision and its outcome for meta-analysis
pub async fn record_decision(
    intent: &str, strategy: &str, plan_steps: u32,
    execution_ms: u64, success: bool,
) -> MetaAnalysis {
    let mut state = META.write().await;

    // Update strategy stats
    if let Some(s) = state.strategies.iter_mut().find(|s| s.id == strategy || s.name == strategy) {
        s.times_used += 1;
        if success { s.times_succeeded += 1; }
        s.success_rate = s.times_succeeded as f32 / s.times_used as f32;
        s.avg_duration_ms = ((s.avg_duration_ms as f64 * 0.9) + (execution_ms as f64 * 0.1)) as u64;
        s.last_used = Some(Utc::now());
    }

    // Assess the decision
    let strategy_fitness = if success { 0.8 } else { 0.3 };
    let plan_efficiency = match plan_steps {
        0 => 0.2,
        1..=3 => 0.9,
        4..=6 => 0.7,
        7..=10 => 0.5,
        _ => 0.3,
    };
    let tool_selection = if success { 0.8 } else { 0.4 };
    let model_fitness = if execution_ms < 5000 { 0.8 } else if execution_ms < 15000 { 0.6 } else { 0.4 };
    let overall = (strategy_fitness + plan_efficiency + tool_selection + model_fitness) / 4.0;

    let mut suggestions = Vec::new();
    if plan_steps > 8 {
        suggestions.push("Plan may be over-decomposed — consider combining steps".into());
    }
    if execution_ms > 10000 && strategy != "strat_debate" {
        suggestions.push("Slow execution — consider a faster reasoning strategy".into());
    }
    if !success {
        suggestions.push(format!("Strategy '{}' failed — evaluate alternatives", strategy));
    }

    let analysis = MetaAnalysis {
        id: gen_id("meta"),
        timestamp: Utc::now(),
        intent: intent.into(),
        strategy_used: strategy.into(),
        plan_steps,
        execution_ms,
        success,
        assessment: Assessment { strategy_fitness, plan_efficiency, tool_selection, model_fitness, overall },
        suggestions: suggestions.clone(),
    };

    if state.analyses.len() >= 100 { state.analyses.pop_front(); }
    state.analyses.push_back(analysis.clone());
    state.total_analyses += 1;

    // Update running average
    let total = state.total_analyses as f32;
    state.avg_reasoning_quality = (state.avg_reasoning_quality * (total - 1.0) + overall) / total;

    // Generate persistent suggestions
    for s in suggestions {
        state.suggestions.push(Suggestion {
            id: gen_id("sug"),
            category: "reasoning".into(),
            description: s,
            confidence: overall,
            created_at: Utc::now(),
            applied: false,
        });
    }
    // Keep only last 20 suggestions
    if state.suggestions.len() > 20 {
        state.suggestions = state.suggestions.split_off(state.suggestions.len() - 20);
    }

    analysis
}

/// Recommend the best strategy for a given intent class
pub async fn recommend_strategy(intent_class: &str) -> Option<ReasoningStrategy> {
    let state = META.read().await;
    state.strategies.iter()
        .filter(|s| s.applicable_to.iter().any(|a| a == intent_class || a == "general"))
        .max_by(|a, b| a.success_rate.partial_cmp(&b.success_rate).unwrap_or(std::cmp::Ordering::Equal))
        .cloned()
}

pub async fn get_strategies() -> Vec<ReasoningStrategy> {
    META.read().await.strategies.clone()
}

pub async fn get_analyses(limit: usize) -> Vec<MetaAnalysis> {
    let state = META.read().await;
    state.analyses.iter().rev().take(limit).cloned().collect()
}

pub async fn get_suggestions() -> Vec<Suggestion> {
    META.read().await.suggestions.iter().filter(|s| !s.applied).cloned().collect()
}

/// Benchmark all strategies — compare success rates and recommend the best per intent class.
/// Intended to be called during dreaming/idle periods.
pub async fn benchmark_strategies() -> serde_json::Value {
    let state = META.read().await;

    let mut rankings: Vec<serde_json::Value> = state.strategies.iter()
        .filter(|s| s.times_used > 0)
        .map(|s| {
            serde_json::json!({
                "name": s.name,
                "success_rate": s.success_rate,
                "avg_duration_ms": s.avg_duration_ms,
                "times_used": s.times_used,
                "efficiency": if s.avg_duration_ms > 0 { s.success_rate / (s.avg_duration_ms as f32 / 1000.0) } else { 0.0 },
                "applicable_to": s.applicable_to,
            })
        })
        .collect();

    rankings.sort_by(|a, b| {
        let ea = a["efficiency"].as_f64().unwrap_or(0.0);
        let eb = b["efficiency"].as_f64().unwrap_or(0.0);
        eb.partial_cmp(&ea).unwrap_or(std::cmp::Ordering::Equal)
    });

    // Build recommendations per intent class
    let mut recommendations: std::collections::HashMap<String, String> = std::collections::HashMap::new();
    let intent_classes = ["general", "complex", "simple", "high_risk", "goal_oriented", "recurring", "exploration", "resource_allocation"];
    for class in intent_classes {
        if let Some(best) = state.strategies.iter()
            .filter(|s| s.applicable_to.iter().any(|a| a == class) && s.times_used > 0)
            .max_by(|a, b| a.success_rate.partial_cmp(&b.success_rate).unwrap_or(std::cmp::Ordering::Equal))
        {
            recommendations.insert(class.into(), best.name.clone());
        }
    }

    serde_json::json!({
        "rankings": rankings,
        "recommendations": recommendations,
        "total_strategies": state.strategies.len(),
        "strategies_with_data": state.strategies.iter().filter(|s| s.times_used > 0).count(),
    })
}

pub async fn get_stats() -> serde_json::Value {
    let state = META.read().await;
    serde_json::json!({
        "total_analyses": state.total_analyses,
        "avg_quality": state.avg_reasoning_quality,
        "strategies": state.strategies.len(),
        "pending_suggestions": state.suggestions.iter().filter(|s| !s.applied).count(),
    })
}

// ─── Persistence ───

pub async fn persist() {
    let state = META.read().await.clone();
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(&state) { Ok(v) => v, Err(_) => return };
        let _ = sqlx::query(
            "INSERT INTO meta_reasoning_store (id, state, updated_at) \
             VALUES (1, $1, NOW()) \
             ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()"
        ).bind(&json).execute(pool).await;
    }
}

pub async fn restore() {
    if let Some(pool) = crate::memory::pool() {
        let row: Option<(serde_json::Value,)> = sqlx::query_as(
            "SELECT state FROM meta_reasoning_store WHERE id = 1"
        ).fetch_optional(pool).await.unwrap_or(None);
        if let Some((json,)) = row {
            if let Ok(restored) = serde_json::from_value::<MetaReasoningState>(json) {
                let mut state = META.write().await;
                *state = restored;
                tracing::info!(analyses = state.total_analyses, "Meta-reasoning restored");
            }
        }
    }
}

pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS meta_reasoning_store (\
                id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())"
        ).execute(pool).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_strategies() {
        let state = MetaReasoningState::default();
        assert!(!state.strategies.is_empty());
        assert!(state.strategies.iter().any(|s| s.name == "LLM Planning"));
    }

    #[test]
    fn test_assessment_serialization() {
        let a = Assessment { strategy_fitness: 0.8, plan_efficiency: 0.7, tool_selection: 0.9, model_fitness: 0.6, overall: 0.75 };
        let json = serde_json::to_string(&a).unwrap();
        assert!(json.contains("0.75"));
    }
}
