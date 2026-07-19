// RedNode-OS — Language Engine
//
// Manages all natural language processing capabilities:
//   - Prompt optimization (best system prompt per task type)
//   - Terminology consistency (glossary enforced across all LLM calls)
//   - Summarization (compress long text for consciousness)
//   - Information extraction (pull structured data from unstructured text)
//   - Communication templates (consistent messaging patterns)
//
// The language engine sits between modules and the LLM. Any module
// that needs to compose a prompt or process natural language text
// goes through the language engine for consistency.
//
// API:
//   POST /language/summarize   — summarize text
//   POST /language/extract     — extract structured info from text
//   GET  /language/glossary    — terminology glossary
//   POST /language/glossary    — add a term
//   GET  /language/stats       — language processing statistics

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

static LANG: once_cell::sync::Lazy<Arc<RwLock<LanguageState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(LanguageState::default())));

/// A glossary term — enforces consistent terminology
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlossaryTerm {
    pub term: String,
    pub definition: String,
    pub preferred_form: String,
    pub alternatives: Vec<String>,
    pub domain: String,
}

/// A prompt template for specific task types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    pub task_type: String,
    pub system_prompt: String,
    pub user_prompt_prefix: String,
    pub output_format: String,
    pub quality_score: f32,
    pub uses: u64,
}

/// Summarization result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Summary {
    pub original_length: usize,
    pub summary_length: usize,
    pub compression_ratio: f32,
    pub summary: String,
    pub key_points: Vec<String>,
}

/// Extraction result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Extraction {
    pub entities: Vec<ExtractedEntity>,
    pub facts: Vec<String>,
    pub actions: Vec<String>,
    pub sentiment: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtractedEntity {
    pub name: String,
    pub entity_type: String,
    pub confidence: f32,
}

/// The language engine state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageState {
    pub glossary: Vec<GlossaryTerm>,
    pub templates: Vec<PromptTemplate>,
    pub total_summaries: u64,
    pub total_extractions: u64,
    pub total_prompt_optimizations: u64,
}

impl Default for LanguageState {
    fn default() -> Self {
        Self {
            glossary: default_glossary(),
            templates: default_templates(),
            total_summaries: 0,
            total_extractions: 0,
            total_prompt_optimizations: 0,
        }
    }
}

fn default_glossary() -> Vec<GlossaryTerm> {
    vec![
        GlossaryTerm { term: "RedNode".into(), definition: "The RedNode-OS digital entity".into(), preferred_form: "RedNode".into(), alternatives: vec!["rednode".into(), "red node".into()], domain: "identity".into() },
        GlossaryTerm { term: "CNS".into(), definition: "Central Nervous System — the Rust core".into(), preferred_form: "CNS".into(), alternatives: vec!["core".into(), "central nervous system".into()], domain: "architecture".into() },
        GlossaryTerm { term: "agent".into(), definition: "A specialized TypeScript service handling a domain".into(), preferred_form: "agent".into(), alternatives: vec!["service".into(), "worker".into()], domain: "architecture".into() },
        GlossaryTerm { term: "tool".into(), definition: "A specific capability within an agent".into(), preferred_form: "tool".into(), alternatives: vec!["action".into(), "command".into(), "function".into()], domain: "execution".into() },
    ]
}

fn default_templates() -> Vec<PromptTemplate> {
    vec![
        PromptTemplate { task_type: "planning".into(), system_prompt: "You are a precise task planner. Decompose the intent into executable steps.".into(), user_prompt_prefix: "Plan this: ".into(), output_format: "JSON array of steps".into(), quality_score: 0.7, uses: 0 },
        PromptTemplate { task_type: "summarization".into(), system_prompt: "Summarize the following text concisely, preserving key facts.".into(), user_prompt_prefix: "Summarize: ".into(), output_format: "Plain text summary".into(), quality_score: 0.8, uses: 0 },
        PromptTemplate { task_type: "extraction".into(), system_prompt: "Extract entities, facts, and actions from the text as JSON.".into(), user_prompt_prefix: "Extract from: ".into(), output_format: "JSON with entities, facts, actions".into(), quality_score: 0.7, uses: 0 },
        PromptTemplate { task_type: "analysis".into(), system_prompt: "Analyze the following and provide structured observations.".into(), user_prompt_prefix: "Analyze: ".into(), output_format: "Structured analysis".into(), quality_score: 0.7, uses: 0 },
    ]
}

// ─── Public API ───

/// Summarize text (rule-based — for LLM-based, use the planner with summarization template)
pub async fn summarize(text: &str, max_sentences: usize) -> Summary {
    let mut state = LANG.write().await;
    state.total_summaries += 1;

    let sentences: Vec<&str> = text.split(|c| c == '.' || c == '!' || c == '?')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let key_points: Vec<String> = sentences.iter()
        .take(max_sentences)
        .map(|s| s.to_string())
        .collect();

    let summary = key_points.join(". ") + ".";

    Summary {
        original_length: text.len(),
        summary_length: summary.len(),
        compression_ratio: if text.is_empty() { 1.0 } else { summary.len() as f32 / text.len() as f32 },
        summary,
        key_points,
    }
}

