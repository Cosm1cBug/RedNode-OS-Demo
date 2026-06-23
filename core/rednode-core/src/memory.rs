use anyhow::Result;
use sqlx::PgPool;
use std::sync::OnceLock;
static POOL: OnceLock<Option<PgPool>> = OnceLock::new();

pub async fn init() -> Result<()> {
    let url = std::env::var("DATABASE_URL").unwrap_or("postgres://rednode:rednode@127.0.0.1:5432/rednode".into());
    match PgPool::connect(&url).await {
        Ok(pool) => {
            tracing::info!("Postgres connected");
            // Run migrations if table missing
            let _ = sqlx::query(
                r#"CREATE TABLE IF NOT EXISTS audit_log (
                    id BIGSERIAL PRIMARY KEY,
                    ts TIMESTAMPTZ DEFAULT now(),
                    actor TEXT NOT NULL,
                    action TEXT NOT NULL,
                    tool TEXT,
                    args JSONB,
                    risk TEXT,
                    approved BOOLEAN,
                    result TEXT,
                    prev_hash TEXT,
                    hash TEXT
                )"#
            ).execute(&pool).await;
            let _ = sqlx::query(
                r#"CREATE TABLE IF NOT EXISTS security_events (
                    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                    ts TIMESTAMPTZ DEFAULT now(),
                    severity TEXT NOT NULL,
                    source TEXT NOT NULL,
                    summary TEXT NOT NULL,
                    raw JSONB,
                    acknowledged BOOLEAN DEFAULT false
                )"#
            ).execute(&pool).await;
            let _ = sqlx::query(
                r#"CREATE TABLE IF NOT EXISTS approvals (
                    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
                    ts TIMESTAMPTZ DEFAULT now(),
                    actor TEXT NOT NULL,
                    tool TEXT NOT NULL,
                    args JSONB,
                    risk TEXT NOT NULL,
                    status TEXT NOT NULL DEFAULT 'pending',
                    intent TEXT,
                    session_id TEXT
                )"#
            ).execute(&pool).await;
            POOL.set(Some(pool)).ok();
            Ok(())
        },
        Err(e) => {
            tracing::warn!("Postgres unavailable (running without DB): {}", e);
            POOL.set(None).ok();
            Ok(())
        }
    }
}

pub fn pool() -> Option<&'static PgPool> {
    POOL.get().and_then(|o| o.as_ref())
}

