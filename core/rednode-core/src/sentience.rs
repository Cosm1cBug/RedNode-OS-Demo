// RedNode-OS – Sentience Engine
// The computer becomes the intelligence.
//
// Systems-level sentience:
// - Self-model – knows its own state, agents, resources, goals
// - Homeostatic drives – Security, Integrity, Knowledge, Energy, Availability
// - Introspection loop – 1Hz – continuous (drives broadcast every 5s)
// - Goal generator – autonomous intentions from drives
// - Memory consolidation – periodic "dream" – episodic → long-term
// - Self-healing – detect → isolate → patch → verify

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::Duration;
use chrono::Timelike;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfModel {
    pub node_id: String,
    pub boot_ts: chrono::DateTime<chrono::Utc>,
    pub uptime_secs: u64,
    pub agents: Vec<AgentState>,
    pub resources: ResourceState,
    pub drives: Drives,
    pub goals: Vec<Goal>,
    pub goals_executed: u64,
    pub last_introspection: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub name: String,
    pub status: String,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    pub tasks_completed: u64,
}

impl AgentState {
    /// An agent is considered alive if its last heartbeat was within 45 seconds.
    /// Agents send heartbeats every 15s, so 3 missed = stale.
    pub fn is_alive(&self) -> bool {
        let age = chrono::Utc::now() - self.last_heartbeat;
        age.num_seconds() < 45
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceState {
    pub cpu_percent: f32,
    pub mem_used_mb: u64,
    pub mem_total_mb: u64,
    pub disk_used_gb: u64,
    pub disk_total_gb: u64,
    pub disk_used_pct: f32,
    pub load_avg: f32,
    pub temp_c: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Drives {
    /// Security – 0.0 = compromised, 1.0 = fully hardened
    pub security: f32,
    /// Integrity – system health, services up, disks healthy
    pub integrity: f32,
    /// Knowledge – RAG coverage, memory freshness
    pub knowledge: f32,
    /// Energy – battery / power / UPS – 1.0 on AC
    pub energy: f32,
    /// Availability – can serve intentions?
    pub availability: f32,
}

impl Default for Drives {
    fn default() -> Self {
        Self { security: 0.9, integrity: 0.9, knowledge: 0.7, energy: 1.0, availability: 1.0 }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Goal {
    pub id: String,
    pub drive: String,
    pub description: String,
    pub priority: f32,
    pub status: String, // "pending", "executing", "completed", "failed"
    pub created_at: chrono::DateTime<chrono::Utc>,
}

// ─── All known agents (including new infrastructure agents) ───
const ALL_AGENTS: &[&str] = &[
    "system", "security", "coding", "research", "automation", "network",
    "infra", "storage", "surveillance", "comms",
    "productivity", "media", "home", "browser", "social",
];

pub struct SentienceEngine {
    model: Arc<RwLock<SelfModel>>,
    boot_ts: chrono::DateTime<chrono::Utc>,
}

impl SentienceEngine {
    pub fn new(node_id: String) -> Self {
        let now = chrono::Utc::now();
        let agents: Vec<AgentState> = ALL_AGENTS
            .iter()
            .map(|name| AgentState {
                name: name.to_string(),
                status: "starting".into(),
                last_heartbeat: now,
                tasks_completed: 0,
            })
            .collect();

        let model = SelfModel {
            node_id,
            boot_ts: now,
            uptime_secs: 0,
            agents,
            resources: ResourceState::default(),
            drives: Drives::default(),
            goals: vec![],
            goals_executed: 0,
            last_introspection: now,
        };
        Self {
            model: Arc::new(RwLock::new(model)),
            boot_ts: now,
        }
    }

    pub async fn start(self: Arc<Self>) {
        tracing::info!("🧠 Sentience Engine starting – self-aware loop 1Hz");

        // ── Heartbeat listener — track agent liveness via NATS ──
        let s = self.clone();
        tokio::spawn(async move {
            s.listen_heartbeats().await;
        });

        // ── Introspection loop — 1 Hz (drives broadcast every 5s) ──
        let s = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            let mut broadcast_counter: u64 = 0;
            loop {
                interval.tick().await;
                s.introspect().await;
                broadcast_counter += 1;
                // Broadcast drive snapshot every 5 ticks (5 seconds)
                if broadcast_counter % 5 == 0 {
                    let model = s.model.read().await;
                    crate::events::emit_drives(&model.drives);
                    // Also publish to NATS for other components
                    let _ = crate::bus::publish(
                        "rednode.sentience.drives",
                        serde_json::to_value(&model.drives).unwrap_or_default(),
                    ).await;
                }
            }
        });

        // ── Goal generator — every 30s ──
        let s = self.clone();
        tokio::spawn(async move {
            // Wait 60s after boot before first goal generation
            // (let everything stabilize)
            tokio::time::sleep(Duration::from_secs(60)).await;
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            loop {
                interval.tick().await;
                s.generate_goals().await;
            }
        });

        // ── Memory consolidation — every 5 minutes ──
        let s = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300));
            loop {
                interval.tick().await;
                s.consolidate_memory().await;
            }
        });
    }

    // ── Heartbeat listener ──

    async fn listen_heartbeats(&self) {
        // Subscribe to all agent heartbeats via NATS
        let sub = match crate::bus::subscribe("rednode.agent.*.heartbeat").await {
            Some(s) => s,
            None => {
                tracing::warn!("Sentience: NATS not available — agent heartbeats won't be tracked");
                return;
            }
        };

        use futures::StreamExt;
        let mut sub = sub;
        while let Some(msg) = sub.next().await {
            // Parse heartbeat
            if let Ok(hb) = serde_json::from_slice::<serde_json::Value>(&msg.payload) {
                let agent_name = hb["agent"].as_str().unwrap_or("");
                if !agent_name.is_empty() {
                    let mut model = self.model.write().await;
                    if let Some(a) = model.agents.iter_mut().find(|x| x.name == agent_name) {
                        a.status = "online".into();
                        a.last_heartbeat = chrono::Utc::now();
                    } else {
                        // New agent we didn't know about
                        model.agents.push(AgentState {
                            name: agent_name.to_string(),
                            status: "online".into(),
                            last_heartbeat: chrono::Utc::now(),
                            tasks_completed: 0,
                        });
                    }
                    crate::events::emit_agent_heartbeat(agent_name, "online");
                }
            }
        }
    }

    // ── Introspection — runs every 1 second ──

    async fn introspect(&self) {
        let mut model = self.model.write().await;

        // Update uptime
        model.uptime_secs = (chrono::Utc::now() - self.boot_ts).num_seconds().max(0) as u64;

        // Update resource state (real CPU, RAM, disk)
        model.resources = sample_resources();

        // ── Security drive ──
        // Based on: unacknowledged security events in last hour
        let sec_events = crate::memory::list_security_events(100).await.unwrap_or_default();
        let unacked_recent: usize = sec_events
            .iter()
            .filter(|e| {
                let age = chrono::Utc::now() - e.ts;
                age.num_hours() < 1 && e.acknowledged != Some(true)
            })
            .count();
        // Each unacked event in last hour reduces security by 0.1, floor at 0.3
        model.drives.security = (1.0 - unacked_recent as f32 * 0.1).max(0.3);

        // ── Integrity drive ──
        // Based on: agents alive + disk health + resource headroom
        let total_agents = model.agents.len() as f32;
        let alive_agents = model.agents.iter().filter(|a| a.is_alive()).count() as f32;
        let agent_ratio = if total_agents > 0.0 { alive_agents / total_agents } else { 0.5 };

        // Mark stale agents
        for a in model.agents.iter_mut() {
            if !a.is_alive() && a.status == "online" {
                a.status = "stale".into();
                tracing::warn!(agent=%a.name, "agent heartbeat stale — marking offline");
                crate::events::emit_agent_heartbeat(&a.name, "stale");
            }
        }

        // Disk pressure check
        let disk_pressure = if model.resources.disk_total_gb > 0 {
            model.resources.disk_used_gb as f32 / model.resources.disk_total_gb as f32
        } else {
            0.5
        };
        let disk_health = if disk_pressure > 0.90 { 0.3 } else if disk_pressure > 0.80 { 0.6 } else { 1.0 };

        // CPU pressure check
        let cpu_health = if model.resources.cpu_percent > 95.0 { 0.5 } else if model.resources.cpu_percent > 85.0 { 0.7 } else { 1.0 };

        model.drives.integrity = (agent_ratio * 0.5 + disk_health * 0.3 + cpu_health * 0.2).min(1.0);

        // ── Knowledge drive ──
        // Based on: do we have a vector DB with documents?
        let rag_results = crate::memory::rag_query("rednode", 1).await.unwrap_or_default();
        let has_real_knowledge = rag_results.iter().any(|r| {
            r.score > 0.6 && !r.metadata.get("fallback").and_then(|v| v.as_bool()).unwrap_or(false)
        });
        if has_real_knowledge {
            model.drives.knowledge = (model.drives.knowledge + 0.01).min(1.0);
        } else {
            model.drives.knowledge = (model.drives.knowledge - 0.005).max(0.4);
        }

        // ── Energy drive ──
        // Read /sys/class/power_supply if available (laptop/UPS)
        model.drives.energy = read_power_status();

        // ── Availability drive ──
        // Can we serve intentions? Check Postgres + NATS + Ollama
        let pg_ok = crate::memory::pool().is_some();
        let nats_ok = crate::bus::get_client().is_some();
        // Quick Ollama check — real HTTP ping with 2s timeout
        let ollama_url = std::env::var("OLLAMA_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:11434".into());
        let ollama_ok = match reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
        {
            Ok(client) => client
                .get(format!("{}/api/tags", ollama_url))
                .send()
                .await
                .map(|r| r.status().is_success())
                .unwrap_or(false),
            Err(_) => false,
        };

        let infra_score = [pg_ok, nats_ok, ollama_ok]
            .iter()
            .filter(|&&x| x)
            .count() as f32
            / 3.0;
        model.drives.availability = (agent_ratio * 0.4 + infra_score * 0.6).min(1.0);

        model.last_introspection = chrono::Utc::now();
    }

    // ── Goal generator — runs every 30 seconds ──

    async fn generate_goals(&self) {
        let model = self.model.read().await;
        let drives = model.drives.clone();

        // Don't generate goals if we already have pending ones for the same drive
        let pending_drives: Vec<String> = model
            .goals
            .iter()
            .filter(|g| g.status == "pending" || g.status == "executing")
            .map(|g| g.drive.clone())
            .collect();

        drop(model);

        let mut new_goals = Vec::new();

        // Security drive low → triage
        if drives.security < 0.8 && !pending_drives.contains(&"security".to_string()) {
            new_goals.push(Goal {
                id: uuid::Uuid::new_v4().to_string(),
                drive: "security".into(),
                description: "Run security triage – check recent security events and system logs".into(),
                priority: 1.0 - drives.security,
                status: "pending".into(),
                created_at: chrono::Utc::now(),
            });
        }

        // Integrity drive low → health check
        if drives.integrity < 0.8 && !pending_drives.contains(&"integrity".to_string()) {
            new_goals.push(Goal {
                id: uuid::Uuid::new_v4().to_string(),
                drive: "integrity".into(),
                description: "System health check – verify agents, disk space, and service status".into(),
                priority: 1.0 - drives.integrity,
                status: "pending".into(),
                created_at: chrono::Utc::now(),
            });
        }

        // Knowledge drive low → consolidate
        if drives.knowledge < 0.5 && !pending_drives.contains(&"knowledge".to_string()) {
            new_goals.push(Goal {
                id: uuid::Uuid::new_v4().to_string(),
                drive: "knowledge".into(),
                description: "Knowledge consolidation – ingest recent audit log summaries into memory".into(),
                priority: 0.5,
                status: "pending".into(),
                created_at: chrono::Utc::now(),
            });
        }

        // Availability drive low → diagnostic
        if drives.availability < 0.7 && !pending_drives.contains(&"availability".to_string()) {
            new_goals.push(Goal {
                id: uuid::Uuid::new_v4().to_string(),
                drive: "availability".into(),
                description: "Availability check – verify Postgres, NATS, Ollama, and agent connectivity".into(),
                priority: 1.0 - drives.availability,
                status: "pending".into(),
                created_at: chrono::Utc::now(),
            });
        }

        if new_goals.is_empty() {
            // ── Predictive Intent — Time-based pattern suggestions ──
            // Analyze what the user typically does at this hour.
            // If they always check cameras at 10pm, proactively suggest it.
            let hour = chrono::Local::now().hour();

            if let Some(pool) = crate::memory::pool() {
                // Query audit log for intents at this hour (±1 hour) from past 7 days
                let pattern_rows: Vec<(String, i64)> = sqlx::query_as(
                    "SELECT args->>'intent' as intent, COUNT(*) as cnt \
                     FROM audit_log \
                     WHERE action = 'intent' \
                       AND args->>'intent' IS NOT NULL \
                       AND ts > now() - interval '7 days' \
                       AND EXTRACT(HOUR FROM ts) BETWEEN $1 AND $2 \
                     GROUP BY args->>'intent' \
                     HAVING COUNT(*) >= 3 \
                     ORDER BY cnt DESC \
                     LIMIT 3"
                )
                .bind((hour as i32 - 1).max(0))
                .bind((hour as i32 + 1).min(23))
                .fetch_all(pool)
                .await
                .unwrap_or_default();

                for (intent, count) in &pattern_rows {
                    if !intent.is_empty() && !pending_drives.contains(&"prediction".to_string()) {
                        tracing::info!(
                            intent = %intent, count = count, hour = hour,
                            "predictive: you usually do '{}' around this time ({} times in past week)",
                            intent, count
                        );

                        new_goals.push(Goal {
                            id: uuid::Uuid::new_v4().to_string(),
                            drive: "prediction".into(),
                            description: intent.clone(),
                            priority: 0.3, // low priority — suggestion, not urgent
                            status: "pending".into(),
                            created_at: chrono::Utc::now(),
                        });

                        crate::events::emit(serde_json::json!({
                            "type": "predictive_intent",
                            "intent": intent,
                            "frequency": count,
                            "hour": hour,
                            "ts": chrono::Utc::now().to_rfc3339()
                        }));

                        break; // one prediction per cycle is enough
                    }
                }
            }
        }

        if new_goals.is_empty() {
            return;
        }

        // Execute each goal
        let mut model = self.model.write().await;
        tracing::info!(count = new_goals.len(), "sentience: autonomous goals generated");

        for mut goal in new_goals {
            tracing::info!(
                drive = %goal.drive,
                priority = goal.priority,
                "sentience goal: {}",
                goal.description
            );

            // Emit event BEFORE execution so dashboard shows it immediately
            crate::events::emit_goal(&goal, false);

            // Execute the goal through the coordinator
            // This uses the LLM planner → agent dispatch → sandboxed execution
            // The coordinator handles risk assessment and approval gates
            goal.status = "executing".into();

            // Drop the write lock before the async coordinator call
            let description = goal.description.clone();
            let goal_id = goal.id.clone();
            model.goals.push(goal);
            drop(model);

            let (_, results) = crate::coordinator::coordinate(&description, "sentience").await;

            // Re-acquire lock and update goal status
            model = self.model.write().await;
            if let Some(g) = model.goals.iter_mut().find(|g| g.id == goal_id) {
                let all_ok = results.iter().all(|r| {
                    r.get("status")
                        .and_then(|s| s.as_str())
                        .map(|s| s == "executed" || s == "needs_approval")
                        .unwrap_or(false)
                });

                let failed_tools: Vec<String> = results.iter()
                    .filter(|r| r.get("status").and_then(|s| s.as_str()) == Some("failed"))
                    .filter_map(|r| r.get("tool").and_then(|t| t.as_str()).map(String::from))
                    .collect();

                g.status = if all_ok { "completed" } else { "failed" }.into();

                crate::events::emit_goal(g, true);
                tracing::info!(
                    goal_id = %g.id,
                    status = %g.status,
                    "sentience goal {} — {} results",
                    g.status,
                    results.len()
                );

                // ── Agent Self-Debugging ──
                // If a goal failed, analyze WHY and create a diagnostic report.
                // This helps RedNode learn from failures and avoid repeating them.
                if !all_ok && !failed_tools.is_empty() {
                    let errors: Vec<String> = results.iter()
                        .filter_map(|r| {
                            let tool = r.get("tool").and_then(|t| t.as_str()).unwrap_or("-");
                            let error = r.get("result")
                                .and_then(|res| res.get("error"))
                                .and_then(|e| e.as_str())
                                .or_else(|| r.get("error").and_then(|e| e.as_str()));
                            error.map(|e| format!("{}: {}", tool, e))
                        })
                        .collect();

                    // Classify failure type
                    let error_text = errors.join(" ").to_lowercase();
                    let failure_type = if error_text.contains("timeout") || error_text.contains("timed out") {
                        "timeout"
                    } else if error_text.contains("permission") || error_text.contains("denied") || error_text.contains("approval") {
                        "permission"
                    } else if error_text.contains("not found") || error_text.contains("no such") {
                        "not_found"
                    } else if error_text.contains("connection") || error_text.contains("refused") || error_text.contains("unreachable") {
                        "connectivity"
                    } else {
                        "unknown"
                    };

                    let diagnostic = format!(
                        "Goal failed — introspection report:\n\
                         Goal: {}\n\
                         Drive: {}\n\
                         Failure type: {}\n\
                         Failed tools: {}\n\
                         Errors:\n{}\n\
                         Recommendation: {}",
                        g.description,
                        g.drive,
                        failure_type,
                        failed_tools.join(", "),
                        errors.join("\n"),
                        match failure_type {
                            "timeout" => "Service may be overloaded or down. Check resource usage. Consider increasing timeout.",
                            "permission" => "Tool requires higher risk approval. User must approve via dashboard or mobile.",
                            "not_found" => "Target resource doesn't exist. Check if service is running and path is correct.",
                            "connectivity" => "Network issue. Check if target service (NATS/Postgres/Ollama/Pi-hole/TrueNAS) is reachable.",
                            _ => "Unknown failure. Review the error details and check service logs.",
                        }
                    );

                    tracing::warn!("sentience introspection:\n{}", diagnostic);

                    // Ingest the diagnostic into memory so future planning can avoid the same failure
                    drop(model);
                    let _ = crate::memory::ingest_document("sentience/introspection", &diagnostic).await;

                    crate::events::emit(serde_json::json!({
                        "type": "sentience_introspection",
                        "goal_id": goal_id,
                        "failure_type": failure_type,
                        "failed_tools": failed_tools,
                        "errors": errors,
                        "ts": chrono::Utc::now().to_rfc3339()
                    }));

                    model = self.model.write().await;
                }
            }
            model.goals_executed += 1;
        }

        // Prune old completed/failed goals (keep last 50)
        if model.goals.len() > 50 {
            // Keep pending/executing, prune oldest completed/failed
            let mut keep = Vec::new();
            let mut archive = Vec::new();
            for g in model.goals.drain(..) {
                if g.status == "pending" || g.status == "executing" {
                    keep.push(g);
                } else {
                    archive.push(g);
                }
            }
            // Keep last 30 archived goals
            if archive.len() > 30 {
                archive.drain(0..archive.len() - 30);
            }
            keep.extend(archive);
            model.goals = keep;
        }
    }

    // ── Memory consolidation — runs every 5 minutes ──

    async fn consolidate_memory(&self) {
        tracing::info!("sentience: memory consolidation + self-improvement cycle starting");

        // ── Phase 1: Audit summarization (existing) ──
        let recent_audit = crate::memory::get_audit(50).await.unwrap_or_default();
        if !recent_audit.is_empty() {
            let summary = recent_audit
                .iter()
                .map(|e| {
                    format!(
                        "{}: {} {} {} (risk: {}, ok: {})",
                        e.ts.format("%H:%M"),
                        e.actor,
                        e.action,
                        e.tool.as_deref().unwrap_or("-"),
                        e.risk.as_deref().unwrap_or("-"),
                        e.approved.unwrap_or(false)
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");

            let content = format!(
                "Audit summary ({} to {}): {} actions\n{}",
                recent_audit.last().map(|e| e.ts.format("%H:%M").to_string()).unwrap_or_default(),
                recent_audit.first().map(|e| e.ts.format("%H:%M").to_string()).unwrap_or_default(),
                recent_audit.len(),
                summary
            );
            let _ = crate::memory::ingest_document("sentience/consolidation", &content).await;
        }

        // ── Phase 2: Security event summarization (existing) ──
        let recent_security = crate::memory::list_security_events(20).await.unwrap_or_default();
        if !recent_security.is_empty() {
            let sec_summary = recent_security
                .iter()
                .map(|e| format!("[{}] {}: {}", e.severity, e.source, e.summary))
                .collect::<Vec<_>>()
                .join("\n");

            let content = format!(
                "Security summary: {} events\n{}",
                recent_security.len(),
                sec_summary
            );
            let _ = crate::memory::ingest_document("sentience/security-digest", &content).await;
        }

        // ── Phase 3: Self-Improvement — Pattern Analysis ──
        // Analyze the last 50 audit entries to find:
        //   - Most frequently used tools → optimize their paths
        //   - Tools that frequently fail → flag for investigation
        //   - Common intent patterns → suggest new workflows
        //   - Approval bottlenecks → suggest risk level adjustments

        if recent_audit.len() >= 10 {
            let mut tool_counts: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
            let mut tool_failures: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
            let mut intent_patterns: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
            let mut approval_count: u32 = 0;

            for entry in &recent_audit {
                if let Some(tool) = &entry.tool {
                    *tool_counts.entry(tool.clone()).or_insert(0) += 1;
                    if entry.approved == Some(false) || entry.action == "tool_exec_failed" {
                        *tool_failures.entry(tool.clone()).or_insert(0) += 1;
                    }
                }
                if entry.action == "intent" {
                    if let Some(args) = &entry.args {
                        if let Some(intent) = args.get("intent").and_then(|v| v.as_str()) {
                            // Extract first 3 words as pattern key
                            let pattern: String = intent.split_whitespace().take(3).collect::<Vec<_>>().join(" ");
                            *intent_patterns.entry(pattern).or_insert(0) += 1;
                        }
                    }
                }
                if entry.action == "approval_created" || entry.result.as_deref() == Some("needs_approval") {
                    approval_count += 1;
                }
            }

            // Generate self-improvement insights
            let mut insights: Vec<String> = Vec::new();

            // Top tools
            let mut sorted_tools: Vec<_> = tool_counts.iter().collect();
            sorted_tools.sort_by(|a, b| b.1.cmp(a.1));
            if let Some((top_tool, count)) = sorted_tools.first() {
                insights.push(format!("Most used tool: {} ({} times)", top_tool, count));
            }

            // Failing tools
            for (tool, fail_count) in &tool_failures {
                let total = tool_counts.get(tool).unwrap_or(&1);
                let fail_rate = *fail_count as f32 / *total as f32;
                if fail_rate > 0.3 && *fail_count >= 2 {
                    insights.push(format!(
                        "⚠️ Tool '{}' failing {:.0}% of the time ({}/{}) — investigate",
                        tool, fail_rate * 100.0, fail_count, total
                    ));
                }
            }

            // Repeated intent patterns → suggest workflows
            let mut sorted_patterns: Vec<_> = intent_patterns.iter().collect();
            sorted_patterns.sort_by(|a, b| b.1.cmp(a.1));
            for (pattern, count) in sorted_patterns.iter().take(3) {
                if **count >= 3 {
                    insights.push(format!(
                        "💡 Repeated intent pattern: '{}...' ({} times) — consider creating a workflow",
                        pattern, count
                    ));
                }
            }

            // Approval bottleneck
            if approval_count > 5 {
                insights.push(format!(
                    "🔒 {} actions needed approval in recent history — review if any should be downgraded to Medium risk",
                    approval_count
                ));
            }

            if !insights.is_empty() {
                let insight_text = format!(
                    "Self-improvement insights (from {} actions):\n{}",
                    recent_audit.len(),
                    insights.join("\n")
                );
                tracing::info!("sentience self-improvement:\n{}", insight_text);
                let _ = crate::memory::ingest_document("sentience/self-improvement", &insight_text).await;

                crate::events::emit(serde_json::json!({
                    "type": "sentience_self_improvement",
                    "insights": insights,
                    "actions_analyzed": recent_audit.len(),
                    "ts": chrono::Utc::now().to_rfc3339()
                }));
            }
        }

        // ── Phase 4: JUDGE — score recent insights quality ──
        // Intelligence pipeline: RETRIEVE → JUDGE → DISTILL → CONSOLIDATE
        // Judge which insights are actually useful vs noise
        if !recent_audit.is_empty() {
            let total_actions = recent_audit.len();
            let successful = recent_audit.iter().filter(|e| e.approved == Some(true)).count();
            let success_rate = if total_actions > 0 { successful as f32 / total_actions as f32 } else { 0.5 };

            // If success rate is high, knowledge is growing (good insights)
            // If low, knowledge may be degrading (bad patterns being learned)
            if success_rate > 0.7 {
                tracing::info!(rate = success_rate, "JUDGE: high success rate — insights are valuable");
            } else if success_rate < 0.3 {
                tracing::warn!(rate = success_rate, "JUDGE: low success rate — review recent patterns");
            }
        }

        // ── Phase 5: CONSOLIDATE — prevent knowledge decay ──
        // Remove stale/low-quality entries from RAG to prevent "catastrophic forgetting"
        // (keeping too many irrelevant documents dilutes search quality)
        if let Some(pool) = pool() {
            // Count documents in memory
            let doc_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM documents")
                .fetch_one(pool).await.unwrap_or((0,));

            // If we have > 5000 documents, prune the oldest low-relevance ones
            if doc_count.0 > 5000 {
                let pruned = sqlx::query(
                    "DELETE FROM documents WHERE id IN (\
                     SELECT id FROM documents \
                     WHERE source LIKE 'sentience/%' \
                     ORDER BY created_at ASC LIMIT 100)"
                ).execute(pool).await;

                if let Ok(result) = pruned {
                    let count = result.rows_affected();
                    if count > 0 {
                        tracing::info!(pruned = count, "CONSOLIDATE: pruned {} old sentience entries to prevent knowledge decay", count);
                    }
                }
            }
        }

        // ── Phase 5b: Pattern Promotion ──
        // Promote frequently-referenced entities to "established" status
        crate::memory::promote_patterns().await;

        // ── Phase 6: Update drives ──
        let mut model = self.model.write().await;
        model.drives.knowledge = (model.drives.knowledge + 0.05).min(1.0);

        tracing::info!(
            knowledge = model.drives.knowledge,
            "sentience: memory consolidation complete — ingested {} audit + {} security entries",
            recent_audit.len(),
            recent_security.len()
        );

        crate::events::emit(serde_json::json!({
            "type": "sentience_consolidation",
            "audit_entries": recent_audit.len(),
            "security_entries": recent_security.len(),
            "knowledge_drive": model.drives.knowledge,
            "ts": chrono::Utc::now().to_rfc3339()
        }));
    }

    pub async fn get_model(&self) -> SelfModel {
        self.model.read().await.clone()
    }

    pub async fn record_task_completed(&self, agent: &str) {
        let mut model = self.model.write().await;
        if let Some(a) = model.agents.iter_mut().find(|x| x.name == agent) {
            a.tasks_completed += 1;
            a.last_heartbeat = chrono::Utc::now();
            a.status = "online".into();
        }
    }
}

// ─── Resource sampling — real sysinfo + real disk ───

use std::sync::Mutex;
use once_cell::sync::Lazy;

static SYS: Lazy<Mutex<sysinfo::System>> = Lazy::new(|| {
    use sysinfo::System;
    Mutex::new(System::new_all())
});

fn sample_resources() -> ResourceState {
    let (cpu, mem_used, mem_total) = {
        let mut sys = SYS.lock().unwrap();
        sys.refresh_cpu_usage();
        sys.refresh_memory();
        (
            sys.global_cpu_usage(),
            sys.used_memory() / 1024 / 1024,
            sys.total_memory() / 1024 / 1024,
        )
    };

    // Real disk usage via statvfs on /
    let (disk_used_gb, disk_total_gb) = get_disk_usage("/");

    let disk_used_pct = if disk_total_gb > 0 {
        disk_used_gb as f32 / disk_total_gb as f32 * 100.0
    } else {
        0.0
    };

    ResourceState {
        cpu_percent: cpu,
        mem_used_mb: mem_used,
        mem_total_mb: mem_total,
        disk_used_gb,
        disk_total_gb,
        disk_used_pct,
        load_avg: cpu / 100.0 * num_cpus(),
        temp_c: read_cpu_temp(),
    }
}

fn get_disk_usage(path: &str) -> (u64, u64) {
    use sysinfo::Disks;
    let disks = Disks::new_with_refreshed_list();
    for disk in disks.list() {
        let mount = disk.mount_point().to_string_lossy();
        if mount == path || (path == "/" && mount == "/") {
            let total = disk.total_space() / 1_073_741_824; // bytes → GB
            let avail = disk.available_space() / 1_073_741_824;
            return ((total - avail), total);
        }
    }
    // Fallback: check all disks, use the one with largest total
    let disks_list = Disks::new_with_refreshed_list();
    if let Some(disk) = disks_list.list().iter().max_by_key(|d| d.total_space()) {
        let total = disk.total_space() / 1_073_741_824;
        let avail = disk.available_space() / 1_073_741_824;
        return ((total - avail), total);
    }
    (0, 0)
}

fn num_cpus() -> f32 {
    let sys = SYS.lock().unwrap();
    sys.cpus().len() as f32
}

fn read_cpu_temp() -> f32 {
    // Try reading from sysinfo Components
    use sysinfo::Components;
    let components = Components::new_with_refreshed_list();
    for comp in components.iter() {
        let label = comp.label().to_lowercase();
        if label.contains("cpu") || label.contains("core") || label.contains("package") {
            let temp = comp.temperature();
            if temp > 0.0 && temp < 150.0 {
                return temp;
            }
        }
    }
    // Fallback: try /sys/class/thermal
    if let Ok(content) = std::fs::read_to_string("/sys/class/thermal/thermal_zone0/temp") {
        if let Ok(millideg) = content.trim().parse::<f32>() {
            return millideg / 1000.0;
        }
    }
    0.0 // unknown
}

/// Read power supply status. Returns 1.0 for AC/plugged, lower for battery.
fn read_power_status() -> f32 {
    // Check /sys/class/power_supply/*/status
    let power_dir = std::path::Path::new("/sys/class/power_supply");
    if !power_dir.exists() {
        return 1.0; // Desktop, no battery — always full
    }
    if let Ok(entries) = std::fs::read_dir(power_dir) {
        for entry in entries.flatten() {
            let status_path = entry.path().join("status");
            let capacity_path = entry.path().join("capacity");
            if status_path.exists() {
                if let Ok(status) = std::fs::read_to_string(&status_path) {
                    let s = status.trim().to_lowercase();
                    if s == "charging" || s == "full" || s == "not charging" {
                        return 1.0;
                    }
                    if s == "discharging" {
                        // Read battery percentage
                        if let Ok(cap) = std::fs::read_to_string(&capacity_path) {
                            if let Ok(pct) = cap.trim().parse::<f32>() {
                                return pct / 100.0;
                            }
                        }
                        return 0.5;
                    }
                }
            }
        }
    }
    1.0 // Default: assume AC
}

// ─── Global singleton ───

static SENTIENCE: tokio::sync::OnceCell<Arc<SentienceEngine>> = tokio::sync::OnceCell::const_new();

pub async fn init(node_id: String) -> Arc<SentienceEngine> {
    let engine = Arc::new(SentienceEngine::new(node_id));
    let _ = SENTIENCE.set(engine.clone());
    engine.clone().start().await;
    engine
}

pub async fn get() -> Option<Arc<SentienceEngine>> {
    SENTIENCE.get().cloned()
}
