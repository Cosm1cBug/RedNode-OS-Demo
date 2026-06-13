use axum::{routing::{get, post}, Router, Json, extract::{ws::{WebSocket, WebSocketUpgrade}, Path, Query}, response::Response};
use serde::{Deserialize, Serialize};
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use std::collections::HashMap;

#[derive(Deserialize)] pub struct IntentRequest { pub intent: String, pub session_id: Option<String> }
#[derive(Serialize)] pub struct IntentResponse { pub ok: bool, pub intent: String, pub plan: Vec<serde_json::Value>, pub results: Vec<serde_json::Value> }

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"ok":true,"node":"rednode-cns","version":"0.2.0"}))
}

async fn intent_handler(Json(req): Json<IntentRequest>) -> Json<IntentResponse> {
    tracing::info!(intent=%req.intent, "intention received");
    let result = crate::intent_router::handle_intent(&req.intent, req.session_id.as_deref().unwrap_or("default")).await;
    Json(IntentResponse { ok: true, intent: req.intent, plan: result.0, results: result.1 })
}

async fn ws_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(|mut socket: WebSocket| async move {
        let _ = socket.send(axum::extract::ws::Message::Text(r#"{"type":"hello","node":"rednode-cns"}"#.into())).await;
        // TODO: forward bus events
    })
}

// --- Audit ---
async fn audit_log(Query(params): Query<HashMap<String, String>>) -> Json<serde_json::Value> {
    let limit = params.get("limit").and_then(|s| s.parse().ok()).unwrap_or(100);
    let rows = crate::memory::get_audit(limit).await.unwrap_or_default();
    Json(serde_json::json!({"ok": true, "entries": rows}))
}

// --- Approvals ---
async fn list_approvals() -> Json<serde_json::Value> {
    let rows = crate::memory::list_approvals("pending").await.unwrap_or_default();
    Json(serde_json::json!({"ok": true, "approvals": rows}))
}

#[derive(Deserialize)]
struct ApproveBody { approved: bool }

async fn approve_handler(Path(id): Path<uuid::Uuid>, Json(body): Json<ApproveBody>) -> Json<serde_json::Value> {
    match crate::memory::approve_id(id, body.approved).await {
        Ok(true) => Json(serde_json::json!({"ok": true, "id": id, "approved": body.approved})),
        Ok(false) => Json(serde_json::json!({"ok": false, "error": "not_found"})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

// --- Memory – RAG ---
async fn memory_query(Query(params): Query<HashMap<String, String>>) -> Json<serde_json::Value> {
    let q = params.get("q").cloned().unwrap_or_default();
    let limit = params.get("limit").and_then(|s| s.parse().ok()).unwrap_or(5);
    match crate::memory::rag_query(&q, limit).await {
        Ok(results) => Json(serde_json::json!({ "ok": true, "query": q, "results": results })),
        Err(e) => Json(serde_json::json!({ "ok": false, "query": q, "error": e.to_string(), "results": [] })),
    }
}

#[derive(Deserialize)]
struct IngestBody { source: String, content: String }

async fn memory_ingest(Json(body): Json<IngestBody>) -> Json<serde_json::Value> {
    match crate::memory::ingest_document(&body.source, &body.content).await {
        Ok(id) => Json(serde_json::json!({"ok": true, "id": id})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

// --- Security Events ---
async fn security_events() -> Json<serde_json::Value> {
    let rows = crate::memory::list_security_events(100).await.unwrap_or_default();
    Json(serde_json::json!({"ok": true, "events": rows}))
}

#[derive(Deserialize)]
struct SecurityEventIn {
    severity: String,
    source: String,
    summary: String,
    #[serde(default)]
    raw: serde_json::Value,
}
async fn security_event_post(Json(ev): Json<SecurityEventIn>) -> Json<serde_json::Value> {
    match crate::memory::log_security_event(&ev.severity, &ev.source, &ev.summary, ev.raw).await {
        Ok(id) => Json(serde_json::json!({"ok": true, "id": id})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

async fn security_event_ack(Path(id): Path<uuid::Uuid>) -> Json<serde_json::Value> {
    match crate::memory::ack_security_event(id).await {
        Ok(true) => Json(serde_json::json!({"ok": true})),
        _ => Json(serde_json::json!({"ok": false})),
    }
}

// --- Agents ---
async fn agents_status() -> Json<serde_json::Value> {
    // TODO: real heartbeat tracking via NATS
    Json(serde_json::json!({
        "ok": true,
        "agents": [
            {"name":"system-agent","status":"online","last_heartbeat":"now"},
            {"name":"security-agent","status":"online","last_heartbeat":"now"},
            {"name":"coding-agent","status":"online","last_heartbeat":"now"},
            {"name":"research-agent","status":"online","last_heartbeat":"now"},
            {"name":"automation-agent","status":"online","last_heartbeat":"now"},
            {"name":"network-agent","status":"online","last_heartbeat":"now"}
        ]
    }))
}

// --- Sentience ---
async fn sentience_status() -> Json<serde_json::Value> {
    if let Some(engine) = crate::sentience::get().await {
        let model = engine.get_model().await;
        Json(serde_json::json!({ "ok": true, "sentience": true, "model": model }))
    } else {
        Json(serde_json::json!({ "ok": true, "sentience": false }))
    }
}

pub fn router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/intent", post(intent_handler))
        .route("/events", get(ws_handler))
        // audit
        .route("/audit", get(audit_log))
        // approvals
        .route("/approvals", get(list_approvals))
        .route("/approvals/:id/approve", post(approve_handler))
        // memory – RAG
        .route("/memory/query", get(memory_query))
        .route("/memory/ingest", post(memory_ingest))
        // security
        .route("/security/events", get(security_events).post(security_event_post))
        .route("/security/events/:id/ack", post(security_event_ack))
        // agents
        .route("/agents/status", get(agents_status))
        // sentience – self-aware OS
        .route("/sentience", get(sentience_status))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