use sha2::{Sha256, Digest};
pub async fn audit_log(
    actor: &str,
    action: &str,
    tool: Option<&str>,
    args: &serde_json::Value,
    risk: &str,
    approved: bool,
    result: &str,
) -> Result<i64> {
    let Some(pool) = pool() else {
        tracing::warn!(actor, tool, "audit_log skipped – no DB");
        return Ok(0);
    };
    // get prev_hash
    let prev: Option<String> = sqlx::query_scalar("SELECT hash FROM audit_log ORDER BY id DESC LIMIT 1")
        .fetch_optional(pool).await?;
    let prev_hash = prev.unwrap_or_else(|| "genesis".into());
    // compute hash
    let mut hasher = Sha256::new();
    hasher.update(prev_hash.as_bytes());
    hasher.update(actor.as_bytes());
    hasher.update(action.as_bytes());
    hasher.update(tool.unwrap_or("").as_bytes());
    hasher.update(args.to_string().as_bytes());
    hasher.update(risk.as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    
    let rec: (i64,) = sqlx::query_as(
        "INSERT INTO audit_log (actor, action, tool, args, risk, approved, result, prev_hash, hash)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9) RETURNING id"
    )
    .bind(actor)
    .bind(action)
    .bind(tool)
    .bind(args)
    .bind(risk)
    .bind(approved)
    .bind(result)
    .bind(prev_hash)
    .bind(hash)
    .fetch_one(pool).await?;
    Ok(rec.0)
}

#[derive(sqlx::FromRow, serde::Serialize)]
pub struct AuditEntry {
    pub id: i64,
    pub ts: chrono::DateTime<chrono::Utc>,
    pub actor: String,
    pub action: String,
    pub tool: Option<String>,
    pub args: Option<serde_json::Value>,
    pub risk: Option<String>,
    pub approved: Option<bool>,
    pub result: Option<String>,
    pub prev_hash: Option<String>,
    pub hash: Option<String>,
}

pub async fn get_audit(limit: i64) -> Result<Vec<AuditEntry>> {
    let Some(pool) = pool() else { return Ok(vec![]) };
    let rows = sqlx::query_as::<_, AuditEntry>(
        "SELECT id, ts, actor, action, tool, args, risk, approved, result, prev_hash, hash FROM audit_log ORDER BY id DESC LIMIT $1"
    ).bind(limit).fetch_all(pool).await?;
    Ok(rows)
}

#[derive(sqlx::FromRow, serde::Serialize)]
pub struct ApprovalRow {
    pub id: uuid::Uuid,
    pub ts: chrono::DateTime<chrono::Utc>,
    pub actor: String,
    pub tool: String,
    pub args: Option<serde_json::Value>,
    pub risk: String,
    pub status: String,
    pub intent: Option<String>,
    pub session_id: Option<String>,
}

pub async fn create_approval(actor: &str, tool: &str, args: &serde_json::Value, risk: &str, intent: Option<&str>, session_id: Option<&str>) -> Result<uuid::Uuid> {
    let Some(pool) = pool() else { anyhow::bail!("no db") };
    let rec: (uuid::Uuid,) = sqlx::query_as(
        "INSERT INTO approvals (actor, tool, args, risk, status, intent, session_id) VALUES ($1,$2,$3,$4,'pending',$5,$6) RETURNING id"
    )
    .bind(actor).bind(tool).bind(args).bind(risk).bind(intent).bind(session_id)
    .fetch_one(pool).await?;
    Ok(rec.0)
}

pub async fn list_approvals(status: &str) -> Result<Vec<ApprovalRow>> {
    let Some(pool) = pool() else { return Ok(vec![]) };
    let rows = sqlx::query_as::<_, ApprovalRow>(
        "SELECT id, ts, actor, tool, args, risk, status, intent, session_id FROM approvals WHERE status = $1 ORDER BY ts DESC LIMIT 100"
    ).bind(status).fetch_all(pool).await?;
    Ok(rows)
}

pub async fn approve_id(id: uuid::Uuid, approved: bool) -> Result<bool> {
    let Some(pool) = pool() else { anyhow::bail!("no db") };
    let res = sqlx::query("UPDATE approvals SET status = $2 WHERE id = $1")
        .bind(id).bind(if approved {"approved"} else {"denied"})
        .execute(pool).await?;
    Ok(res.rows_affected() > 0)
}

pub async fn get_approval(id: uuid::Uuid) -> Result<Option<ApprovalRow>> {
    let Some(pool) = pool() else { return Ok(None) };
    let row = sqlx::query_as::<_, ApprovalRow>(
        "SELECT id, ts, actor, tool, args, risk, status, intent, session_id FROM approvals WHERE id = $1"
    ).bind(id).fetch_optional(pool).await?;
    Ok(row)
}

#[derive(sqlx::FromRow, serde::Serialize)]
pub struct SecurityEvent {
    pub id: uuid::Uuid,
    pub ts: chrono::DateTime<chrono::Utc>,
    pub severity: String,
    pub source: String,
    pub summary: String,
    pub raw: Option<serde_json::Value>,
    pub acknowledged: Option<bool>,
}

pub async fn log_security_event(severity: &str, source: &str, summary: &str, raw: serde_json::Value) -> Result<uuid::Uuid> {
    if let Some(pool) = pool() {
        let rec: (uuid::Uuid,) = sqlx::query_as(
            "INSERT INTO security_events (severity, source, summary, raw) VALUES ($1,$2,$3,$4) RETURNING id"
        ).bind(severity).bind(source).bind(summary).bind(raw)
        .fetch_one(pool).await?;
        return Ok(rec.0);
    }
    Ok(uuid::Uuid::new_v4())
}

pub async fn list_security_events(limit: i64) -> Result<Vec<SecurityEvent>> {
    let Some(pool) = pool() else { return Ok(vec![]) };
    let rows = sqlx::query_as::<_, SecurityEvent>(
        "SELECT id, ts, severity, source, summary, raw, acknowledged FROM security_events ORDER BY ts DESC LIMIT $1"
    ).bind(limit).fetch_all(pool).await?;
    Ok(rows)
}

pub async fn ack_security_event(id: uuid::Uuid) -> Result<bool> {
    let Some(pool) = pool() else { return Ok(false) };
    let res = sqlx::query("UPDATE security_events SET acknowledged = true WHERE id = $1")
        .bind(id).execute(pool).await?;
    Ok(res.rows_affected() > 0)
}

// ============================================================================
// Vector Memory – Qdrant + Ollama Embeddings – RAG
// ============================================================================

// OnceLock already imported at top of file
use qdrant_client::qdrant::{CreateCollection, VectorParams, Distance, SearchPoints, UpsertPoints, PointStruct};
use qdrant_client::Qdrant;

static QDRANT: OnceLock<Option<Qdrant>> = OnceLock::new();
static OLLAMA_URL: &str = "http://127.0.0.1:11434";
static EMBED_MODEL: &str = "nomic-embed-text";

async fn qdrant_client() -> Option<&'static Qdrant> {
    if QDRANT.get().is_none() {
        let url = std::env::var("QDRANT_URL").unwrap_or("http://127.0.0.1:6334".into());
        match Qdrant::from_url(&url).build() {
            Ok(client) => {
                // Ensure collection exists
                let _ = client.create_collection(CreateCollection {
                    collection_name: "rednode_docs".into(),
                    vectors_config: Some(VectorParams {
                        size: 768,
                        distance: Distance::Cosine.into(),
                        ..Default::default()
                    }.into()),
                    ..Default::default()
                }).await;
                let _ = QDRANT.set(Some(client));
            },
            Err(e) => {
                tracing::warn!("Qdrant unavailable: {} – vector search will fallback to Postgres", e);
                let _ = QDRANT.set(None);
            }
        }
    }
    QDRANT.get().and_then(|o| o.as_ref())
}

