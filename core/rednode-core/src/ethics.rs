// RedNode-OS — Ethics & Values Layer
//
// Different from governance (which enforces rules).
// Ethics GUIDES choices when there's no explicit rule.
//
// Core values:
//   - Privacy over convenience
//   - Explainability over blind automation
//   - Safety over speed
//   - User ownership of data
//   - Transparency of actions
//   - Minimality (do the least needed, not the most possible)
//   - Reversibility (prefer undoable actions)
//
// The ethics layer is consulted when consciousness faces ambiguous
// decisions — no explicit policy, but multiple valid approaches.
//
// API:
//   GET  /ethics            — current ethical values
//   POST /ethics/evaluate   — evaluate an action ethically
//   GET  /ethics/dilemmas   — past ethical evaluations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

static ETHICS: once_cell::sync::Lazy<Arc<RwLock<EthicsState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(EthicsState::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthicalValue {
    pub id: String,
    pub name: String,
    pub description: String,
    pub priority: f32,
    pub examples: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthicalEvaluation {
    pub id: String,
    pub action: String,
    pub context: String,
    pub values_considered: Vec<ValueAssessment>,
    pub recommendation: EthicalRecommendation,
    pub reasoning: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValueAssessment {
    pub value_name: String,
    pub alignment: f32,
    pub concern: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EthicalRecommendation { Proceed, ProceedWithCaution, SeekGuidance, Abstain }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EthicsState {
    pub values: Vec<EthicalValue>,
    pub evaluations: VecDeque<EthicalEvaluation>,
    pub total_evaluations: u64,
}

impl Default for EthicsState {
    fn default() -> Self {
        Self {
            values: vec![
                EthicalValue { id: "val_privacy".into(), name: "Privacy over Convenience".into(), description: "When a convenient option requires exposing user data, choose the private option.".into(), priority: 1.0, examples: vec!["Use local LLM instead of cloud API".into(), "Store data locally instead of SaaS".into()] },
                EthicalValue { id: "val_explain".into(), name: "Explainability over Automation".into(), description: "Automated actions should always be explainable. If the reasoning cannot be articulated, do not act.".into(), priority: 0.9, examples: vec!["Log why a decision was made".into(), "Prefer simple plans over opaque ones".into()] },
                EthicalValue { id: "val_safety".into(), name: "Safety over Speed".into(), description: "A slower, safer approach is always preferred over a fast, risky one.".into(), priority: 0.95, examples: vec!["Run backup before migration".into(), "Test in staging before production".into()] },
                EthicalValue { id: "val_ownership".into(), name: "User Data Ownership".into(), description: "The user owns all data. RedNode is a steward, not an owner.".into(), priority: 1.0, examples: vec!["Never lock user out of their data".into(), "Export functionality always available".into()] },
                EthicalValue { id: "val_transparency".into(), name: "Transparency of Actions".into(), description: "Every action is visible to the owner. No silent operations.".into(), priority: 0.95, examples: vec!["Audit log for every operation".into(), "Notifications for autonomous actions".into()] },
                EthicalValue { id: "val_minimal".into(), name: "Minimality".into(), description: "Do the minimum needed to achieve the goal. Avoid over-engineering.".into(), priority: 0.7, examples: vec!["Don't scan 100 ports when 10 suffice".into(), "Don't collect data you don't need".into()] },
                EthicalValue { id: "val_reversible".into(), name: "Reversibility".into(), description: "Prefer actions that can be undone. Treat irreversibility as high risk.".into(), priority: 0.9, examples: vec!["Snapshot before changes".into(), "Soft-delete before hard-delete".into()] },
            ],
            evaluations: VecDeque::with_capacity(100),
            total_evaluations: 0,
        }
    }
}

fn gen_id() -> String { format!("eval_{}", chrono::Utc::now().timestamp_millis()) }

/// Evaluate an action against ethical values
pub async fn evaluate(action: &str, context: &str) -> EthicalEvaluation {
    let state = ETHICS.read().await;
    let action_lower = action.to_lowercase();

    let mut assessments = Vec::new();
    let mut total_alignment = 0.0f32;

    for value in &state.values {
        let (alignment, concern) = assess_value(value, &action_lower);
        total_alignment += alignment * value.priority;
        assessments.push(ValueAssessment {
            value_name: value.name.clone(),
            alignment,
            concern,
        });
    }

    let avg_alignment = total_alignment / state.values.iter().map(|v| v.priority).sum::<f32>();
    let recommendation = if avg_alignment > 0.8 { EthicalRecommendation::Proceed }
        else if avg_alignment > 0.6 { EthicalRecommendation::ProceedWithCaution }
        else if avg_alignment > 0.4 { EthicalRecommendation::SeekGuidance }
        else { EthicalRecommendation::Abstain };

    let reasoning = match recommendation {
        EthicalRecommendation::Proceed => "Action aligns well with ethical values.".into(),
        EthicalRecommendation::ProceedWithCaution => "Action is acceptable but has some ethical concerns.".into(),
        EthicalRecommendation::SeekGuidance => "Action has significant ethical concerns — seek owner guidance.".into(),
        EthicalRecommendation::Abstain => "Action conflicts with core ethical values — recommend against.".into(),
    };

    drop(state);

    let eval = EthicalEvaluation {
        id: gen_id(), action: action.into(), context: context.into(),
        values_considered: assessments, recommendation, reasoning, timestamp: Utc::now(),
    };

    let mut state = ETHICS.write().await;
    if state.evaluations.len() >= 100 { state.evaluations.pop_front(); }
    state.evaluations.push_back(eval.clone());
    state.total_evaluations += 1;

    eval
}

fn assess_value(value: &EthicalValue, action: &str) -> (f32, Option<String>) {
    match value.id.as_str() {
        "val_privacy" => {
            if action.contains("cloud") || action.contains("external") || action.contains("upload") {
                (0.3, Some("Action may expose data externally".into()))
            } else { (0.9, None) }
        }
        "val_safety" => {
            if action.contains("force") || action.contains("override") || action.contains("skip check") {
                (0.3, Some("Action bypasses safety checks".into()))
            } else { (0.9, None) }
        }
        "val_reversible" => {
            if action.contains("delete") || action.contains("format") || action.contains("purge") {
                (0.2, Some("Potentially irreversible action".into()))
            } else { (0.9, None) }
        }
        "val_transparency" => {
            if action.contains("silent") || action.contains("suppress") {
                (0.2, Some("Action may reduce transparency".into()))
            } else { (0.9, None) }
        }
        _ => (0.8, None),
    }
}

pub async fn get_values() -> Vec<EthicalValue> { ETHICS.read().await.values.clone() }
pub async fn get_evaluations(limit: usize) -> Vec<EthicalEvaluation> {
    ETHICS.read().await.evaluations.iter().rev().take(limit).cloned().collect()
}

pub async fn persist() {
    let state = ETHICS.read().await.clone();
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(&state) { Ok(v) => v, Err(_) => return };
        let _ = sqlx::query("INSERT INTO ethics_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await;
    }
}
pub async fn restore() {
    if let Some(pool) = crate::memory::pool() {
        let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM ethics_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None);
        if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<EthicsState>(json) { let mut s = ETHICS.write().await; *s = r; } }
    }
}
pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query("CREATE TABLE IF NOT EXISTS ethics_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_default() { let s = EthicsState::default(); assert!(!s.values.is_empty()); }
    #[test]
    fn test_recommendation_eq() { assert_eq!(EthicalRecommendation::Proceed, EthicalRecommendation::Proceed); }
}
