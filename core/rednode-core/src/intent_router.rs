pub async fn handle_intent(intent: &str, session: &str) -> (Vec<serde_json::Value>, Vec<serde_json::Value>) {
    tracing::info!(intent, session, "routing intention through CNS");
    crate::coordinator::coordinate(intent, session).await
}