#[derive(Deserialize)]
struct OllamaEmbedResponse { embedding: Vec<f32> }

async fn embed_ollama(text: &str) -> Result<Vec<f32>> {
    #[derive(Serialize)]
    struct Req<'a> { model: &'a str, prompt: &'a str }
    let ollama = std::env::var("OLLAMA_URL").unwrap_or(OLLAMA_URL.into());
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;
    let res = client.post(format!("{}/api/embeddings", ollama))
        .json(&Req { model: EMBED_MODEL, prompt: text })
        .send().await?;
    if !res.status().is_success() {
        anyhow::bail!("ollama embed failed: {}", res.status());
    }
    let out: OllamaEmbedResponse = res.json().await?;
    Ok(out.embedding)
}

#[derive(Serialize, Clone)]
pub struct RagHit {
    pub source: String,
    pub content: String,
    pub score: f32,
    pub metadata: serde_json::Value,
}

pub async fn rag_query(query: &str, limit: u64) -> Result<Vec<RagHit>> {
    // 1. Try Qdrant vector search
    if let Some(qd) = qdrant_client().await {
        match embed_ollama(query).await {
            Ok(vec) => {
                let search = SearchPoints {
                    collection_name: "rednode_docs".into(),
                    vector: vec,
                    limit,
                    with_payload: Some(true.into()),
                    ..Default::default()
                };
                match qd.search_points(search).await {
                    Ok(resp) => {
                        let hits: Vec<RagHit> = resp.result.into_iter().map(|p| {
                            // Qdrant Value -> serde_json
                            let payload_json = serde_json::to_value(&p.payload).unwrap_or_default();
                            let content = payload_json.get("content")
                                .and_then(|v| v.get("string_value"))
                                .and_then(|v| v.as_str())
                                .or_else(|| payload_json.get("content").and_then(|v| v.as_str()))
                                .unwrap_or_default().to_string();
                            let source = payload_json.get("source")
                                .and_then(|v| v.get("string_value"))
                                .and_then(|v| v.as_str())
                                .or_else(|| payload_json.get("source").and_then(|v| v.as_str()))
                                .unwrap_or("qdrant").to_string();
                            RagHit {
                                source,
                                content,
                                score: p.score,
                                metadata: serde_json::json!({ "id": format!("{:?}", p.id) }),
                            }
                        }).collect();
                        if !hits.is_empty() {
                            return Ok(hits);
                        }
                    },
                    Err(e) => tracing::warn!("qdrant search failed: {}", e),
                }
            },
            Err(e) => tracing::warn!("ollama embed failed: {} – falling back", e),
        }
    }

    // 2. Fallback: Postgres full-text / simple LIKE
    if let Some(pool) = pool() {
        let pattern = format!("%{}%", query);
        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT COALESCE(source,'postgres') as source, content FROM documents WHERE content ILIKE $1 ORDER BY created_at DESC LIMIT $2"
        )
        .bind(pattern)
        .bind(limit as i64)
        .fetch_all(pool)
        .await
        .unwrap_or_default();
        if !rows.is_empty() {
            return Ok(rows.into_iter().map(|(source, content)| RagHit {
                source, content, score: 0.5, metadata: serde_json::json!({})
            }).collect());
        }
    }

    // 3. Final fallback – static knowledge – ensures UI never empty
    Ok(vec![
        RagHit {
            source: "memory_longterm".into(),
            content: "RedNode is a society of specialized agents – System, Security, Coding, Research, Automation, Network".into(),
            score: 0.92,
            metadata: serde_json::json!({}),
        },
        RagHit {
            source: "knowledge_graph".into(),
            content: format!("No vector DB results for '{}' – Qdrant/Ollama not running? Falling back to static knowledge. To enable real RAG: docker compose up qdrant ollama && ollama pull nomic-embed-text", query),
            score: 0.5,
            metadata: serde_json::json!({"fallback": true}),
        }
    ])
}

