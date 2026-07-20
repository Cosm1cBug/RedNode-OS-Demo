// RedNode-OS — Knowledge Distillation
//
// Compresses accumulated experience into reusable "wisdom":
//   - Coding best practices (from code reviews and learnings)
//   - Incident response playbooks (from security events + repairs)
//   - Infrastructure runbooks (from successful maintenance)
//   - Personal preferences (from interaction patterns)
//   - System configuration guides (from config changes)
//
// Distilled knowledge is structured Markdown stored in the knowledge base,
// queryable by intent ("What's our runbook for TrueNAS disk replacement?").
//
// The distillation process:
//   1. Gather raw material (reflections, audit logs, learnings, memories)
//   2. Identify patterns and recurring procedures
//   3. Synthesize into structured documents
//   4. Score by usefulness (how often referenced, user feedback)
//   5. Version and update (documents evolve with new experience)
//
// Integration:
//   - Reflection system provides raw learnings
//   - Memory provides historical context
//   - Research agent produces new knowledge
//   - Curiosity engine feeds discoveries
//
// API:
//   GET  /distillation               — list all distilled documents
//   GET  /distillation/:id           — get a document
//   POST /distillation/trigger       — trigger distillation now
//   GET  /distillation/search        — search distilled knowledge

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;

static DISTILLED: once_cell::sync::Lazy<Arc<RwLock<DistillationStore>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(DistillationStore::default())));

/// A distilled knowledge document
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistilledDocument {
    pub id: String,
    pub title: String,
    pub category: DocumentCategory,
    pub content: String,
    pub tags: Vec<String>,
    pub sources: Vec<String>,
    pub version: u32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub usefulness_score: f32,
    pub reference_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum DocumentCategory {
    Runbook,
    BestPractice,
    IncidentPlaybook,
    ConfigGuide,
    Preference,
    TroubleshootingGuide,
    Architecture,
    Textbook,
    DecisionTree,
    TrainingMaterial,
    Custom(String),
}

/// Raw material gathered for distillation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistillationInput {
    pub source_type: String,
    pub content: String,
    pub context: String,
    pub timestamp: DateTime<Utc>,
}

/// The distillation store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DistillationStore {
    pub documents: Vec<DistilledDocument>,
    pub pending_inputs: VecDeque<DistillationInput>,
    pub last_distillation: Option<DateTime<Utc>>,
    pub total_distillations: u64,
}

impl Default for DistillationStore {
    fn default() -> Self {
        Self {
            documents: Vec::new(),
            pending_inputs: VecDeque::with_capacity(100),
            last_distillation: None,
            total_distillations: 0,
        }
    }
}

fn gen_id() -> String {
    format!("doc_{}", chrono::Utc::now().timestamp_millis())
}

// ─── Public API ───

/// List all distilled documents
pub async fn list_documents() -> Vec<DistilledDocument> {
    DISTILLED.read().await.documents.clone()
}

/// Get a specific document by ID
pub async fn get_document(id: &str) -> Option<DistilledDocument> {
    let mut store = DISTILLED.write().await;
    if let Some(doc) = store.documents.iter_mut().find(|d| d.id == id) {
        doc.reference_count += 1;
        return Some(doc.clone());
    }
    None
}

/// Search documents by keyword
pub async fn search(query: &str) -> Vec<DistilledDocument> {
    let store = DISTILLED.read().await;
    let query_lower = query.to_lowercase();

    store.documents.iter()
        .filter(|d| {
            d.title.to_lowercase().contains(&query_lower)
            || d.content.to_lowercase().contains(&query_lower)
            || d.tags.iter().any(|t| t.to_lowercase().contains(&query_lower))
        })
        .cloned()
        .collect()
}

/// Add raw material for future distillation
pub async fn add_input(source_type: &str, content: &str, context: &str) {
    let mut store = DISTILLED.write().await;
    if store.pending_inputs.len() >= 100 {
        store.pending_inputs.pop_front();
    }
    store.pending_inputs.push_back(DistillationInput {
        source_type: source_type.into(),
        content: content.into(),
        context: context.into(),
        timestamp: Utc::now(),
    });
}

/// Create or update a distilled document manually
pub async fn upsert_document(
    id: Option<&str>,
    title: &str,
    category: DocumentCategory,
    content: &str,
    tags: Vec<String>,
    sources: Vec<String>,
) -> DistilledDocument {
    let mut store = DISTILLED.write().await;

    if let Some(existing_id) = id {
        if let Some(doc) = store.documents.iter_mut().find(|d| d.id == existing_id) {
            doc.title = title.into();
            doc.category = category;
            doc.content = content.into();
            doc.tags = tags;
            doc.sources.extend(sources);
            doc.version += 1;
            doc.updated_at = Utc::now();
            return doc.clone();
        }
    }

    let doc = DistilledDocument {
        id: gen_id(),
        title: title.into(),
        category,
        content: content.into(),
        tags,
        sources,
        version: 1,
        created_at: Utc::now(),
        updated_at: Utc::now(),
        usefulness_score: 0.5,
        reference_count: 0,
    };

    store.documents.push(doc.clone());
    tracing::info!(title = title, "Knowledge document distilled");

    crate::events::emit(serde_json::json!({
        "type": "knowledge_distilled",
        "title": title,
        "ts": Utc::now().to_rfc3339(),
    }));

    doc
}

