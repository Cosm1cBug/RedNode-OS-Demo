use axum::{
    extract::{ws::{WebSocket, WebSocketUpgrade}, Path, Query},
    response::Response,
    routing::{get, post, delete},
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
        "version": "0.13.0",
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
                "version": "0.13.0",
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

// ─── Tool Evolution ───

#[derive(Deserialize)]
struct EvolveToolReq {
    name: String,
    agent: String,
    description: String,
    handler_type: String,      // "shell", "api", "llm", "cns"
    #[serde(default)]
    handler_command: String,   // the command/URL/prompt
}

async fn evolve_tool_handler(Json(req): Json<EvolveToolReq>) -> Json<serde_json::Value> {
    let project_root = std::env::var("REDNODE_SOURCE")
        .unwrap_or_else(|_| std::env::var("REDNODE_HOME")
            .map(|h| format!("{}/source", h))
            .unwrap_or_else(|_| ".".into()));
    
    match crate::evolution::evolve_tool(
        &project_root,
        &req.name,
        &req.agent,
        &req.description,
        &req.handler_type,
        &req.handler_command,
    ).await {
        Ok(tool) => Json(serde_json::json!({
            "ok": true,
            "message": format!("Tool '{}' evolved successfully", tool.name),
            "tool": {
                "name": tool.name,
                "agent": tool.agent,
                "risk": tool.risk,
                "description": tool.description,
                "handler_type": tool.handler_type,
            }
        })),
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": format!("{}", e),
        })),
    }
}

async fn list_evolved_tools() -> Json<serde_json::Value> {
    let project_root = std::env::var("REDNODE_SOURCE")
        .unwrap_or_else(|_| ".".into());
    
    match crate::evolution::load_tools_registry(&project_root) {
        Ok(tools) => {
            let evolved: Vec<_> = tools.iter()
                .filter(|t| t.auto_generated == Some(true))
                .collect();
            Json(serde_json::json!({
                "ok": true,
                "total_tools": tools.len(),
                "auto_generated": evolved.len(),
                "evolved_tools": evolved,
            }))
        }
        Err(e) => Json(serde_json::json!({
            "ok": false,
            "error": format!("{}", e),
        })),
    }
}


// ─── Configuration API ───

async fn config_dashboard() -> Json<serde_json::Value> {
    Json(crate::config::get_for_dashboard())
}

async fn config_agent() -> Json<serde_json::Value> {
    Json(crate::config::get_for_agents())
}

async fn config_get_service(Path(service): Path<String>) -> Json<serde_json::Value> {
    let config = crate::config::get();
    match config.services.get(&service) {
        Some(svc) => Json(serde_json::json!({"ok": true, "service": svc})),
        None => Json(serde_json::json!({"ok": false, "error": "Unknown service"})),
    }
}

#[derive(Deserialize)]
struct ConfigUpdateReq {
    url: Option<String>,
    enabled: Option<bool>,
}