pub async fn ingest_document(source: &str, content: &str) -> Result<String> {
    // ── Step 0: PII Detection ──
    // Scan for sensitive data before storing anything
    let pii_result = crate::pii::scan(content);
    let store_content = if pii_result.has_pii {
        let action = crate::pii::get_pii_action();
        tracing::warn!(
            source = %source,
            pii_count = pii_result.pii_count,
            action = %action,
            "PII detected in document: {} item(s)",
            pii_result.pii_count
        );

        // Log PII findings as security event
        let finding_summary: Vec<String> = pii_result.findings.iter()
            .map(|f| format!("[{}] {}", f.severity, f.pii_type))
            .collect();
        let _ = log_security_event(
            if pii_result.findings.iter().any(|f| f.severity == "high") { "HIGH" } else { "MEDIUM" },
            "pii-scanner",
            &format!("PII detected in '{}': {}", source, finding_summary.join(", ")),
            serde_json::json!({
                "source": source,
                "pii_count": pii_result.pii_count,
                "types": finding_summary,
                "action": action,
            }),
        ).await;

        crate::events::emit_security_event(
            if pii_result.findings.iter().any(|f| f.severity == "high") { "HIGH" } else { "MEDIUM" },
            "pii-scanner",
            &format!("PII detected in '{}': {} item(s) — action: {}", source, pii_result.pii_count, action),
        );

        match action.as_str() {
            "block" => {
                tracing::warn!(source = %source, "PII action=block — document NOT ingested");
                anyhow::bail!("Document blocked: contains {} PII item(s). Set REDNODE_PII_ACTION=redact to auto-redact.", pii_result.pii_count);
            }
            "redact" => {
                tracing::info!(source = %source, "PII action=redact — storing redacted version");
                pii_result.redacted.clone()
            }
            _ => {
                // "log" — store original but log the finding
                tracing::info!(source = %source, "PII action=log — storing original with warning");
                content.to_string()
            }
        }
    } else {
        content.to_string()
    };
    let content = &store_content;

    // ── Step 1: Embed full document ──
    let embedding = match embed_ollama(content).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("embed failed, storing without vector: {}", e);
            vec![0.0; 768]
        }
    };

    // ── Step 2: Store full document in Postgres ──
    let doc_id = uuid::Uuid::new_v4();
    if let Some(pool) = pool() {
        let _ = sqlx::query(
            "INSERT INTO documents (id, source, content, embedding) VALUES ($1,$2,$3,$4::vector)"
        )
        .bind(doc_id)
        .bind(source)
        .bind(content)
        .bind(format!("[{}]", embedding.iter().map(|x| x.to_string()).collect::<Vec<_>>().join(",")))
        .execute(pool).await;
    }

    // ── Step 3: Upsert full document to Qdrant ──
    if let Some(qd) = qdrant_client().await {
        use qdrant_client::qdrant::{PointStruct, UpsertPoints, value::Kind, Value};
        use std::collections::HashMap;
        let mut payload = HashMap::new();
        payload.insert("source".to_string(), Value { kind: Some(Kind::StringValue(source.to_string())) });
        payload.insert("content".to_string(), Value { kind: Some(Kind::StringValue(content.to_string())) });
        payload.insert("type".to_string(), Value { kind: Some(Kind::StringValue("document".to_string())) });
        let point = PointStruct::new(
            doc_id.to_string(),
            embedding,
            payload,
        );
        let _ = qd.upsert_points(UpsertPoints {
            collection_name: "rednode_docs".into(),
            wait: Some(true),
            points: vec![point],
            ..Default::default()
        }).await;
    }

    // ── Step 4: Extract and embed propositions (fine-grained memory) ──
    // Propositions are atomic factual statements extracted from the document.
    // Each proposition gets its own embedding → finer-grained RAG retrieval.
    // Only for documents longer than 200 chars (short docs are already atomic).
    if content.len() > 200 {
        tokio::spawn(extract_and_embed_propositions(
            source.to_string(),
            content.to_string(),
            doc_id.to_string(),
        ));
    }

    // ── Step 5: Extract entities and build knowledge graph ──
    extract_and_store_entities(source, content);

    Ok(doc_id.to_string())
}