/// Run the distillation process — synthesize pending inputs into documents.
/// Uses rule-based extraction (LLM-assisted distillation is called from
/// the coordinator when the LLM is available).
pub async fn distill() -> Vec<DistilledDocument> {
    let mut store = DISTILLED.write().await;
    let mut new_docs = Vec::new();

    if store.pending_inputs.is_empty() {
        return new_docs;
    }

    // Group inputs by source type
    let mut by_type: std::collections::HashMap<String, Vec<DistillationInput>> =
        std::collections::HashMap::new();
    for input in store.pending_inputs.drain(..) {
        by_type.entry(input.source_type.clone()).or_default().push(input);
    }

    // Distill each group
    for (source_type, inputs) in by_type {
        if inputs.len() < 3 {
            // Not enough material yet — re-queue
            for input in inputs {
                store.pending_inputs.push_back(input);
            }
            continue;
        }

        let category = match source_type.as_str() {
            "security_event" | "security" => DocumentCategory::IncidentPlaybook,
            "reflection" | "daily_summary" => DocumentCategory::BestPractice,
            "maintenance" | "repair" => DocumentCategory::Runbook,
            "config" | "config_change" => DocumentCategory::ConfigGuide,
            "failure" | "troubleshooting" => DocumentCategory::TroubleshootingGuide,
            _ => DocumentCategory::Custom(source_type.clone()),
        };

        // Build a synthesized document from inputs
        let mut content_parts = Vec::new();
        let mut tags = std::collections::HashSet::new();
        let mut sources_set = std::collections::HashSet::new();

        content_parts.push(format!("# {} Knowledge\n", source_type));
        content_parts.push(format!("*Distilled from {} observations*\n", inputs.len()));

        for (i, input) in inputs.iter().enumerate() {
            content_parts.push(format!("## Observation {}\n", i + 1));
            content_parts.push(format!("**Context:** {}\n", input.context));
            content_parts.push(format!("{}\n", input.content));

            // Extract simple tags from content
            for word in input.content.split_whitespace() {
                let clean = word.trim_matches(|c: char| !c.is_alphanumeric()).to_lowercase();
                if clean.len() > 3 && clean.len() < 20 {
                    tags.insert(clean);
                }
            }
            sources_set.insert(input.source_type.clone());
        }

        let title = format!("{} — Distilled Knowledge", source_type);
        let content = content_parts.join("\n");

        let doc = DistilledDocument {
            id: gen_id(),
            title: title.clone(),
            category,
            content,
            tags: tags.into_iter().take(10).collect(),
            sources: sources_set.into_iter().collect(),
            version: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            usefulness_score: 0.5,
            reference_count: 0,
        };

        store.documents.push(doc.clone());
        new_docs.push(doc);

        tracing::info!(title = %title, "New knowledge document distilled");
    }

    store.last_distillation = Some(Utc::now());
    store.total_distillations += 1;

    new_docs
}

/// Background tick — check if distillation should run
pub async fn tick() {
    let store = DISTILLED.read().await;

    // Distill if we have 10+ pending inputs and haven't distilled in 24h
    let should_distill = store.pending_inputs.len() >= 10
        && store.last_distillation.map_or(true, |last| {
            (Utc::now() - last).num_hours() >= 24
        });

    drop(store);

    if should_distill {
        let new_docs = distill().await;
        if !new_docs.is_empty() {
            crate::cognitive_bus::emit(
                crate::cognitive_bus::CognitiveEventType::MemoryStored,
                "distillation",
                serde_json::json!({"action": "distilled", "count": new_docs.len()}),
            ).await;
        }
    }
}

/// Get stats for API
pub async fn get_stats() -> serde_json::Value {
    let store = DISTILLED.read().await;
    serde_json::json!({
        "document_count": store.documents.len(),
        "pending_inputs": store.pending_inputs.len(),
        "total_distillations": store.total_distillations,
        "last_distillation": store.last_distillation,
    })
}

// ─── Persistence ───

pub async fn persist() {
    let store = DISTILLED.read().await.clone();
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(&store) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Failed to serialize distillation store: {}", e);
                return;
            }
        };

        let _ = sqlx::query(
            "INSERT INTO distillation_store (id, state, updated_at) \
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
            "SELECT state FROM distillation_store WHERE id = 1"
        )
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        if let Some((json,)) = row {
            if let Ok(restored) = serde_json::from_value::<DistillationStore>(json) {
                let mut store = DISTILLED.write().await;
                *store = restored;
                tracing::info!(
                    documents = store.documents.len(),
                    pending = store.pending_inputs.len(),
                    "Knowledge distillation restored"
                );
            }
        }
    }
}

pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS distillation_store (\
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
    fn test_default_store() {
        let store = DistillationStore::default();
        assert!(store.documents.is_empty());
        assert!(store.pending_inputs.is_empty());
        assert_eq!(store.total_distillations, 0);
    }

    #[test]
    fn test_document_serialization() {
        let doc = DistilledDocument {
            id: "test_1".into(),
            title: "TrueNAS Disk Replacement".into(),
            category: DocumentCategory::Runbook,
            content: "Step 1: Identify failed disk...".into(),
            tags: vec!["truenas".into(), "disk".into()],
            sources: vec!["maintenance".into()],
            version: 1,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            usefulness_score: 0.8,
            reference_count: 5,
        };
        let json = serde_json::to_string(&doc).unwrap();
        assert!(json.contains("TrueNAS"));
        assert!(json.contains("Runbook"));
    }

    #[test]
    fn test_category_equality() {
        assert_eq!(DocumentCategory::Runbook, DocumentCategory::Runbook);
        assert_ne!(DocumentCategory::Runbook, DocumentCategory::BestPractice);
    }
}