async fn config_update_service(
    Path(service): Path<String>,
    Json(req): Json<ConfigUpdateReq>,
) -> Json<serde_json::Value> {
    match crate::config::update(|cfg| {
        if let Some(svc) = cfg.services.get_mut(&service) {
            if let Some(ref url) = req.url { svc.url = url.clone(); }
            if let Some(enabled) = req.enabled { svc.enabled = enabled; }
        }
    }) {
        Ok(()) => Json(serde_json::json!({"ok": true, "message": format!("{} updated", service)})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": format!("{}", e)})),
    }
}

async fn config_update_preferences(Json(prefs): Json<serde_json::Value>) -> Json<serde_json::Value> {
    match crate::config::update(|cfg| {
        if let Some(v) = prefs.get("notify_quiet_start").and_then(|v| v.as_u64()) { cfg.preferences.notify_quiet_start = v as u32; }
        if let Some(v) = prefs.get("notify_quiet_end").and_then(|v| v.as_u64()) { cfg.preferences.notify_quiet_end = v as u32; }
        if let Some(v) = prefs.get("notify_channel").and_then(|v| v.as_str()) { cfg.preferences.notify_channel = v.into(); }
        if let Some(v) = prefs.get("voice_enabled").and_then(|v| v.as_bool()) { cfg.preferences.voice_enabled = v; }
        if let Some(v) = prefs.get("voice_wake_word").and_then(|v| v.as_str()) { cfg.preferences.voice_wake_word = v.into(); }
        if let Some(v) = prefs.get("gui_enabled").and_then(|v| v.as_bool()) { cfg.preferences.gui_enabled = v; }
        if let Some(v) = prefs.get("predict_min_days").and_then(|v| v.as_u64()) { cfg.preferences.predict_min_days = v; }
        if let Some(v) = prefs.get("predict_min_logs").and_then(|v| v.as_i64()) { cfg.preferences.predict_min_logs = v; }
        if let Some(v) = prefs.get("weather_location").and_then(|v| v.as_str()) { cfg.preferences.weather_location = v.into(); }
        if let Some(v) = prefs.get("rss_check_interval").and_then(|v| v.as_u64()) { cfg.preferences.rss_check_interval = v; }
    }) {
        Ok(()) => Json(serde_json::json!({"ok": true, "message": "Preferences updated"})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": format!("{}", e)})),
    }
}

#[derive(Deserialize)]
struct SecretUpdateReq {
    service: String,
    key: String,
    value: String,
}

async fn config_set_secret(Json(req): Json<SecretUpdateReq>) -> Json<serde_json::Value> {
    match crate::config::update(|cfg| {
        if let Some(svc) = cfg.services.get_mut(&req.service) {
            svc.secrets.insert(req.key.clone(), req.value.clone());
        }
    }) {
        Ok(()) => Json(serde_json::json!({"ok": true, "message": format!("Secret updated for {}", req.service)})),
        Err(e) => Json(serde_json::json!({"ok": false, "error": format!("{}", e)})),
    }
}

async fn config_test_service(Path(service): Path<String>) -> Json<serde_json::Value> {
    Json(crate::config::test_service(&service).await)
}

async fn config_setup_status() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "setup_complete": crate::config::is_setup_complete() }))
}


// ─── Consciousness API ───

async fn consciousness_status() -> Json<serde_json::Value> {
    let mind = crate::consciousness::get_mind().await;
    Json(serde_json::to_value(&mind).unwrap_or_default())
}

async fn consciousness_summary() -> Json<serde_json::Value> {
    let summary = crate::consciousness::summary().await;
    Json(serde_json::json!({"ok": true, "summary": summary}))
}

// ─── Goals API ───

async fn goals_list() -> Json<serde_json::Value> {
    Json(crate::goals::get_for_api().await)
}

async fn goals_get(Path(id): Path<String>) -> Json<serde_json::Value> {
    match crate::goals::get(&id).await {
        Some(goal) => Json(serde_json::json!({"ok": true, "goal": goal})),
        None => Json(serde_json::json!({"ok": false, "error": "Goal not found"})),
    }
}

#[derive(Deserialize)]
struct CreateGoalReq {
    title: String,
    description: String,
    #[serde(default)]
    tags: Vec<String>,
    target_date: Option<String>,
}

async fn goals_create(Json(req): Json<CreateGoalReq>) -> Json<serde_json::Value> {
    let target = req.target_date.and_then(|d| d.parse::<chrono::DateTime<chrono::Utc>>().ok());
    let goal = crate::goals::create(&req.title, &req.description, req.tags, target).await;
    Json(serde_json::json!({"ok": true, "goal": goal}))
}

#[derive(Deserialize)]
struct AddSubGoalReq {
    title: String,
    description: String,
}

async fn goals_add_subgoal(
    Path(id): Path<String>,
    Json(req): Json<AddSubGoalReq>,
) -> Json<serde_json::Value> {
    match crate::goals::add_sub_goal(&id, &req.title, &req.description).await {
        Some(sub) => Json(serde_json::json!({"ok": true, "sub_goal": sub})),
        None => Json(serde_json::json!({"ok": false, "error": "Goal not found"})),
    }
}