/// Extract propositions (atomic facts) from a document and embed each separately.
/// Runs as a background task to not block the ingest response.
async fn extract_and_embed_propositions(source: String, content: String, doc_id: String) {
    let ollama_url = std::env::var("OLLAMA_URL")
        .unwrap_or_else(|_| "http://127.0.0.1:11434".into());
    let model = std::env::var("REDNODE_MODEL")
        .unwrap_or_else(|_| "qwen2.5:14b-instruct-q4_K_M".into());

    // Ask LLM to extract propositions
    let prompt = format!(
        "Extract 3-8 atomic factual propositions from this text. \
         Each proposition should be a single, self-contained fact. \
         Output ONLY a JSON array of strings. No explanation.\n\n\
         Text: \"{}\"\n\nJSON array:",
        content.chars().take(3000).collect::<String>()
    );

    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
    {
        Ok(c) => c,
        Err(_) => return,
    };

    let resp = match client
        .post(format!("{}/api/chat", ollama_url))
        .json(&serde_json::json!({
            "model": model,
            "messages": [
                {"role": "system", "content": "Extract atomic facts as a JSON array of strings."},
                {"role": "user", "content": prompt}
            ],
            "stream": false,
            "options": {"temperature": 0.1, "num_predict": 1024}
        }))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::debug!("proposition extraction failed: {}", e);
            return;
        }
    };

    let body: serde_json::Value = match resp.json().await {
        Ok(b) => b,
        Err(_) => return,
    };

    let response_text = body["message"]["content"].as_str().unwrap_or("[]");

    // Parse propositions from LLM response
    let json_str = {
        let trimmed = response_text.trim()
            .trim_start_matches("```json").trim_start_matches("```")
            .trim_end_matches("```").trim();
        if let Some(start) = trimmed.find('[') {
            if let Some(end) = trimmed.rfind(']') {
                trimmed[start..=end].to_string()
            } else { return; }
        } else { return; }
    };

    let propositions: Vec<String> = match serde_json::from_str(&json_str) {
        Ok(p) => p,
        Err(_) => return,
    };

    if propositions.is_empty() {
        return;
    }

    tracing::info!(
        source = %source,
        count = propositions.len(),
        "extracted {} propositions from document",
        propositions.len()
    );

    // Embed and store each proposition
    for (i, prop) in propositions.iter().enumerate() {
        if prop.trim().is_empty() {
            continue;
        }

        let prop_embedding = match embed_ollama(prop).await {
            Ok(v) => v,
            Err(_) => continue,
        };

        let prop_id = uuid::Uuid::new_v4();

        // Store in Qdrant with proposition type marker
        if let Some(qd) = qdrant_client().await {
            use qdrant_client::qdrant::{PointStruct, UpsertPoints, value::Kind, Value};
            use std::collections::HashMap;
            let mut payload = HashMap::new();
            payload.insert("source".to_string(), Value { kind: Some(Kind::StringValue(format!("{}/prop-{}", source, i))) });
            payload.insert("content".to_string(), Value { kind: Some(Kind::StringValue(prop.clone())) });
            payload.insert("type".to_string(), Value { kind: Some(Kind::StringValue("proposition".to_string())) });
            payload.insert("parent_doc".to_string(), Value { kind: Some(Kind::StringValue(doc_id.clone())) });

            let point = PointStruct::new(
                prop_id.to_string(),
                prop_embedding,
                payload,
            );
            let _ = qd.upsert_points(UpsertPoints {
                collection_name: "rednode_docs".into(),
                wait: Some(true),
                points: vec![point],
                ..Default::default()
            }).await;
        }
    }
}

// ============================================================================
// Knowledge Graph – Kuzu (native) OR Postgres fallback
// ============================================================================
//
// The knowledge graph stores entities and relationships extracted from
// ingested documents. It enriches RAG queries with structured context.
//
// Two backends:
//   1. Kuzu (compile with --features kuzu) — embedded graph DB, Cypher queries
//   2. Postgres fallback (default) — JSON entities table, SQL relationship queries
//
// Entity types: Person, Project, Technology, Concept, Tool, Service, Device
// Relationship types: USES, RELATED_TO, PART_OF, DEPENDS_ON, RUNS_ON

#[cfg(feature = "kuzu")]
mod kg_kuzu {
    use super::*;
    use std::sync::Mutex;
    use once_cell::sync::OnceCell;
    static DB: OnceCell<Mutex<kuzu::Database>> = OnceCell::new();

    pub fn init(path: &str) -> Result<()> {
        let db = kuzu::Database::new(path, kuzu::SystemConfig::default())?;
        let conn = kuzu::Connection::new(&db)?;
        let schema = [
            "CREATE NODE TABLE IF NOT EXISTS Entity(name STRING, kind STRING, properties STRING, PRIMARY KEY(name))",
            "CREATE NODE TABLE IF NOT EXISTS Project(name STRING, path STRING, PRIMARY KEY(name))",
            "CREATE NODE TABLE IF NOT EXISTS Technology(name STRING, PRIMARY KEY(name))",
            "CREATE NODE TABLE IF NOT EXISTS Repo(url STRING, PRIMARY KEY(url))",
            "CREATE NODE TABLE IF NOT EXISTS File(path STRING, PRIMARY KEY(path))",
            "CREATE NODE TABLE IF NOT EXISTS Function(name STRING, file STRING, line INT64, PRIMARY KEY(name, file))",
            "CREATE REL TABLE IF NOT EXISTS USES(FROM Project TO Technology)",
            "CREATE REL TABLE IF NOT EXISTS CONTAINS(FROM Repo TO File)",
            "CREATE REL TABLE IF NOT EXISTS DEFINES(FROM File TO Function)",
            "CREATE REL TABLE IF NOT EXISTS RELATED_TO(FROM Entity TO Entity, relation STRING)",
        ];
        for s in schema { let _ = conn.query(s); }
        DB.set(Mutex::new(db)).ok();
        tracing::info!("Kuzu knowledge graph initialized at {}", path);
        Ok(())
    }

    pub fn query(cypher: &str) -> Result<Vec<serde_json::Value>> {
        let db_guard = DB.get().ok_or_else(|| anyhow::anyhow!("kuzu not initialized"))?.lock()
            .map_err(|e| anyhow::anyhow!("kuzu lock poisoned: {}", e))?;
        let conn = kuzu::Connection::new(&*db_guard)?;
        let mut result = conn.query(cypher)?;
        let mut rows = Vec::new();
        while result.has_next() {
            if let Ok(row) = result.get_next() {
                rows.push(serde_json::json!({ "row": format!("{:?}", row) }));
            }
        }
        Ok(rows)
    }