/// Extract structured information from text (rule-based entity extraction)
pub async fn extract(text: &str) -> Extraction {
    let mut state = LANG.write().await;
    state.total_extractions += 1;

    let mut entities = Vec::new();
    let text_lower = text.to_lowercase();

    // Extract technology entities from glossary
    for term in &state.glossary {
        if text_lower.contains(&term.term.to_lowercase()) {
            entities.push(ExtractedEntity {
                name: term.preferred_form.clone(),
                entity_type: term.domain.clone(),
                confidence: 0.9,
            });
        }
    }

    // Extract simple facts (sentences with "is", "has", "runs")
    let facts: Vec<String> = text.split('.')
        .filter(|s| {
            let sl = s.to_lowercase();
            sl.contains(" is ") || sl.contains(" has ") || sl.contains(" runs ")
        })
        .map(|s| s.trim().to_string())
        .take(5)
        .collect();

    // Extract actions (sentences with imperative verbs)
    let actions: Vec<String> = text.split('.')
        .filter(|s| {
            let sl = s.trim().to_lowercase();
            sl.starts_with("check") || sl.starts_with("update") || sl.starts_with("restart")
            || sl.starts_with("install") || sl.starts_with("configure") || sl.starts_with("deploy")
        })
        .map(|s| s.trim().to_string())
        .take(5)
        .collect();

    Extraction { entities, facts, actions, sentiment: None }
}

/// Get the best prompt template for a task type
pub async fn get_template(task_type: &str) -> Option<PromptTemplate> {
    let mut state = LANG.write().await;
    if let Some(t) = state.templates.iter_mut().find(|t| t.task_type == task_type) {
        t.uses += 1;
        state.total_prompt_optimizations += 1;
        return Some(t.clone());
    }
    None
}

/// Add a glossary term
pub async fn add_glossary_term(term: GlossaryTerm) {
    let mut state = LANG.write().await;
    if !state.glossary.iter().any(|g| g.term == term.term) {
        state.glossary.push(term);
    }
}

/// Get the glossary
pub async fn get_glossary() -> Vec<GlossaryTerm> {
    LANG.read().await.glossary.clone()
}

/// Apply terminology consistency to text
pub async fn normalize_terminology(text: &str) -> String {
    let state = LANG.read().await;
    let mut result = text.to_string();
    for term in &state.glossary {
        for alt in &term.alternatives {
            // Case-insensitive replacement with preferred form
            let re = regex::RegexBuilder::new(&regex::escape(alt))
                .case_insensitive(true)
                .build();
            if let Ok(re) = re {
                result = re.replace_all(&result, term.preferred_form.as_str()).to_string();
            }
        }
    }
    result
}

pub async fn get_stats() -> serde_json::Value {
    let state = LANG.read().await;
    serde_json::json!({
        "glossary_terms": state.glossary.len(),
        "templates": state.templates.len(),
        "total_summaries": state.total_summaries,
        "total_extractions": state.total_extractions,
        "total_prompt_optimizations": state.total_prompt_optimizations,
    })
}

// ─── Persistence ───

pub async fn persist() {
    let state = LANG.read().await.clone();
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(&state) {
            Ok(v) => v,
            Err(e) => { tracing::warn!("Failed to serialize language engine: {}", e); return; }
        };
        let _ = sqlx::query(
            "INSERT INTO language_store (id, state, updated_at) \
             VALUES (1, $1, NOW()) \
             ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()"
        ).bind(&json).execute(pool).await;
    }
}

pub async fn restore() {
    if let Some(pool) = crate::memory::pool() {
        let row: Option<(serde_json::Value,)> = sqlx::query_as(
            "SELECT state FROM language_store WHERE id = 1"
        ).fetch_optional(pool).await.unwrap_or(None);
        if let Some((json,)) = row {
            if let Ok(restored) = serde_json::from_value::<LanguageState>(json) {
                let mut state = LANG.write().await;
                *state = restored;
                tracing::info!(
                    glossary = state.glossary.len(),
                    "Language engine restored"
                );
            }
        }
    }
}

pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS language_store (\
                id INTEGER PRIMARY KEY, \
                state JSONB NOT NULL, \
                updated_at TIMESTAMPTZ DEFAULT NOW()\
            )"
        ).execute(pool).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_state() {
        let state = LanguageState::default();
        assert!(!state.glossary.is_empty());
        assert!(!state.templates.is_empty());
    }

    #[test]
    fn test_glossary_term() {
        let g = &LanguageState::default().glossary[0];
        assert_eq!(g.preferred_form, "RedNode");
    }

    #[test]
    fn test_extraction_serialization() {
        let e = Extraction {
            entities: vec![ExtractedEntity { name: "NixOS".into(), entity_type: "technology".into(), confidence: 0.9 }],
            facts: vec!["NixOS is a Linux distribution".into()],
            actions: vec![],
            sentiment: None,
        };
        let json = serde_json::to_string(&e).expect("serialize extraction");
        assert!(json.contains("NixOS"));
    }
}
