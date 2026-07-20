// RedNode-OS — Episodic Memory
//
// Separates memory into distinct human-like types:
//   - Semantic: facts ("TrueNAS runs on 10.0.50.20")
//   - Procedural: skills ("how to restart a Docker container")
//   - Episodic: experiences ("last Tuesday's outage and recovery")
//   - Working: current task context (volatile, cleared between tasks)
//   - Long-term: consolidated patterns and wisdom
//   - World: spatial/topological knowledge (handled by world_model.rs)
//
// Episodic memory stores EXPERIENCES — time-stamped narratives of
// what happened, what was tried, what worked, and what was felt.
// This is fundamentally different from factual memory (semantic) or
// skill memory (procedural).
//
// Benefits of separation:
//   - "Remember what happened last time X broke?" → episodic
//   - "What is the IP of the NAS?" → semantic
//   - "How do I check SMART status?" → procedural
//   - "What am I currently working on?" → working
//
// Integration:
//   - Consciousness creates episodes from task completions
//   - Reflection consolidates episodes into learnings
//   - Distillation promotes recurring episodes to procedures
//   - Planner queries relevant episodes for context
//
// API:
//   GET  /memory/episodic         — query episodes
//   GET  /memory/procedural       — query procedures
//   GET  /memory/working          — current working memory
//   GET  /memory/semantic/stats   — semantic memory statistics
//   POST /memory/episode          — record an episode manually

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

static EPISODIC: once_cell::sync::Lazy<Arc<RwLock<StructuredMemory>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(StructuredMemory::default())));

