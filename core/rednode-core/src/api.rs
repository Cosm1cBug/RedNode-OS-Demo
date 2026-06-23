use axum::{
    extract::{ws::{WebSocket, WebSocketUpgrade}, Path, Query},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

#[derive(Deserialize)]
pub struct IntentRequest {
    pub intent: String,
    pub session_id: Option<String>,
}

#[derive(Serialize)]
pub struct IntentResponse {
    pub ok: bool,
    pub intent: String,
    pub plan: Vec<serde_json::Value>,
    pub results: Vec<serde_json::Value>,
}

async fn health() -> Json<serde_json::Value> {
    let uptime = if let Some(engine) = crate::sentience::get().await {
        let model = engine.get_model().await;
        model.uptime_secs
    } else {
        0
    };
    Json(serde_json::json!({
        "ok": true,
        "node": "rednode-cns",
        "version": "0.8.0",
        "uptime_secs": uptime
    }))
}

async fn intent_handler(Json(req): Json<IntentRequest>) -> Json<IntentResponse> {
    let session = req.session_id.as_deref().unwrap_or("default");
    tracing::info!(intent = %req.intent, session, "intention received");

    // Emit to event bus
    crate::events::emit_intent(&req.intent, session);

    let (plan, results) = crate::intent_router::handle_intent(&req.intent, session).await;

    // Emit plan to event bus
    crate::events::emit_plan(&req.intent, &plan);

    // Emit each result
    for r in &results {
        let tool = r.get("tool").and_then(|v| v.as_str()).unwrap_or("-");
        let agent = r.get("agent").and_then(|v| v.as_str()).unwrap_or("-");
        let status = r.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");
        let audit_id = r.get("result")
            .and_then(|v| v.get("audit_id"))
            .and_then(|v| v.as_i64());
        crate::events::emit_tool_result(tool, agent, status, audit_id);
    }

    // Record task completion in sentience
    if let Some(engine) = crate::sentience::get().await {
        for r in &results {
            if let Some(agent) = r.get("agent").and_then(|v| v.as_str()) {
                let agent_name = agent.replace("-agent", "");
                engine.record_task_completed(&agent_name).await;
            }
        }
    }

    Json(IntentResponse {
        ok: true,
        intent: req.intent,
        plan,
        results,
    })
}

// ─── WebSocket — Real-Time Event Stream ───

async fn ws_handler(ws: WebSocketUpgrade) -> Response {
    ws.on_upgrade(handle_ws)
}

async fn handle_ws(mut socket: WebSocket) {
    use axum::extract::ws::Message;

    // Send hello
    let _ = socket
        .send(Message::Text(
            serde_json::json!({
                "type": "hello",
                "node": "rednode-cns",
                "version": "0.8.0",
                "ts": chrono::Utc::now().to_rfc3339()
            })
            .to_string(),
        ))
        .await;

    // Subscribe to the event bus
    let mut rx = match crate::events::subscribe() {
        Some(rx) => rx,
        None => {
            let _ = socket
                .send(Message::Text(
                    r#"{"type":"error","message":"event bus not initialized"}"#.into(),
                ))
                .await;
            return;
        }
    };

    // Forward events from broadcast channel to WebSocket
    // Also handle incoming messages from the client (e.g., ping/pong)
    loop {
        tokio::select! {
            // Event from bus → send to client
            event = rx.recv() => {
                match event {
                    Ok(ev) => {
                        let text = serde_json::to_string(&ev).unwrap_or_default();
                        if socket.send(Message::Text(text)).await.is_err() {
                            // Client disconnected
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        // Client is slow, skipped n events — that's OK
                        tracing::debug!(skipped = n, "WebSocket client lagging, skipped events");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break; // Bus shut down
                    }
                }
            }
            // Message from client (ping, close, etc.)
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(data))) => {
                        let _ = socket.send(Message::Pong(data)).await;
                    }
                    _ => {} // Ignore text/binary from client for now
                }
            }
        }
    }
}

// ─── Audit ───

async fn audit_log(Query(params): Query<HashMap<String, String>>) -> Json<serde_json::Value> {
    let limit = params
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(100);
    let rows = crate::memory::get_audit(limit).await.unwrap_or_default();
    Json(serde_json::json!({"ok": true, "entries": rows}))
}

// ─── Approvals ───

async fn list_approvals() -> Json<serde_json::Value> {
    let rows = crate::memory::list_approvals("pending")
        .await
        .unwrap_or_default();
    Json(serde_json::json!({"ok": true, "approvals": rows}))
}

#[derive(Deserialize)]
struct ApproveBody {
    approved: bool,
}