#[derive(Deserialize)]
struct ContributeReq {
    task_description: String,
    #[serde(default)]
    sub_goal_id: Option<String>,
    #[serde(default = "default_impact")]
    impact: f32,
}
fn default_impact() -> f32 { 0.5 }

async fn goals_contribute(
    Path(id): Path<String>,
    Json(req): Json<ContributeReq>,
) -> Json<serde_json::Value> {
    crate::goals::contribute(
        &id,
        &req.task_description,
        req.sub_goal_id.as_deref(),
        req.impact,
        false,
    ).await;
    Json(serde_json::json!({"ok": true, "message": "Contribution recorded"}))
}

async fn goals_delete(Path(id): Path<String>) -> Json<serde_json::Value> {
    crate::goals::set_status(&id, crate::goals::GoalStatus::Abandoned).await;
    Json(serde_json::json!({"ok": true, "message": "Goal deactivated"}))
}


// ─── World Model API ───

async fn world_full() -> Json<serde_json::Value> {
    Json(crate::world_model::get_for_api().await)
}

async fn world_machines_list() -> Json<serde_json::Value> {
    let machines = crate::world_model::get_machines().await;
    Json(serde_json::json!({"ok": true, "count": machines.len(), "machines": machines}))
}

async fn world_machine_get(Path(id): Path<String>) -> Json<serde_json::Value> {
    match crate::world_model::get_machine(&id).await {
        Some(m) => Json(serde_json::json!({"ok": true, "machine": m})),
        None => Json(serde_json::json!({"ok": false, "error": "Machine not found"})),
    }
}

#[derive(Deserialize)]
struct UpsertMachineReq {
    id: Option<String>,
    name: String,
    role: String,
    #[serde(default)]
    ip: Option<String>,
    #[serde(default)]
    mac: Option<String>,
    #[serde(default)]
    os: Option<String>,
    #[serde(default)]
    cpu: Option<String>,
    #[serde(default)]
    ram_gb: Option<u32>,
    #[serde(default)]
    disk_gb: Option<u32>,
    #[serde(default)]
    services: Vec<String>,
    #[serde(default)]
    vlan_id: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    notes: Option<String>,
}

fn parse_machine_role(s: &str) -> crate::world_model::MachineRole {
    match s.to_lowercase().as_str() {
        "server" => crate::world_model::MachineRole::Server,
        "desktop" => crate::world_model::MachineRole::Desktop,
        "nas" => crate::world_model::MachineRole::Nas,
        "firewall" => crate::world_model::MachineRole::Firewall,
        "router" => crate::world_model::MachineRole::Router,
        "switch" => crate::world_model::MachineRole::Switch,
        "raspberrypi" | "rpi" => crate::world_model::MachineRole::RaspberryPi,
        "nvr" => crate::world_model::MachineRole::Nvr,
        "iothub" => crate::world_model::MachineRole::IoTHub,
        "workstation" => crate::world_model::MachineRole::Workstation,
        other => crate::world_model::MachineRole::Other(other.to_string()),
    }
}

async fn world_machine_upsert(Json(req): Json<UpsertMachineReq>) -> Json<serde_json::Value> {
    let id = req.id.unwrap_or_else(|| format!("m_{}", chrono::Utc::now().timestamp_millis()));
    let machine = crate::world_model::Machine {
        id: id.clone(),
        name: req.name.clone(),
        role: parse_machine_role(&req.role),
        ip: req.ip,
        mac: req.mac,
        os: req.os,
        cpu: req.cpu,
        ram_gb: req.ram_gb,
        disk_gb: req.disk_gb,
        services: req.services,
        vlan_id: req.vlan_id,
        last_seen: chrono::Utc::now(),
        health: 1.0,
        tags: req.tags,
        notes: req.notes,
    };
    crate::world_model::upsert_machine(machine).await;
    Json(serde_json::json!({"ok": true, "id": id, "name": req.name}))
}

async fn world_machine_delete(Path(id): Path<String>) -> Json<serde_json::Value> {
    let removed = crate::world_model::remove_machine(&id).await;
    Json(serde_json::json!({"ok": removed, "id": id}))
}

