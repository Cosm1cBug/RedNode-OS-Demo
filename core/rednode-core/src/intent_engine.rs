// RedNode-OS — Intent Engine
//
// Converts vague objectives into explicit intents before planning.
// Example: "Make RedNode faster" → determines whether the goal is
// lower latency, lower RAM, reduced power, faster boot, or better inference.
//
// The intent engine sits BEFORE the planner:
//   Vague Input → Intent Engine → Clarified Intent → Planner → Steps
//
// API:
//   POST /intent/clarify     — clarify a vague intent
//   GET  /intent/history     — past clarifications

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

static INTENTS: once_cell::sync::Lazy<Arc<RwLock<IntentEngineState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(IntentEngineState::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClarifiedIntent {
    pub id: String,
    pub original: String,
    pub clarified: String,
    pub domain: String,
    pub specificity: f32,
    pub sub_intents: Vec<String>,
    pub ambiguities: Vec<Ambiguity>,
    pub needs_user_input: bool,
    pub confidence: f32,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ambiguity {
    pub question: String,
    pub options: Vec<String>,
    pub default: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentEngineState {
    pub history: VecDeque<ClarifiedIntent>,
    pub total_clarifications: u64,
    pub patterns: Vec<IntentPattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentPattern {
    pub trigger: String,
    pub domain: String,
    pub sub_intents: Vec<String>,
    pub clarifying_questions: Vec<String>,
}

impl Default for IntentEngineState {
    fn default() -> Self {
        Self {
            history: VecDeque::with_capacity(100),
            total_clarifications: 0,
            patterns: default_patterns(),
        }
    }
}

fn default_patterns() -> Vec<IntentPattern> {
    vec![
        IntentPattern { trigger: "faster".into(), domain: "performance".into(), sub_intents: vec!["reduce latency".into(), "reduce RAM usage".into(), "optimize CPU".into(), "faster boot".into()], clarifying_questions: vec!["What should be faster — response time, boot time, or resource usage?".into()] },
        IntentPattern { trigger: "secure".into(), domain: "security".into(), sub_intents: vec!["harden SSH".into(), "update firewall".into(), "scan vulnerabilities".into(), "check certificates".into()], clarifying_questions: vec!["Harden against what — external attacks, internal threats, or compliance?".into()] },
        IntentPattern { trigger: "fix".into(), domain: "troubleshooting".into(), sub_intents: vec!["diagnose issue".into(), "check logs".into(), "restart service".into(), "rollback change".into()], clarifying_questions: vec!["What is broken — a service, network, storage, or application?".into()] },
        IntentPattern { trigger: "backup".into(), domain: "data_protection".into(), sub_intents: vec!["create snapshot".into(), "verify integrity".into(), "test restore".into(), "schedule recurring".into()], clarifying_questions: vec!["Full backup, incremental, or verify existing?".into()] },
        IntentPattern { trigger: "monitor".into(), domain: "observability".into(), sub_intents: vec!["check health".into(), "view metrics".into(), "set alerts".into(), "review logs".into()], clarifying_questions: vec!["Monitor what — system health, network, services, or security?".into()] },
    ]
}

fn gen_id() -> String { format!("intent_{}", chrono::Utc::now().timestamp_millis()) }

/// Clarify a vague intent into specific actionable intents
pub async fn clarify(input: &str) -> ClarifiedIntent {
    let mut state = INTENTS.write().await;
    let input_lower = input.to_lowercase();

    // Match against known patterns
    let matched = state.patterns.iter().find(|p| input_lower.contains(&p.trigger));

    let (clarified, domain, sub_intents, ambiguities, needs_input, confidence, specificity) = match matched {
        Some(pattern) => {
            let ambiguities: Vec<Ambiguity> = pattern.clarifying_questions.iter().map(|q| Ambiguity {
                question: q.clone(),
                options: pattern.sub_intents.clone(),
                default: pattern.sub_intents.first().cloned(),
            }).collect();

            let has_specifics = pattern.sub_intents.iter().any(|si| input_lower.contains(&si.to_lowercase()));

            if has_specifics {
                let specific: Vec<String> = pattern.sub_intents.iter().filter(|si| input_lower.contains(&si.to_lowercase())).cloned().collect();
                (input.into(), pattern.domain.clone(), specific, vec![], false, 0.9, 0.9)
            } else {
                (format!("{} (domain: {})", input, pattern.domain), pattern.domain.clone(), pattern.sub_intents.clone(), ambiguities, true, 0.5, 0.3)
            }
        }
        None => {
            // No pattern match — pass through with lower confidence
            (input.into(), "general".into(), vec![input.into()], vec![], false, 0.6, 0.7)
        }
    };

    let result = ClarifiedIntent {
        id: gen_id(), original: input.into(), clarified, domain, specificity,
        sub_intents, ambiguities, needs_user_input: needs_input,
        confidence, timestamp: Utc::now(),
    };

    if state.history.len() >= 100 { state.history.pop_front(); }
    state.history.push_back(result.clone());
    state.total_clarifications += 1;

    result
}

pub async fn get_history(limit: usize) -> Vec<ClarifiedIntent> {
    INTENTS.read().await.history.iter().rev().take(limit).cloned().collect()
}

pub async fn persist() { let s = INTENTS.read().await.clone(); if let Some(pool) = crate::memory::pool() { let json = match serde_json::to_value(&s) { Ok(v) => v, Err(_) => return }; let _ = sqlx::query("INSERT INTO intent_engine_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await; } }
pub async fn restore() { if let Some(pool) = crate::memory::pool() { let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM intent_engine_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None); if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<IntentEngineState>(json) { let mut s = INTENTS.write().await; *s = r; } } } }
pub async fn init_table() { if let Some(pool) = crate::memory::pool() { let _ = sqlx::query("CREATE TABLE IF NOT EXISTS intent_engine_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await; } }

#[cfg(test)]
mod tests { use super::*; #[test] fn test_default() { let s = IntentEngineState::default(); assert!(!s.patterns.is_empty()); } }