    pub fn add_entity(name: &str, kind: &str, properties: &str) -> Result<()> {
        let db_guard = DB.get().ok_or_else(|| anyhow::anyhow!("kuzu not initialized"))?.lock()
            .map_err(|e| anyhow::anyhow!("kuzu lock poisoned: {}", e))?;
        let conn = kuzu::Connection::new(&*db_guard)?;
        let cypher = format!(
            "MERGE (e:Entity {{name: '{}', kind: '{}', properties: '{}'}})",
            name.replace('\'', "''"),
            kind.replace('\'', "''"),
            properties.replace('\'', "''")
        );
        let _ = conn.query(&cypher);
        Ok(())
    }

    pub fn add_relationship(from: &str, to: &str, relation: &str) -> Result<()> {
        let db_guard = DB.get().ok_or_else(|| anyhow::anyhow!("kuzu not initialized"))?.lock()
            .map_err(|e| anyhow::anyhow!("kuzu lock poisoned: {}", e))?;
        let conn = kuzu::Connection::new(&*db_guard)?;
        let cypher = format!(
            "MATCH (a:Entity {{name: '{}'}}), (b:Entity {{name: '{}'}}) \
             MERGE (a)-[:RELATED_TO {{relation: '{}'}}]->(b)",
            from.replace('\'', "''"),
            to.replace('\'', "''"),
            relation.replace('\'', "''")
        );
        let _ = conn.query(&cypher);
        Ok(())
    }
}

#[cfg(feature = "kuzu")]
pub use kg_kuzu::{init as kg_init, query as kg_query, add_entity as kg_add_entity, add_relationship as kg_add_rel};

// ─── Postgres-based knowledge graph fallback ───
// When Kuzu is not compiled, use Postgres JSON tables for entity storage.
// This gives you a working knowledge graph without the Kuzu dependency.

#[cfg(not(feature = "kuzu"))]
pub mod kg_postgres {
    use super::*;

    pub fn init(_path: &str) -> Result<()> {
        // Create the knowledge graph tables in Postgres if they don't exist
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                if let Some(pool) = super::pool() {
                    let _ = sqlx::query(
                        r#"CREATE TABLE IF NOT EXISTS kg_entities (
                            name TEXT PRIMARY KEY,
                            kind TEXT NOT NULL,
                            properties JSONB DEFAULT '{}',
                            created_at TIMESTAMPTZ DEFAULT now(),
                            updated_at TIMESTAMPTZ DEFAULT now()
                        )"#
                    ).execute(pool).await;

                    let _ = sqlx::query(
                        r#"CREATE TABLE IF NOT EXISTS kg_relationships (
                            id BIGSERIAL PRIMARY KEY,
                            from_entity TEXT NOT NULL REFERENCES kg_entities(name) ON DELETE CASCADE,
                            to_entity TEXT NOT NULL REFERENCES kg_entities(name) ON DELETE CASCADE,
                            relation TEXT NOT NULL,
                            properties JSONB DEFAULT '{}',
                            created_at TIMESTAMPTZ DEFAULT now(),
                            UNIQUE(from_entity, to_entity, relation)
                        )"#
                    ).execute(pool).await;

                    let _ = sqlx::query(
                        "CREATE INDEX IF NOT EXISTS idx_kg_entities_kind ON kg_entities(kind)"
                    ).execute(pool).await;

                    let _ = sqlx::query(
                        "CREATE INDEX IF NOT EXISTS idx_kg_rel_from ON kg_relationships(from_entity)"
                    ).execute(pool).await;

