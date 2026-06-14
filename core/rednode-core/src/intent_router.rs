/// Intent Router — entry point for all user intentions.
///
/// Enriches the intent with session context from memory before
/// forwarding to the coordinator for planning + execution.
pub async fn handle_intent(
    intent: &str,
    session: &str,
) -> (Vec<serde_json::Value>, Vec<serde_json::Value>) {
    tracing::info!(intent, session, "routing intention through CNS");

    // Enrich: query RAG for relevant context (last 3 results)
    let context = match crate::memory::rag_query(intent, 3).await {
        Ok(hits) => {
            let relevant: Vec<String> = hits
                .iter()
                .filter(|h| h.score > 0.6 && !h.metadata.get("fallback").and_then(|v| v.as_bool()).unwrap_or(false))
                .map(|h| h.content.chars().take(200).collect::<String>())
                .collect();
            if !relevant.is_empty() {
                tracing::debug!(
                    intent,
                    hits = relevant.len(),
                    "intent enriched with {} RAG context items",
                    relevant.len()
                );
            }
            relevant
        }
        Err(_) => vec![],
    };

    // Build enriched intent if we have context
    let enriched_intent = if context.is_empty() {
        intent.to_string()
    } else {
        format!(
            "{}\n\n[Context from memory: {}]",
            intent,
            context.join(" | ")
        )
    };

    // Log the intent to memory for future context
    let _ = crate::memory::audit_log(
        "user",
        "intent",
        None,
        &serde_json::json!({"intent": intent, "session": session}),
        "low",
        true,
        "",
    )
    .await;

    crate::coordinator::coordinate(&enriched_intent, session).await
}
