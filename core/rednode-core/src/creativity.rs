// RedNode-OS — Creativity Engine
//
// Separate from reasoning. Generates novel ideas for:
//   architecture design, coding ideas, UI design, branding,
//   research hypotheses, documentation, infrastructure optimization.
//
// Creativity uses divergent thinking: generate many options,
// then reasoning selects the best. The two paths are intentionally
// separate so analytical constraints don't kill creative exploration.
//
// API:
//   POST /creativity/brainstorm — generate ideas for a topic
//   GET  /creativity/ideas      — recent ideas
//   POST /creativity/evaluate   — rate an idea

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

static CREATIVE: once_cell::sync::Lazy<Arc<RwLock<CreativityState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(CreativityState::default())));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainstormRequest { pub topic: String, pub domain: CreativeDomain, pub constraints: Vec<String>, pub count: u32 }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum CreativeDomain { Architecture, Coding, UI, Branding, Research, Documentation, Infrastructure, Security, Writing, Visual, Business, Custom(String) }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreativeIdea {
    pub id: String, pub topic: String, pub domain: CreativeDomain, pub title: String,
    pub description: String, pub novelty_score: f32, pub feasibility_score: f32,
    pub rating: Option<f32>, pub created_at: DateTime<Utc>, pub status: IdeaStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum IdeaStatus { New, Evaluated, Accepted, Rejected, Implemented }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrainstormResult { pub id: String, pub topic: String, pub ideas: Vec<CreativeIdea>, pub timestamp: DateTime<Utc> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreativityState { pub ideas: VecDeque<CreativeIdea>, pub sessions: VecDeque<BrainstormResult>, pub total_ideas: u64, pub total_sessions: u64 }

impl Default for CreativityState { fn default() -> Self { Self { ideas: VecDeque::with_capacity(200), sessions: VecDeque::with_capacity(50), total_ideas: 0, total_sessions: 0 } } }

fn gen_id(prefix: &str) -> String { format!("{}_{}", prefix, chrono::Utc::now().timestamp_millis()) }

/// Run a brainstorm session — generates creative ideas via templates + randomization
pub async fn brainstorm(request: BrainstormRequest) -> BrainstormResult {
    let count = request.count.min(20).max(1);
    let mut ideas = Vec::new();

    let templates = match request.domain {
        CreativeDomain::Architecture => vec![
            ("Microservice decomposition", "Split into focused services communicating via message queue"),
            ("Event sourcing", "Store state as event log instead of mutable records"),
            ("CQRS pattern", "Separate read and write paths for different optimization"),
            ("Plugin architecture", "Make core minimal, extend via plugins"),
            ("Edge computing", "Push computation to edge devices, aggregate centrally"),
        ],
        CreativeDomain::Security => vec![
            ("Honeypot integration", "Deploy decoy services to detect intrusion attempts"),
            ("Behavioral analysis", "Profile normal behavior, alert on deviations"),
            ("Zero-trust mesh", "Every service authenticates to every other service"),
            ("Automated forensics", "Auto-capture memory dump and network state on incident"),
            ("Chaos engineering", "Randomly inject failures to test resilience"),
        ],
        CreativeDomain::Infrastructure => vec![
            ("Immutable infrastructure", "Never patch — replace with new, tested images"),
            ("GitOps pipeline", "All infra changes via git commits, auto-applied"),
            ("Self-healing clusters", "Services auto-restart, auto-scale, auto-migrate"),
            ("Declarative networking", "Define network state, let system converge to it"),
            ("Predictive scaling", "Scale resources based on predicted demand, not current"),
        ],
        CreativeDomain::Writing => vec![
            ("Narrative structure", "Tell the story through a problem-solution-result arc"),
            ("Audience adaptation", "Rewrite for a different audience level"),
            ("Analogy technique", "Explain using a familiar analogy from another domain"),
            ("Compression challenge", "Convey the same meaning in half the words"),
            ("Multi-format", "Present as FAQ, tutorial, reference, and cheatsheet"),
        ],
        CreativeDomain::Visual => vec![
            ("Dashboard redesign", "Organize information by priority and frequency of use"),
            ("Color semantics", "Use color to encode meaning — red for danger, green for healthy"),
            ("Progressive disclosure", "Show summary first, details on demand"),
            ("Spatial mapping", "Arrange elements to mirror physical/logical topology"),
            ("Animation for state", "Use transitions to show state changes over time"),
        ],
        CreativeDomain::Business => vec![
            ("Value proposition", "Define the unique value this provides vs alternatives"),
            ("Risk-reward matrix", "Map all options by risk level and potential reward"),
            ("Resource optimization", "Identify the highest-impact use of limited resources"),
            ("Stakeholder mapping", "Identify who benefits, who decides, who blocks"),
            ("Monetization model", "Explore subscription, one-time, freemium, or open-core"),
        ],
        _ => vec![
            ("Lateral thinking", "Apply concepts from an unrelated domain"),
            ("Constraint removal", "What if we removed the biggest limitation?"),
            ("Combination", "Combine two existing capabilities into something new"),
            ("Simplification", "What is the simplest version that still works?"),
            ("Inversion", "What if we did the opposite of the current approach?"),
        ],
    };

    for (i, (title, desc)) in templates.iter().enumerate() {
        if i >= count as usize { break; }
        let idea = CreativeIdea {
            id: gen_id("idea"), topic: request.topic.clone(), domain: request.domain.clone(),
            title: format!("{}: {}", title, request.topic), description: desc.to_string(),
            novelty_score: 0.5 + (i as f32 * 0.1).min(0.4),
            feasibility_score: 0.8 - (i as f32 * 0.1).max(0.0),
            rating: None, created_at: Utc::now(), status: IdeaStatus::New,
        };
        ideas.push(idea);
    }

    let result = BrainstormResult { id: gen_id("bs"), topic: request.topic, ideas: ideas.clone(), timestamp: Utc::now() };

    let mut state = CREATIVE.write().await;
    for idea in ideas { if state.ideas.len() >= 200 { state.ideas.pop_front(); } state.ideas.push_back(idea); state.total_ideas += 1; }
    if state.sessions.len() >= 50 { state.sessions.pop_front(); } state.sessions.push_back(result.clone());
    state.total_sessions += 1;

    result
}

pub async fn rate_idea(idea_id: &str, rating: f32) -> bool {
    let mut state = CREATIVE.write().await;
    if let Some(idea) = state.ideas.iter_mut().find(|i| i.id == idea_id) {
        idea.rating = Some(rating.clamp(0.0, 1.0));
        idea.status = if rating > 0.6 { IdeaStatus::Accepted } else { IdeaStatus::Rejected };
        return true;
    }
    false
}

pub async fn get_ideas(limit: usize) -> Vec<CreativeIdea> { CREATIVE.read().await.ideas.iter().rev().take(limit).cloned().collect() }
pub async fn get_stats() -> serde_json::Value {
    let s = CREATIVE.read().await;
    serde_json::json!({ "total_ideas": s.total_ideas, "total_sessions": s.total_sessions, "idea_count": s.ideas.len() })
}

pub async fn persist() { let s = CREATIVE.read().await.clone(); if let Some(pool) = crate::memory::pool() { let json = match serde_json::to_value(&s) { Ok(v) => v, Err(_) => return }; let _ = sqlx::query("INSERT INTO creativity_store (id, state, updated_at) VALUES (1, $1, NOW()) ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()").bind(&json).execute(pool).await; } }
pub async fn restore() { if let Some(pool) = crate::memory::pool() { let row: Option<(serde_json::Value,)> = sqlx::query_as("SELECT state FROM creativity_store WHERE id = 1").fetch_optional(pool).await.unwrap_or(None); if let Some((json,)) = row { if let Ok(r) = serde_json::from_value::<CreativityState>(json) { let mut s = CREATIVE.write().await; *s = r; } } } }
pub async fn init_table() { if let Some(pool) = crate::memory::pool() { let _ = sqlx::query("CREATE TABLE IF NOT EXISTS creativity_store (id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())").execute(pool).await; } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn test_default() { let s = CreativityState::default(); assert_eq!(s.total_ideas, 0); }
    #[test] fn test_domain_eq() { assert_eq!(CreativeDomain::Security, CreativeDomain::Security); }
}