                    tracing::info!("Knowledge graph initialized (Postgres fallback mode)");
                } else {
                    tracing::warn!("Knowledge graph: no Postgres — graph features unavailable");
                }
            });
        });
        Ok(())
    }

    pub fn query(cypher_like: &str) -> Result<Vec<serde_json::Value>> {
        // Parse simplified Cypher-like queries into SQL
        // Supports: MATCH (e:Entity) WHERE e.kind = 'X' RETURN e
        //           MATCH (a)-[r]->(b) WHERE a.name = 'X' RETURN b
        //           Simple entity and relationship lookups

        let q = cypher_like.trim().to_lowercase();

        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let Some(pool) = super::pool() else {
                    return Ok(vec![serde_json::json!({"error": "no database"})]);
                };

                // Entity search by kind
                if q.contains("kind") || q.contains("type") {
                    let kind = extract_quoted_value(cypher_like, "kind")
                        .or_else(|| extract_quoted_value(cypher_like, "type"))
                        .unwrap_or_default();

                    let rows: Vec<(String, String, Option<serde_json::Value>)> = sqlx::query_as(
                        "SELECT name, kind, properties FROM kg_entities WHERE kind ILIKE $1 ORDER BY name LIMIT 50"
                    ).bind(format!("%{}%", kind)).fetch_all(pool).await.unwrap_or_default();

                    return Ok(rows.iter().map(|(name, kind, props)| {
                        serde_json::json!({"name": name, "kind": kind, "properties": props})
                    }).collect());
                }

                // Entity search by name
                if q.contains("name") {
                    let name = extract_quoted_value(cypher_like, "name").unwrap_or_default();
                    let rows: Vec<(String, String, Option<serde_json::Value>)> = sqlx::query_as(
                        "SELECT name, kind, properties FROM kg_entities WHERE name ILIKE $1 ORDER BY name LIMIT 50"
                    ).bind(format!("%{}%", name)).fetch_all(pool).await.unwrap_or_default();

                    return Ok(rows.iter().map(|(name, kind, props)| {
                        serde_json::json!({"name": name, "kind": kind, "properties": props})
                    }).collect());
                }

                // Relationship query — find connected entities
                if q.contains("related") || q.contains("relationship") || q.contains("->") {
                    let entity = extract_quoted_value(cypher_like, "name")
                        .or_else(|| extract_quoted_value(cypher_like, "from"))
                        .unwrap_or_default();

                    let rows: Vec<(String, String, String)> = sqlx::query_as(
                        "SELECT r.from_entity, r.relation, r.to_entity FROM kg_relationships r \
                         WHERE r.from_entity ILIKE $1 OR r.to_entity ILIKE $1 \
                         ORDER BY r.created_at DESC LIMIT 50"
                    ).bind(format!("%{}%", entity)).fetch_all(pool).await.unwrap_or_default();

                    return Ok(rows.iter().map(|(from, rel, to)| {
                        serde_json::json!({"from": from, "relation": rel, "to": to})
                    }).collect());
                }

                // Default: list all entities
                let rows: Vec<(String, String)> = sqlx::query_as(
                    "SELECT name, kind FROM kg_entities ORDER BY updated_at DESC LIMIT 50"
                ).fetch_all(pool).await.unwrap_or_default();

                Ok(rows.iter().map(|(name, kind)| {
                    serde_json::json!({"name": name, "kind": kind})
                }).collect())
            })
        })
    }

    pub fn add_entity(name: &str, kind: &str, properties: &str) -> Result<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                if let Some(pool) = super::pool() {
                    let props: serde_json::Value = serde_json::from_str(properties)
                        .unwrap_or(serde_json::json!({"raw": properties}));
                    let _ = sqlx::query(
                        "INSERT INTO kg_entities (name, kind, properties, updated_at) VALUES ($1, $2, $3, now()) \
                         ON CONFLICT (name) DO UPDATE SET kind = $2, properties = $3, updated_at = now()"
                    ).bind(name).bind(kind).bind(props).execute(pool).await;
                }
                Ok(())
            })
        })
    }

    pub fn add_relationship(from: &str, to: &str, relation: &str) -> Result<()> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                if let Some(pool) = super::pool() {
                    // Ensure both entities exist
                    let _ = sqlx::query(
                        "INSERT INTO kg_entities (name, kind) VALUES ($1, 'unknown') ON CONFLICT DO NOTHING"
                    ).bind(from).execute(pool).await;
                    let _ = sqlx::query(
                        "INSERT INTO kg_entities (name, kind) VALUES ($1, 'unknown') ON CONFLICT DO NOTHING"
                    ).bind(to).execute(pool).await;
                    // Add relationship
                    let _ = sqlx::query(
                        "INSERT INTO kg_relationships (from_entity, to_entity, relation) VALUES ($1, $2, $3) \
                         ON CONFLICT (from_entity, to_entity, relation) DO NOTHING"
                    ).bind(from).bind(to).bind(relation).execute(pool).await;
                }
                Ok(())
            })
        })
    }

    fn extract_quoted_value(text: &str, key: &str) -> Option<String> {
        // Extract value after key = 'value' or key = "value"
        let patterns = [
            format!("{} = '", key),
            format!("{} = \"", key),
            format!("{}='", key),
            format!("{}=\"", key),
        ];
        for pat in &patterns {
            if let Some(start) = text.to_lowercase().find(&pat.to_lowercase()) {
                let after = &text[start + pat.len()..];
                let end_char = if pat.ends_with('\'') { '\'' } else { '"' };
                if let Some(end) = after.find(end_char) {
                    return Some(after[..end].to_string());
                }
            }
        }
        None
    }
}

#[cfg(not(feature = "kuzu"))]
pub use kg_postgres::{init as kg_init, query as kg_query, add_entity as kg_add_entity, add_relationship as kg_add_rel};