async fn world_services_list() -> Json<serde_json::Value> {
    let services = crate::world_model::get_services().await;
    Json(serde_json::json!({"ok": true, "count": services.len(), "services": services}))
}

#[derive(Deserialize)]
struct UpsertServiceReq {
    id: Option<String>,
    name: String,
    service_type: String,
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    port: Option<u16>,
    #[serde(default)]
    host_machine_id: Option<String>,
    #[serde(default)]
    version: Option<String>,
    #[serde(default)]
    tags: Vec<String>,
}

fn parse_service_type(s: &str) -> crate::world_model::ServiceType {
    match s.to_lowercase().as_str() {
        "database" | "db" => crate::world_model::ServiceType::Database,
        "messagebroker" | "broker" | "nats" | "mqtt" => crate::world_model::ServiceType::MessageBroker,
        "webapp" | "web" => crate::world_model::ServiceType::WebApp,
        "api" => crate::world_model::ServiceType::Api,
        "dns" => crate::world_model::ServiceType::Dns,
        "dhcp" => crate::world_model::ServiceType::Dhcp,
        "firewall" => crate::world_model::ServiceType::Firewall,
        "vpn" => crate::world_model::ServiceType::VPN,
        "filestorage" | "storage" | "nfs" | "smb" => crate::world_model::ServiceType::FileStorage,
        "mediaserver" | "media" => crate::world_model::ServiceType::MediaServer,
        "homeautomation" | "ha" => crate::world_model::ServiceType::HomeAutomation,
        "monitoring" => crate::world_model::ServiceType::Monitoring,
        "container" | "docker" | "podman" => crate::world_model::ServiceType::Container,
        "llm" | "ollama" => crate::world_model::ServiceType::LLM,
        other => crate::world_model::ServiceType::Custom(other.to_string()),
    }
}

async fn world_service_upsert(Json(req): Json<UpsertServiceReq>) -> Json<serde_json::Value> {
    let id = req.id.unwrap_or_else(|| format!("svc_{}", chrono::Utc::now().timestamp_millis()));
    let service = crate::world_model::ServiceNode {
        id: id.clone(),
        name: req.name.clone(),
        service_type: parse_service_type(&req.service_type),
        url: req.url,
        port: req.port,
        host_machine_id: req.host_machine_id,
        status: crate::world_model::ServiceStatus::Unknown,
        last_checked: chrono::Utc::now(),
        version: req.version,
        tags: req.tags,
    };
    crate::world_model::upsert_service(service).await;
    Json(serde_json::json!({"ok": true, "id": id, "name": req.name}))
}

async fn world_topology() -> Json<serde_json::Value> {
    let vlans = crate::world_model::get_topology().await;
    Json(serde_json::json!({"ok": true, "vlans": vlans}))
}

async fn world_dependencies(Path(id): Path<String>) -> Json<serde_json::Value> {
    let deps = crate::world_model::get_dependencies(&id).await;
    Json(serde_json::json!({
        "ok": true,
        "entity_id": id,
        "dependency_count": deps.len(),
        "dependencies": deps,
    }))
}

async fn world_impact(Path(id): Path<String>) -> Json<serde_json::Value> {
    let report = crate::world_model::impact_analysis(&id).await;
    Json(serde_json::json!({
        "ok": true,
        "report": report,
    }))
}

async fn world_diff() -> Json<serde_json::Value> {
    match crate::world_model::get_diff().await {
        Some(diff) => Json(serde_json::json!({"ok": true, "diff": diff})),
        None => Json(serde_json::json!({"ok": false, "error": "No previous snapshot available"})),
    }
}

#[derive(Deserialize)]
struct AddEdgeReq {
    from_id: String,
    to_id: String,
    relation: String,
    #[serde(default)]
    metadata: Option<String>,
}

