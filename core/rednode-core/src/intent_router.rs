/// Intent Router — entry point for all user intentions.
///
/// Multi-turn conversation support:
///   - Maintains per-session conversation history (last 10 turns)
///   - Enriches each intent with session context + RAG memory
///   - Resolves pronouns/references ("do that again", "on the same machine")
///   - Logs every intent for future pattern analysis
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Per-session conversation history
#[derive(Debug, Clone)]
struct ConversationTurn {
    intent: String,
    summary: String, // brief result summary
    ts: chrono::DateTime<chrono::Utc>,
}

/// Session store — holds recent conversation history per session ID
static SESSIONS: once_cell::sync::Lazy<Arc<RwLock<HashMap<String, Vec<ConversationTurn>>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(HashMap::new())));

const MAX_TURNS_PER_SESSION: usize = 10;
const SESSION_EXPIRY_SECS: i64 = 3600; // 1 hour of inactivity

pub async fn handle_intent(
    intent: &str,
    session: &str,
) -> (Vec<serde_json::Value>, Vec<serde_json::Value>) {
    tracing::info!(intent, session, "routing intention through CNS");

    // ── Step 1: Load session history ──
    let session_context = get_session_context(session).await;

    // ── Step 2: Resolve references ("do that again", "same thing", "it") ──
    let resolved_intent = resolve_references(intent, &session_context);

    // ── Step 3: Query RAG for relevant knowledge ──
    let rag_context = match crate::memory::rag_query(&resolved_intent, 3).await {
        Ok(hits) => hits
            .iter()
            .filter(|h| {
                h.score > 0.6
                    && !h
                        .metadata
                        .get("fallback")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
            })
            .map(|h| h.content.chars().take(200).collect::<String>())
            .collect::<Vec<_>>(),
        Err(_) => vec![],
    };

    // ── Step 4: Build enriched intent with all context ──
    let mut context_parts: Vec<String> = Vec::new();

    // Add session history (last 3 turns)
    if !session_context.is_empty() {
        let history: Vec<String> = session_context
            .iter()
            .rev()
            .take(3)
            .rev()
            .map(|t| format!("User said: '{}' → {}", t.intent, t.summary))
            .collect();
        context_parts.push(format!("Conversation history:\n{}", history.join("\n")));
    }

    // Add RAG context
    if !rag_context.is_empty() {
        context_parts.push(format!("Relevant knowledge: {}", rag_context.join(" | ")));
    }

    let enriched_intent = if context_parts.is_empty() {
        resolved_intent.clone()
    } else {
        format!(
            "{}\n\n[Context:\n{}]",
            resolved_intent,
            context_parts.join("\n")
        )
    };

    // ── Step 5: Execute via coordinator ──
    let (plan, results) = crate::coordinator::coordinate(&enriched_intent, session).await;

    // ── Step 6: Store this turn in session history ──
    let result_summary = summarize_results(&results);
    store_session_turn(session, &resolved_intent, &result_summary).await;

    // ── Step 7: Log to audit trail ──
    let _ = crate::memory::audit_log(
        "user",
        "intent",
        None,
        &serde_json::json!({
            "intent": intent,
            "resolved": resolved_intent,
            "session": session,
            "had_history": !session_context.is_empty(),
            "rag_hits": rag_context.len(),
        }),
        "low",
        true,
        &result_summary,
    )
    .await;

    (plan, results)
}

/// Get recent conversation turns for a session
async fn get_session_context(session: &str) -> Vec<ConversationTurn> {
    let sessions = SESSIONS.read().await;
    if let Some(turns) = sessions.get(session) {
        // Filter out expired turns
        let now = chrono::Utc::now();
        turns
            .iter()
            .filter(|t| (now - t.ts).num_seconds() < SESSION_EXPIRY_SECS)
            .cloned()
            .collect()
    } else {
        vec![]
    }
}

/// Store a conversation turn
async fn store_session_turn(session: &str, intent: &str, summary: &str) {
    let mut sessions = SESSIONS.write().await;
    let turns = sessions.entry(session.to_string()).or_default();

    turns.push(ConversationTurn {
        intent: intent.to_string(),
        summary: summary.to_string(),
        ts: chrono::Utc::now(),
    });

    // Keep only last N turns
    if turns.len() > MAX_TURNS_PER_SESSION {
        let drain = turns.len() - MAX_TURNS_PER_SESSION;
        turns.drain(0..drain);
    }

    // Prune expired sessions periodically (every 100 stores)
    static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    if COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % 100 == 0 {
        let now = chrono::Utc::now();
        sessions.retain(|_, turns| {
            turns
                .last()
                .map(|t| (now - t.ts).num_seconds() < SESSION_EXPIRY_SECS)
                .unwrap_or(false)
        });
    }
}

/// Resolve conversational references
/// "do that again" → repeat last intent
/// "the same" / "same thing" → repeat last intent
/// "also" / "and also" → combine with last intent
fn resolve_references(intent: &str, history: &[ConversationTurn]) -> String {
    if history.is_empty() {
        return intent.to_string();
    }

    let lower = intent.to_lowercase();
    let last = &history[history.len() - 1];

    // "do that again", "repeat that", "same thing", "again"
    if lower == "again"
        || lower == "do that again"
        || lower == "repeat that"
        || lower == "same thing"
        || lower == "do it again"
        || lower == "one more time"
    {
        tracing::info!(resolved_to = %last.intent, "resolved reference: '{}' → repeating last intent", intent);
        return last.intent.clone();
    }

    // "also check X" → "check X" (the LLM planner handles the combination)
    // "and show Y" → "show Y"
    // These don't need special resolution — the planner handles them with session context

    // "what about X" → context from last turn helps the planner
    // No modification needed — session context is already injected

    intent.to_string()
}

/// Create a brief summary of results for session history
fn summarize_results(results: &[serde_json::Value]) -> String {
    if results.is_empty() {
        return "no results".to_string();
    }

    let parts: Vec<String> = results
        .iter()
        .map(|r| {
            let tool = r.get("tool").and_then(|v| v.as_str()).unwrap_or("-");
            let status = r.get("status").and_then(|v| v.as_str()).unwrap_or("?");
            format!("{}:{}", tool, status)
        })
        .collect();

    parts.join(", ")
}