/// Extract entities from text and add to knowledge graph.
/// Called during document ingestion to build the graph automatically.
/// Promote entities that appear frequently.
/// Entities mentioned 3+ times across different documents get "established" status.
/// Called during consolidation cycles.
pub async fn promote_patterns() {
    if let Some(pool) = pool() {
        // Find entities referenced by 3+ different documents
        let promoted = sqlx::query(
            "UPDATE kg_entities SET \
             properties = jsonb_set(COALESCE(properties, '{}'), '{status}', '\"established\"') \
             WHERE name IN ( \
               SELECT to_entity FROM kg_relationships \
               WHERE relation = 'mentions' \
               GROUP BY to_entity HAVING COUNT(DISTINCT from_entity) >= 3 \
             ) AND (properties->>'status' IS NULL OR properties->>'status' != 'established')"
        ).execute(pool).await;

        if let Ok(result) = promoted {
            let count = result.rows_affected();
            if count > 0 {
                tracing::info!(promoted = count, "pattern promotion: {} entities → established (3+ document mentions)");
            }
        }
    }
}

/// Pathfinder: find scored paths between two entities in the knowledge graph.
/// Scores paths by: hop count (shorter = better) + entity status (established = bonus).
pub async fn kg_pathfind(from: &str, to: &str, max_depth: u32) -> Result<Vec<serde_json::Value>> {
    let Some(pool) = pool() else { return Ok(vec![]); };

    // BFS to find paths from → to via relationships (max depth hops)
    let mut paths: Vec<serde_json::Value> = Vec::new();
    let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
    let mut queue: std::collections::VecDeque<(String, Vec<String>, f32)> = std::collections::VecDeque::new();

    queue.push_back((from.to_string(), vec![from.to_string()], 0.0));
    visited.insert(from.to_string());

    while let Some((current, path, cost)) = queue.pop_front() {
        if path.len() > (max_depth as usize + 1) {
            continue;
        }

        // Check if we reached the target
        if current.to_lowercase() == to.to_lowercase() && path.len() > 1 {
            let score = 1.0 / (cost + path.len() as f32); // shorter + lower cost = higher score
            paths.push(serde_json::json!({
                "path": path,
                "hops": path.len() - 1,
                "cost": cost,
                "score": score,
            }));
            continue;
        }

        // Expand neighbors
        let neighbors: Vec<(String, String)> = sqlx::query_as(
            "SELECT to_entity, relation FROM kg_relationships WHERE from_entity = $1 \
             UNION SELECT from_entity, relation FROM kg_relationships WHERE to_entity = $1"
        ).bind(&current).fetch_all(pool).await.unwrap_or_default();

        for (neighbor, _relation) in neighbors {
            if !visited.contains(&neighbor) {
                visited.insert(neighbor.clone());

                // Check if neighbor is "established" (promoted) — lower cost
                let props: Option<(Option<serde_json::Value>,)> = sqlx::query_as(
                    "SELECT properties FROM kg_entities WHERE name = $1"
                ).bind(&neighbor).fetch_optional(pool).await.unwrap_or(None);

                let hop_cost = if props.and_then(|p| p.0).and_then(|v| v.get("status").and_then(|s| s.as_str()).map(|s| s == "established")).unwrap_or(false) {
                    0.5 // established entities are cheaper to traverse
                } else {
                    1.0
                };

                let mut new_path = path.clone();
                new_path.push(neighbor.clone());
                queue.push_back((neighbor, new_path, cost + hop_cost));
            }
        }
    }

    // Sort by score (highest first)
    paths.sort_by(|a, b| {
        let sa = a["score"].as_f64().unwrap_or(0.0);
        let sb = b["score"].as_f64().unwrap_or(0.0);
        sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(paths.into_iter().take(5).collect()) // top 5 paths
}

pub fn extract_and_store_entities(source: &str, content: &str) {
    // Simple entity extraction — keywords → entities
    // For production: use NER model via Ollama
    let words: Vec<&str> = content.split_whitespace().collect();

    // Extract technology names (common patterns)
    let tech_keywords = [
        "rust", "typescript", "python", "docker", "kubernetes", "nixos", "linux",
        "postgresql", "postgres", "qdrant", "redis", "nats", "ollama", "whisper",
        "piper", "grafana", "prometheus", "loki", "frigate", "pihole", "truenas",
        "wireguard", "tailscale", "nginx", "react", "nextjs", "flutter", "tauri",
        "git", "github", "ansible", "terraform", "mqtt", "signal",
    ];

    for keyword in &tech_keywords {
        if content.to_lowercase().contains(keyword) {
            let _ = kg_add_entity(keyword, "technology", "{}");
            // Link to source document
            let _ = kg_add_rel(source, keyword, "mentions");
        }
    }

    // Add the source as an entity
    let _ = kg_add_entity(source, "document", &format!("{{\"length\": {}}}", content.len()));
}

// Initialize vector + graph stores – call from memory::init()
pub async fn init_vector_graph() {
    // Qdrant – lazy init on first query
    let _ = qdrant_client().await;
    // Kuzu / Postgres knowledge graph
    let kg_path = std::env::var("KUZU_PATH").unwrap_or("/var/lib/rednode/kuzu".into());
    let _ = std::fs::create_dir_all(std::path::Path::new(&kg_path).parent().unwrap_or(std::path::Path::new("/tmp")));
    if let Err(e) = kg_init(&kg_path) {
        tracing::warn!("Knowledge graph init failed: {}", e);
    }
}