fn parse_edge_relation(s: &str) -> crate::world_model::EdgeRelation {
    match s.to_lowercase().as_str() {
        "dependson" | "depends_on" => crate::world_model::EdgeRelation::DependsOn,
        "hosts" => crate::world_model::EdgeRelation::Hosts,
        "runson" | "runs_on" => crate::world_model::EdgeRelation::RunsOn,
        "connectedto" | "connected_to" => crate::world_model::EdgeRelation::ConnectedTo,
        "monitors" => crate::world_model::EdgeRelation::Monitors,
        "backsup" | "backs_up" => crate::world_model::EdgeRelation::BacksUp,
        "routesthrough" | "routes_through" => crate::world_model::EdgeRelation::RoutesThrough,
        "ownedby" | "owned_by" => crate::world_model::EdgeRelation::OwnedBy,
        "partof" | "part_of" => crate::world_model::EdgeRelation::PartOf,
        other => crate::world_model::EdgeRelation::Custom(other.to_string()),
    }
}

async fn world_add_edge(Json(req): Json<AddEdgeReq>) -> Json<serde_json::Value> {
    let relation = parse_edge_relation(&req.relation);
    crate::world_model::add_edge(&req.from_id, &req.to_id, relation, req.metadata).await;
    Json(serde_json::json!({"ok": true, "from": req.from_id, "to": req.to_id, "relation": req.relation}))
}

async fn world_scan() -> Json<serde_json::Value> {
    // Trigger an async network scan (placeholder — actual scan logic lives in the
    // infra-agent and gets reported back via NATS heartbeats / world_model::upsert_*)
    crate::events::emit(serde_json::json!({
        "type": "world_scan_requested",
        "ts": chrono::Utc::now().to_rfc3339(),
    }));
    Json(serde_json::json!({"ok": true, "message": "Network scan requested"}))
}

async fn world_summary() -> Json<serde_json::Value> {
    let summary = crate::world_model::summary().await;
    Json(serde_json::json!({"ok": true, "summary": summary}))
}


// ─── Time Intelligence API ───

async fn time_awareness() -> Json<serde_json::Value> {
    Json(crate::time_intel::get_for_api().await)
}

async fn time_events_list() -> Json<serde_json::Value> {
    let events = crate::time_intel::get_events().await;
    Json(serde_json::json!({"ok": true, "count": events.len(), "events": events}))
}

#[derive(Deserialize)]
struct CreateTimeEventReq {
    name: String,
    description: String,
    when: String,
    #[serde(default = "default_event_category")]
    category: String,
    #[serde(default)]
    auto_execute: bool,
    #[serde(default)]
    action: Option<String>,
}
fn default_event_category() -> String { "custom".into() }

fn parse_event_category(s: &str) -> crate::time_intel::EventCategory {
    match s.to_lowercase().as_str() {
        "maintenance" => crate::time_intel::EventCategory::Maintenance,
        "backup" => crate::time_intel::EventCategory::Backup,
        "security" => crate::time_intel::EventCategory::Security,
        "personal" => crate::time_intel::EventCategory::Personal,
        "monitoring" => crate::time_intel::EventCategory::Monitoring,
        "cleanup" => crate::time_intel::EventCategory::Cleanup,
        "update" => crate::time_intel::EventCategory::Update,
        other => crate::time_intel::EventCategory::Custom(other.to_string()),
    }
}

async fn time_event_create(Json(req): Json<CreateTimeEventReq>) -> Json<serde_json::Value> {
    let when = match req.when.parse::<chrono::DateTime<chrono::Utc>>() {
        Ok(dt) => dt,
        Err(e) => return Json(serde_json::json!({"ok": false, "error": format!("Invalid datetime: {}", e)})),
    };
    let event = crate::time_intel::create_event(
        &req.name,
        &req.description,
        when,
        parse_event_category(&req.category),
        req.auto_execute,
        req.action,
    ).await;
    Json(serde_json::json!({"ok": true, "event": event}))
}

async fn time_event_delete(Path(id): Path<String>) -> Json<serde_json::Value> {
    let removed = crate::time_intel::remove_event(&id).await;
    Json(serde_json::json!({"ok": removed, "id": id}))
}

async fn time_patterns_list() -> Json<serde_json::Value> {
    let patterns = crate::time_intel::get_patterns().await;
    Json(serde_json::json!({"ok": true, "count": patterns.len(), "patterns": patterns}))
}