/// An episode — a time-stamped experience narrative
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    pub id: String,
    pub title: String,
    pub narrative: String,
    pub category: EpisodeCategory,
    pub participants: Vec<String>,
    pub actions_taken: Vec<String>,
    pub outcome: EpisodeOutcome,
    pub emotion: EmotionalContext,
    pub lessons: Vec<String>,
    pub related_episodes: Vec<String>,
    pub started_at: DateTime<Utc>,
    pub ended_at: DateTime<Utc>,
    pub importance: f32,
    pub recall_count: u64,
    pub last_recalled: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EpisodeCategory {
    Incident,
    Maintenance,
    Discovery,
    Interaction,
    Achievement,
    Failure,
    Routine,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum EpisodeOutcome {
    Success,
    PartialSuccess,
    Failure,
    Ongoing,
    Unknown,
}

/// Operational state indicators during the episode
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalContext {
    pub confidence: f32,
    pub urgency: f32,
    pub curiosity: f32,
    pub satisfaction: f32,
}

/// A procedural memory — how to do something
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Procedure {
    pub id: String,
    pub name: String,
    pub description: String,
    pub steps: Vec<ProcedureStep>,
    pub tools_required: Vec<String>,
    pub success_rate: f32,
    pub times_executed: u64,
    pub last_executed: Option<DateTime<Utc>>,
    pub source_episodes: Vec<String>,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcedureStep {
    pub order: u32,
    pub description: String,
    pub tool: Option<String>,
    pub expected_outcome: String,
}

/// Working memory — volatile context for current operations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkingMemory {
    pub current_intent: Option<String>,
    pub context: Vec<ContextItem>,
    pub scratchpad: Vec<String>,
    pub active_episode_id: Option<String>,
    pub last_cleared: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextItem {
    pub key: String,
    pub value: serde_json::Value,
    pub source: String,
    pub added_at: DateTime<Utc>,
    pub ttl_secs: Option<u64>,
}

/// A detected contradiction between memories
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contradiction {
    pub episode_a_id: String,
    pub episode_b_id: String,
    pub description: String,
    pub detected_at: DateTime<Utc>,
}

/// The complete structured memory system
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StructuredMemory {
    pub episodes: VecDeque<Episode>,
    pub procedures: Vec<Procedure>,
    pub working: WorkingMemory,
    pub semantic_count: u64,
    pub total_episodes: u64,
    pub total_recalls: u64,
    pub contradictions: Vec<Contradiction>,
    /// Memory entropy: 0.0 = highly redundant, 1.0 = maximally diverse
    pub entropy: f32,
}

impl Default for StructuredMemory {
    fn default() -> Self {
        Self {
            episodes: VecDeque::with_capacity(500),
            procedures: Vec::new(),
            working: WorkingMemory {
                current_intent: None,
                context: Vec::new(),
                scratchpad: Vec::new(),
                active_episode_id: None,
                last_cleared: Utc::now(),
            },
            semantic_count: 0,
            total_episodes: 0,
            total_recalls: 0,
            contradictions: Vec::new(),
            entropy: 0.0,
        }
    }
}

fn gen_id(prefix: &str) -> String {
    format!("{}_{}", prefix, chrono::Utc::now().timestamp_millis())
}

// ─── Public API: Episodic ───

/// Record a new episode
pub async fn record_episode(
    title: &str, narrative: &str, category: EpisodeCategory,
    participants: Vec<String>, actions: Vec<String>,
    outcome: EpisodeOutcome, lessons: Vec<String>, importance: f32,
) -> Episode {
    let mut mem = EPISODIC.write().await;
    let now = Utc::now();
    let mind = crate::consciousness::get_mind().await;

    let episode = Episode {
        id: gen_id("ep"),
        title: title.into(),
        narrative: narrative.into(),
        category,
        participants,
        actions_taken: actions,
        outcome,
        emotion: EmotionalContext {
            confidence: mind.awareness.confidence,
            urgency: mind.awareness.urgency,
            curiosity: mind.awareness.curiosity,
            satisfaction: if importance > 0.7 { 0.8 } else { 0.5 },
        },
        lessons,
        related_episodes: Vec::new(),
        started_at: now,
        ended_at: now,
        importance: importance.clamp(0.0, 1.0),
        recall_count: 0,
        last_recalled: None,
    };

    if mem.episodes.len() >= 500 { mem.episodes.pop_front(); }
    mem.episodes.push_back(episode.clone());
    mem.total_episodes += 1;

    let episode_id = episode.id.clone();
    drop(mem);

    // Auto-index the episode with goal tags and capability domains
    auto_index_episode(&episode_id).await;

    // Emit to cognitive bus
    crate::cognitive_bus::emit(
        crate::cognitive_bus::CognitiveEventType::EpisodeRecorded,
        "episodic_memory",
        serde_json::json!({"id": episode_id, "title": title, "category": format!("{:?}", episode.category)}),
    ).await;

    tracing::info!(title = title, category = ?episode.category, "Episode recorded");
    episode
}

/// Search episodes by keyword
pub async fn search_episodes(query: &str, limit: usize) -> Vec<Episode> {
    let mut mem = EPISODIC.write().await;
    let q = query.to_lowercase();

    let mut results: Vec<Episode> = mem.episodes.iter()
        .filter(|e| {
            e.title.to_lowercase().contains(&q)
            || e.narrative.to_lowercase().contains(&q)
            || e.lessons.iter().any(|l| l.to_lowercase().contains(&q))
        })
        .cloned()
        .collect();

    // Update recall counts
    for result in &results {
        if let Some(ep) = mem.episodes.iter_mut().find(|e| e.id == result.id) {
            ep.recall_count += 1;
            ep.last_recalled = Some(Utc::now());
        }
    }
    mem.total_recalls += results.len() as u64;

    results.sort_by(|a, b| b.importance.partial_cmp(&a.importance).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(limit);
    results
}

/// Get recent episodes
pub async fn get_recent_episodes(limit: usize) -> Vec<Episode> {
    EPISODIC.read().await.episodes.iter().rev().take(limit).cloned().collect()
}

// ─── Public API: Procedural ───

/// Record or update a procedure
pub async fn upsert_procedure(
    name: &str, description: &str, steps: Vec<ProcedureStep>,
    tools: Vec<String>, source_episodes: Vec<String>,
) -> Procedure {
    let mut mem = EPISODIC.write().await;

    if let Some(existing) = mem.procedures.iter_mut().find(|p| p.name == name) {
        existing.steps = steps;
        existing.description = description.into();
        existing.source_episodes.extend(source_episodes);
        return existing.clone();
    }

    let proc = Procedure {
        id: gen_id("proc"),
        name: name.into(),
        description: description.into(),
        steps,
        tools_required: tools,
        success_rate: 0.5,
        times_executed: 0,
        last_executed: None,
        source_episodes,
        verified: false,
    };
    mem.procedures.push(proc.clone());
    proc
}

/// Search procedures by keyword
pub async fn search_procedures(query: &str) -> Vec<Procedure> {
    let mem = EPISODIC.read().await;
    let q = query.to_lowercase();
    mem.procedures.iter()
        .filter(|p| p.name.to_lowercase().contains(&q) || p.description.to_lowercase().contains(&q))
        .cloned()
        .collect()
}

pub async fn get_procedures() -> Vec<Procedure> {
    EPISODIC.read().await.procedures.clone()
}

/// Record that a procedure was executed
pub async fn procedure_executed(name: &str, success: bool) {
    let mut mem = EPISODIC.write().await;
    if let Some(proc) = mem.procedures.iter_mut().find(|p| p.name == name) {
        proc.times_executed += 1;
        proc.last_executed = Some(Utc::now());
        let total = proc.times_executed as f32;
        let successes = proc.success_rate * (total - 1.0) + if success { 1.0 } else { 0.0 };
        proc.success_rate = successes / total;
    }
}

// ─── Public API: Working Memory ───

/// Set the current intent in working memory
pub async fn set_working_intent(intent: &str) {
    let mut mem = EPISODIC.write().await;
    mem.working.current_intent = Some(intent.into());
}

/// Add context to working memory
pub async fn add_context(key: &str, value: serde_json::Value, source: &str, ttl_secs: Option<u64>) {
    let mut mem = EPISODIC.write().await;
    mem.working.context.push(ContextItem {
        key: key.into(), value, source: source.into(),
        added_at: Utc::now(), ttl_secs,
    });
}

/// Get working memory
pub async fn get_working() -> WorkingMemory {
    EPISODIC.read().await.working.clone()
}

/// Clear working memory
pub async fn clear_working() {
    let mut mem = EPISODIC.write().await;
    mem.working = WorkingMemory {
        current_intent: None,
        context: Vec::new(),
        scratchpad: Vec::new(),
        active_episode_id: None,
        last_cleared: Utc::now(),
    };
}

/// Detect contradictions: episodes with similar titles/categories but opposite outcomes
pub async fn detect_contradictions() -> Vec<Contradiction> {
    let mut mem = EPISODIC.write().await;
    let mut new_contradictions = Vec::new();

    let episodes: Vec<_> = mem.episodes.iter().cloned().collect();
    for i in 0..episodes.len() {
        for j in (i + 1)..episodes.len() {
            let a = &episodes[i];
            let b = &episodes[j];
            // Same category, same participants, but different outcomes
            if a.category == b.category
                && a.outcome != b.outcome
                && a.outcome != EpisodeOutcome::Ongoing
                && b.outcome != EpisodeOutcome::Ongoing
                && a.outcome != EpisodeOutcome::Unknown
                && b.outcome != EpisodeOutcome::Unknown
            {
                // Check if titles are similar (share 2+ words)
                let a_words: std::collections::HashSet<_> = a.title.to_lowercase().split_whitespace().collect();
                let b_words: std::collections::HashSet<_> = b.title.to_lowercase().split_whitespace().collect();
                let overlap = a_words.intersection(&b_words).count();
                if overlap >= 2 {
                    let already_found = mem.contradictions.iter()
                        .any(|c| (c.episode_a_id == a.id && c.episode_b_id == b.id)
                            || (c.episode_a_id == b.id && c.episode_b_id == a.id));
                    if !already_found {
                        let c = Contradiction {
                            episode_a_id: a.id.clone(),
                            episode_b_id: b.id.clone(),
                            description: format!(
                                "'{}' ({:?}) vs '{}' ({:?})",
                                a.title, a.outcome, b.title, b.outcome
                            ),
                            detected_at: Utc::now(),
                        };
                        new_contradictions.push(c);
                    }
                }
            }
        }
    }

    mem.contradictions.extend(new_contradictions.clone());
    // Keep last 50 contradictions
    if mem.contradictions.len() > 50 {
        mem.contradictions = mem.contradictions.split_off(mem.contradictions.len() - 50);
    }

    new_contradictions
}

/// Compute memory entropy — diversity of episode categories
/// 0.0 = all episodes are the same category, 1.0 = maximally diverse
pub async fn compute_entropy() -> f32 {
    let mut mem = EPISODIC.write().await;
    if mem.episodes.is_empty() {
        mem.entropy = 0.0;
        return 0.0;
    }

    let mut category_counts: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for ep in &mem.episodes {
        let key = format!("{:?}", ep.category);
        *category_counts.entry(key).or_insert(0) += 1;
    }

    let total = mem.episodes.len() as f32;
    let mut entropy = 0.0f32;
    for count in category_counts.values() {
        let p = *count as f32 / total;
        if p > 0.0 {
            entropy -= p * p.ln();
        }
    }
    // Normalize by max possible entropy (ln of number of categories)
    let max_entropy = (category_counts.len() as f32).ln().max(1.0);
    let normalized = (entropy / max_entropy).clamp(0.0, 1.0);

    mem.entropy = normalized;
    normalized
}

/// Auto-index an episode — add tags based on goal keywords and capability domains
pub async fn auto_index_episode(episode_id: &str) {
    let goals = crate::goals::active().await;
    let identity = crate::identity::get().await;

    let mut mem = EPISODIC.write().await;
    if let Some(ep) = mem.episodes.iter_mut().find(|e| e.id == episode_id) {
        let narrative_lower = ep.narrative.to_lowercase();

        // Tag with matching goal tags
        for goal in &goals {
            for tag in &goal.tags {
                if narrative_lower.contains(&tag.to_lowercase()) && !ep.lessons.contains(tag) {
                    ep.lessons.push(format!("[auto:goal:{}] {}", goal.title, tag));
                }
            }
        }

        // Tag with matching capability domains
        for cap in &identity.capabilities {
            if narrative_lower.contains(&cap.domain.to_lowercase()) {
                let tag = format!("[auto:capability:{}]", cap.name);
                if !ep.lessons.contains(&tag) {
                    ep.lessons.push(tag);
                }
            }
        }
    }
}

/// Get detected contradictions
pub async fn get_contradictions() -> Vec<Contradiction> {
    EPISODIC.read().await.contradictions.clone()
}

/// Clean expired context items
pub async fn tick() {
    let mut mem = EPISODIC.write().await;
    let now = Utc::now();
    mem.working.context.retain(|c| {
        match c.ttl_secs {
            Some(ttl) => (now - c.added_at).num_seconds() < ttl as i64,
            None => true,
        }
    });
}

pub async fn get_stats() -> serde_json::Value {
    let mem = EPISODIC.read().await;
    serde_json::json!({
        "episodes": mem.episodes.len(),
        "procedures": mem.procedures.len(),
        "working_context_items": mem.working.context.len(),
        "total_episodes": mem.total_episodes,
        "total_recalls": mem.total_recalls,
        "contradictions": mem.contradictions.len(),
        "entropy": mem.entropy,
    })
}

// ─── Persistence ───

pub async fn persist() {
    let mem = EPISODIC.read().await.clone();
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(&mem) { Ok(v) => v, Err(_) => return };
        let _ = sqlx::query(
            "INSERT INTO episodic_memory_store (id, state, updated_at) \
             VALUES (1, $1, NOW()) \
             ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()"
        ).bind(&json).execute(pool).await;
    }
}

pub async fn restore() {
    if let Some(pool) = crate::memory::pool() {
        let row: Option<(serde_json::Value,)> = sqlx::query_as(
            "SELECT state FROM episodic_memory_store WHERE id = 1"
        ).fetch_optional(pool).await.unwrap_or(None);
        if let Some((json,)) = row {
            if let Ok(restored) = serde_json::from_value::<StructuredMemory>(json) {
                let mut mem = EPISODIC.write().await;
                *mem = restored;
                tracing::info!(episodes = mem.episodes.len(), procedures = mem.procedures.len(), "Episodic memory restored");
            }
        }
    }
}

pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS episodic_memory_store (\
                id INTEGER PRIMARY KEY, state JSONB NOT NULL, updated_at TIMESTAMPTZ DEFAULT NOW())"
        ).execute(pool).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_memory() {
        let mem = StructuredMemory::default();
        assert!(mem.episodes.is_empty());
        assert!(mem.procedures.is_empty());
        assert!(mem.working.current_intent.is_none());
    }

    #[test]
    fn test_episode_serialization() {
        let ep = Episode {
            id: "test".into(), title: "Test Outage".into(), narrative: "Server went down".into(),
            category: EpisodeCategory::Incident, participants: vec!["system-agent".into()],
            actions_taken: vec!["restarted service".into()], outcome: EpisodeOutcome::Success,
            emotion: EmotionalContext { confidence: 0.5, urgency: 0.8, curiosity: 0.1, satisfaction: 0.7 },
            lessons: vec!["Always check logs first".into()], related_episodes: vec![],
            started_at: Utc::now(), ended_at: Utc::now(), importance: 0.8,
            recall_count: 0, last_recalled: None,
        };
        let json = serde_json::to_string(&ep).unwrap();
        assert!(json.contains("Test Outage"));
    }
}