async fn approve_handler(
    Path(id): Path<uuid::Uuid>,
    Json(body): Json<ApproveBody>,
) -> Json<serde_json::Value> {
    let result = crate::memory::approve_id(id, body.approved).await;

    // Emit approval decision to event bus
    crate::events::emit(serde_json::json!({
        "type": "approval_decision",
        "id": id.to_string(),
        "approved": body.approved,
        "ts": chrono::Utc::now().to_rfc3339()
    }));

    match result {
        Ok(true) => Json(serde_json::json!({"ok": true, "id": id, "approved": body.approved})),
        Ok(false) => Json(serde_json::json!({"ok": false, "error": "not_found"})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

// ─── Memory – RAG ───

async fn memory_query(Query(params): Query<HashMap<String, String>>) -> Json<serde_json::Value> {
    let q = params.get("q").cloned().unwrap_or_default();
    let limit = params
        .get("limit")
        .and_then(|s| s.parse().ok())
        .unwrap_or(5);
    match crate::memory::rag_query(&q, limit).await {
        Ok(results) => Json(serde_json::json!({ "ok": true, "query": q, "results": results })),
        Err(e) => Json(serde_json::json!({ "ok": false, "query": q, "error": e.to_string(), "results": [] })),
    }
}

#[derive(Deserialize)]
struct IngestBody {
    source: String,
    content: String,
}

async fn memory_ingest(Json(body): Json<IngestBody>) -> Json<serde_json::Value> {
    match crate::memory::ingest_document(&body.source, &body.content).await {
        Ok(id) => Json(serde_json::json!({"ok": true, "id": id})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

// ─── Security Events ───

async fn security_events() -> Json<serde_json::Value> {
    let rows = crate::memory::list_security_events(100)
        .await
        .unwrap_or_default();
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
    // Emit to event bus for real-time dashboard
    crate::events::emit_security_event(&ev.severity, &ev.source, &ev.summary);

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

// ─── Agents ───

async fn agents_status() -> Json<serde_json::Value> {
    // Real agent status from Sentience Engine (tracks NATS heartbeats)
    if let Some(engine) = crate::sentience::get().await {
        let model = engine.get_model().await;
        let agents: Vec<serde_json::Value> = model
            .agents
            .iter()
            .map(|a| {
                serde_json::json!({
                    "name": format!("{}-agent", a.name),
                    "status": a.status,
                    "last_heartbeat": a.last_heartbeat.to_rfc3339(),
                    "alive": a.is_alive(),
                    "tasks_completed": a.tasks_completed,
                })
            })
            .collect();
        Json(serde_json::json!({"ok": true, "agents": agents}))
    } else {
        // Fallback if sentience not running
        Json(serde_json::json!({
            "ok": true,
            "agents": [],
            "note": "sentience engine not running — agent tracking unavailable"
        }))
    }
}

// ─── Sentience ───

async fn sentience_status() -> Json<serde_json::Value> {
    if let Some(engine) = crate::sentience::get().await {
        let model = engine.get_model().await;
        Json(serde_json::json!({ "ok": true, "sentience": true, "model": model }))
    } else {
        Json(serde_json::json!({ "ok": true, "sentience": false }))
    }
}

// ─── Knowledge Graph ───

async fn kg_query_handler(Query(params): Query<HashMap<String, String>>) -> Json<serde_json::Value> {
    let q = params.get("q").or(params.get("cypher")).cloned().unwrap_or_default();
    if q.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "Missing 'q' or 'cypher' query parameter"}));
    }
    match crate::memory::kg_query(&q) {
        Ok(results) => Json(serde_json::json!({"ok": true, "query": q, "results": results})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": e.to_string()})),
    }
}

#[derive(Deserialize)]
struct KgEntityBody {
    name: String,
    kind: String,
    #[serde(default)]
    properties: String,
    #[serde(default)]
    relationships: Vec<KgRelBody>,
}

#[derive(Deserialize)]
struct KgRelBody {
    to: String,
    relation: String,
}

async fn kg_add_entity_handler(Json(body): Json<KgEntityBody>) -> Json<serde_json::Value> {
    if let Err(e) = crate::memory::kg_add_entity(&body.name, &body.kind, &body.properties) {
        return Json(serde_json::json!({"ok": false, "error": e.to_string()}));
    }
    for rel in &body.relationships {
        let _ = crate::memory::kg_add_rel(&body.name, &rel.to, &rel.relation);
    }
    Json(serde_json::json!({"ok": true, "entity": body.name, "kind": body.kind, "relationships": body.relationships.len()}))
}

// ─── Router ───

pub fn router() -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/intent", post(intent_handler))
        .route("/events", get(ws_handler))
        // Audit
        .route("/audit", get(audit_log))
        // Approvals
        .route("/approvals", get(list_approvals))
        .route("/approvals/:id/approve", post(approve_handler))
        // Memory – RAG
        .route("/memory/query", get(memory_query))
        .route("/memory/ingest", post(memory_ingest))
        // Security
        .route(
            "/security/events",
            get(security_events).post(security_event_post),
        )
        .route("/security/events/:id/ack", post(security_event_ack))
        // Agents
        .route("/agents/status", get(agents_status))
        // Sentience
        .route("/sentience", get(sentience_status))
        // Knowledge Graph
        .route("/kg/query", get(kg_query_handler))
        .route("/kg/entity", post(kg_add_entity_handler))
        // Auth middleware — checks Bearer token on all routes except /health and /events
        // Set REDNODE_API_TOKEN env var to enable. If unset, auth is disabled (dev mode).
        .layer(axum::middleware::from_fn(crate::auth::auth_middleware))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