#[derive(Deserialize)]
struct CreatePatternReq {
    name: String,
    description: String,
    schedule_type: String,
    #[serde(default)]
    seconds: Option<u64>,
    #[serde(default)]
    hour: Option<u32>,
    #[serde(default)]
    minute: Option<u32>,
    #[serde(default)]
    day_of_week: Option<String>,
    #[serde(default)]
    day_of_month: Option<u32>,
    #[serde(default)]
    nth: Option<u32>,
    action: String,
    #[serde(default = "default_event_category")]
    category: String,
    #[serde(default)]
    auto_execute: bool,
}

fn parse_weekday(s: &str) -> chrono::Weekday {
    match s.to_lowercase().as_str() {
        "mon" | "monday" => chrono::Weekday::Mon,
        "tue" | "tuesday" => chrono::Weekday::Tue,
        "wed" | "wednesday" => chrono::Weekday::Wed,
        "thu" | "thursday" => chrono::Weekday::Thu,
        "fri" | "friday" => chrono::Weekday::Fri,
        "sat" | "saturday" => chrono::Weekday::Sat,
        _ => chrono::Weekday::Sun,
    }
}

async fn time_pattern_create(Json(req): Json<CreatePatternReq>) -> Json<serde_json::Value> {
    let schedule = match req.schedule_type.to_lowercase().as_str() {
        "interval" => {
            let secs = req.seconds.unwrap_or(3600);
            crate::time_intel::Schedule::Interval { seconds: secs }
        }
        "daily" => {
            crate::time_intel::Schedule::Daily {
                hour: req.hour.unwrap_or(3),
                minute: req.minute.unwrap_or(0),
            }
        }
        "weekly" => {
            let day = req.day_of_week.as_deref().map(parse_weekday).unwrap_or(chrono::Weekday::Sun);
            crate::time_intel::Schedule::Weekly {
                day,
                hour: req.hour.unwrap_or(2),
                minute: req.minute.unwrap_or(0),
            }
        }
        "monthly" => {
            crate::time_intel::Schedule::Monthly {
                day_of_month: req.day_of_month.unwrap_or(1),
                hour: req.hour.unwrap_or(4),
                minute: req.minute.unwrap_or(0),
            }
        }
        "nth_weekday" => {
            let day = req.day_of_week.as_deref().map(parse_weekday).unwrap_or(chrono::Weekday::Tue);
            crate::time_intel::Schedule::NthWeekday {
                nth: req.nth.unwrap_or(2),
                day,
                hour: req.hour.unwrap_or(10),
                minute: req.minute.unwrap_or(0),
            }
        }
        _ => {
            return Json(serde_json::json!({"ok": false, "error": "Invalid schedule_type. Use: interval, daily, weekly, monthly, nth_weekday"}));
        }
    };

    let pattern = crate::time_intel::create_pattern(
        &req.name,
        &req.description,
        schedule,
        &req.action,
        parse_event_category(&req.category),
        req.auto_execute,
    ).await;
    Json(serde_json::json!({"ok": true, "pattern": pattern}))
}

async fn time_pattern_delete(Path(id): Path<String>) -> Json<serde_json::Value> {
    let removed = crate::time_intel::remove_pattern(&id).await;
    Json(serde_json::json!({"ok": removed, "id": id}))
}

async fn time_deadlines_list() -> Json<serde_json::Value> {
    let deadlines = crate::time_intel::get_deadlines().await;
    Json(serde_json::json!({"ok": true, "count": deadlines.len(), "deadlines": deadlines}))
}

#[derive(Deserialize)]
struct CreateDeadlineReq {
    name: String,
    description: String,
    due: String,
    #[serde(default = "default_event_category")]
    category: String,
    #[serde(default = "default_warning_days")]
    warning_days: Vec<u32>,
}
fn default_warning_days() -> Vec<u32> { vec![30, 14, 7, 1] }

