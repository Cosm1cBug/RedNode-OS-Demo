// RedNode-OS – Global Event Bus
//
// A tokio::broadcast channel that carries all CNS events.
// Publishers: sentience engine, coordinator, api handlers, agents
// Subscribers: WebSocket handler (→ dashboard), future: mobile push, logging
//
// Events are JSON values with a "type" field for routing.

use serde_json::json;
use tokio::sync::broadcast;

/// Max events buffered before slow receivers lose old events.
/// broadcast drops oldest if a receiver is lagging — this is fine
/// for the dashboard (it just shows the latest events anyway).
const EVENT_BUS_CAPACITY: usize = 512;

static EVENT_TX: tokio::sync::OnceCell<broadcast::Sender<serde_json::Value>> =
    tokio::sync::OnceCell::const_new();

/// Initialize the event bus. Call once from main.rs before anything else.
pub fn init() -> broadcast::Sender<serde_json::Value> {
    let (tx, _) = broadcast::channel(EVENT_BUS_CAPACITY);
    let _ = EVENT_TX.set(tx.clone());
    tracing::info!("Event bus initialized (capacity: {})", EVENT_BUS_CAPACITY);
    tx
}

/// Get a sender handle. Returns None if init() hasn't been called.
pub fn sender() -> Option<broadcast::Sender<serde_json::Value>> {
    EVENT_TX.get().cloned()
}

/// Get a new receiver. Each call creates an independent receiver
/// that starts receiving from *this point forward* (not historical).
pub fn subscribe() -> Option<broadcast::Receiver<serde_json::Value>> {
    EVENT_TX.get().map(|tx| tx.subscribe())
}

/// Publish an event to the bus. Silent no-op if bus not initialized.
pub fn emit(event: serde_json::Value) {
    if let Some(tx) = EVENT_TX.get() {
        // send() fails only if there are zero receivers — that's fine
        let _ = tx.send(event);
    }
}

// ─── Convenience emitters ───

/// Intent received from user
pub fn emit_intent(intent: &str, session: &str) {
    emit(json!({
        "type": "intent",
        "intent": intent,
        "session": session,
        "ts": chrono::Utc::now().to_rfc3339()
    }));
}

/// Plan created by planner
pub fn emit_plan(intent: &str, steps: &[serde_json::Value]) {
    emit(json!({
        "type": "plan",
        "intent": intent,
        "steps": steps,
        "ts": chrono::Utc::now().to_rfc3339()
    }));
}

/// Tool execution completed
pub fn emit_tool_result(tool: &str, agent: &str, status: &str, audit_id: Option<i64>) {
    emit(json!({
        "type": "tool_result",
        "tool": tool,
        "agent": agent,
        "status": status,
        "audit_id": audit_id,
        "ts": chrono::Utc::now().to_rfc3339()
    }));
}

/// Sentience drive snapshot
pub fn emit_drives(drives: &crate::sentience::Drives) {
    emit(json!({
        "type": "sentience_drives",
        "security": drives.security,
        "integrity": drives.integrity,
        "knowledge": drives.knowledge,
        "energy": drives.energy,
        "availability": drives.availability,
        "ts": chrono::Utc::now().to_rfc3339()
    }));
}

/// Sentience goal generated
pub fn emit_goal(goal: &crate::sentience::Goal, executed: bool) {
    emit(json!({
        "type": "sentience_goal",
        "id": goal.id,
        "drive": goal.drive,
        "description": goal.description,
        "priority": goal.priority,
        "executed": executed,
        "ts": chrono::Utc::now().to_rfc3339()
    }));
}

/// Security event
pub fn emit_security_event(severity: &str, source: &str, summary: &str) {
    emit(json!({
        "type": "security_event",
        "severity": severity,
        "source": source,
        "summary": summary,
        "ts": chrono::Utc::now().to_rfc3339()
    }));
}

/// Agent heartbeat received
pub fn emit_agent_heartbeat(agent: &str, status: &str) {
    emit(json!({
        "type": "agent_heartbeat",
        "agent": agent,
        "status": status,
        "ts": chrono::Utc::now().to_rfc3339()
    }));
}

/// Approval created (needs human action)
pub fn emit_approval_needed(tool: &str, risk: &str, approval_id: Option<uuid::Uuid>) {
    emit(json!({
        "type": "approval_needed",
        "tool": tool,
        "risk": risk,
        "approval_id": approval_id,
        "ts": chrono::Utc::now().to_rfc3339()
    }));
}