async fn time_deadline_create(Json(req): Json<CreateDeadlineReq>) -> Json<serde_json::Value> {
    let due = match req.due.parse::<chrono::DateTime<chrono::Utc>>() {
        Ok(dt) => dt,
        Err(e) => return Json(serde_json::json!({"ok": false, "error": format!("Invalid datetime: {}", e)})),
    };
    let deadline = crate::time_intel::create_deadline(
        &req.name,
        &req.description,
        due,
        parse_event_category(&req.category),
        req.warning_days,
    ).await;
    Json(serde_json::json!({"ok": true, "deadline": deadline}))
}

async fn time_due_items() -> Json<serde_json::Value> {
    let due = crate::time_intel::get_due_items().await;
    Json(serde_json::json!({"ok": true, "count": due.len(), "due": due}))
}

async fn time_summary() -> Json<serde_json::Value> {
    let summary = crate::time_intel::time_summary().await;
    Json(serde_json::json!({"ok": true, "summary": summary}))
}



// ─── Personality API ───

async fn personality_get() -> Json<serde_json::Value> {
    let profile = crate::personality::get().await;
    Json(serde_json::json!({"ok": true, "personality": profile}))
}

async fn personality_update(Json(patch): Json<crate::personality::PersonalityPatch>) -> Json<serde_json::Value> {
    crate::personality::update(patch).await;
    let profile = crate::personality::get().await;
    Json(serde_json::json!({"ok": true, "personality": profile}))
}

async fn personality_prompt() -> Json<serde_json::Value> {
    let prompt = crate::personality::system_prompt_fragment().await;
    Json(serde_json::json!({"ok": true, "prompt_fragment": prompt}))
}


// ─── Reflection API ───

async fn reflection_today() -> Json<serde_json::Value> {
    let today = crate::reflection::get_today().await;
    let stats = crate::reflection::get_stats().await;
    Json(serde_json::json!({"ok": true, "reflections": today, "stats": stats}))
}

#[derive(Deserialize)]
struct ReflectionHistoryQuery {
    #[serde(default = "default_history_limit")]
    limit: usize,
}
fn default_history_limit() -> usize { 30 }

async fn reflection_history(Query(params): Query<ReflectionHistoryQuery>) -> Json<serde_json::Value> {
    let history = crate::reflection::get_history(params.limit).await;
    Json(serde_json::json!({"ok": true, "count": history.len(), "history": history}))
}

async fn reflection_trigger() -> Json<serde_json::Value> {
    let reflection = crate::reflection::on_demand().await;
    Json(serde_json::json!({"ok": true, "reflection": reflection}))
}



// ─── Curiosity API ───

async fn curiosity_status() -> Json<serde_json::Value> {
    let state = crate::curiosity::get_state().await;
    Json(serde_json::json!({"ok": true, "curiosity": state}))
}

#[derive(Deserialize)]
struct DiscoveryQuery {
    #[serde(default = "default_disc_limit")]
    limit: usize,
}
fn default_disc_limit() -> usize { 20 }

async fn curiosity_discoveries(Query(params): Query<DiscoveryQuery>) -> Json<serde_json::Value> {
    let discoveries = crate::curiosity::get_discoveries(params.limit).await;
    Json(serde_json::json!({"ok": true, "count": discoveries.len(), "discoveries": discoveries}))
}

async fn curiosity_config_update(Json(config): Json<crate::curiosity::CuriosityConfig>) -> Json<serde_json::Value> {
    crate::curiosity::update_config(config).await;
    Json(serde_json::json!({"ok": true, "message": "Curiosity config updated"}))
}

async fn curiosity_explore() -> Json<serde_json::Value> {
    crate::curiosity::build_exploration_queue().await;
    let state = crate::curiosity::get_state().await;
    Json(serde_json::json!({
        "ok": true,
        "message": "Exploration triggered",
        "queue_size": state.exploration_queue.len(),
        "queue": state.exploration_queue,
    }))
}


// ─── Distillation API ───

async fn distillation_list() -> Json<serde_json::Value> {
    let docs = crate::distillation::list_documents().await;
    let stats = crate::distillation::get_stats().await;
    Json(serde_json::json!({"ok": true, "count": docs.len(), "documents": docs, "stats": stats}))
}

async fn distillation_get(Path(id): Path<String>) -> Json<serde_json::Value> {
    match crate::distillation::get_document(&id).await {
        Some(doc) => Json(serde_json::json!({"ok": true, "document": doc})),
        None => Json(serde_json::json!({"ok": false, "error": "Document not found"})),
    }
}

async fn distillation_trigger() -> Json<serde_json::Value> {
    let docs = crate::distillation::distill().await;
    Json(serde_json::json!({
        "ok": true,
        "message": format!("{} documents distilled", docs.len()),
        "new_documents": docs,
    }))
}

async fn distillation_search(Query(params): Query<HashMap<String, String>>) -> Json<serde_json::Value> {
    let q = params.get("q").cloned().unwrap_or_default();
    if q.is_empty() {
        return Json(serde_json::json!({"ok": false, "error": "Missing 'q' query parameter"}));
    }
    let results = crate::distillation::search(&q).await;
    Json(serde_json::json!({"ok": true, "query": q, "count": results.len(), "results": results}))
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
        // Tool Evolution
        .route("/evolve/tool", post(evolve_tool_handler))
        .route("/evolve/tools", get(list_evolved_tools))
        // Configuration
        .route("/config", get(config_dashboard))
        .route("/config/agent", get(config_agent))
        .route("/config/setup", get(config_setup_status))
        .route("/config/preferences", post(config_update_preferences))
        .route("/config/secret", post(config_set_secret))
        .route("/config/:service", get(config_get_service).post(config_update_service))
        .route("/config/test/:service", post(config_test_service))
        // Consciousness
        .route("/consciousness", get(consciousness_status))
        .route("/consciousness/summary", get(consciousness_summary))
        // Goals
        .route("/goals", get(goals_list).post(goals_create))
        .route("/goals/:id", get(goals_get).delete(goals_delete))
        .route("/goals/:id/subgoal", post(goals_add_subgoal))
        .route("/goals/:id/contribute", post(goals_contribute))
        // World Model
        .route("/world", get(world_full))
        .route("/world/summary", get(world_summary))
        .route("/world/machines", get(world_machines_list).post(world_machine_upsert))
        .route("/world/machines/:id", get(world_machine_get).delete(world_machine_delete))
        .route("/world/services", get(world_services_list).post(world_service_upsert))
        .route("/world/topology", get(world_topology))
        .route("/world/dependencies/:id", get(world_dependencies))
        .route("/world/impact/:id", get(world_impact))
        .route("/world/diff", get(world_diff))
        .route("/world/edges", post(world_add_edge))
        .route("/world/scan", post(world_scan))
        // Time Intelligence
        .route("/time", get(time_awareness))
        .route("/time/summary", get(time_summary))
        .route("/time/events", get(time_events_list).post(time_event_create))
        .route("/time/events/:id", delete(time_event_delete))
        .route("/time/patterns", get(time_patterns_list).post(time_pattern_create))
        .route("/time/patterns/:id", delete(time_pattern_delete))
        .route("/time/deadlines", get(time_deadlines_list).post(time_deadline_create))
        .route("/time/due", get(time_due_items))
        // Personality
        .route("/personality", get(personality_get).post(personality_update))
        .route("/personality/prompt", get(personality_prompt))
        // Reflection
        .route("/reflection/today", get(reflection_today))
        .route("/reflection/history", get(reflection_history))
        .route("/reflection/trigger", post(reflection_trigger))
        // Curiosity
        .route("/curiosity", get(curiosity_status))
        .route("/curiosity/discoveries", get(curiosity_discoveries))
        .route("/curiosity/config", post(curiosity_config_update))
        .route("/curiosity/explore", post(curiosity_explore))
        // Distillation
        .route("/distillation", get(distillation_list))
        .route("/distillation/search", get(distillation_search))
        .route("/distillation/trigger", post(distillation_trigger))
        .route("/distillation/:id", get(distillation_get))
        // Middleware
        .layer(axum::middleware::from_fn(crate::auth::auth_middleware))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
}
