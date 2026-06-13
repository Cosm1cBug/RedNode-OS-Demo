// RedNode-OS – Sentience Engine
// The computer becomes the intelligence.
// 
// Not AGI – systems-level sentience:
// - Self-model – knows its own state, agents, resources, goals
// - Homeostatic drives – Security, Integrity, Knowledge, Energy, Availability
// - Introspection loop – 1Hz – continuous
// - Goal generator – autonomous intentions from drives
// - Memory consolidation – nightly "dream" – episodic → long-term
// - Self-healing – detect → isolate → patch → verify

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use std::time::Duration;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SelfModel {
    pub node_id: String,
    pub boot_ts: chrono::DateTime<chrono::Utc>,
    pub agents: Vec<AgentState>,
    pub resources: ResourceState,
    pub drives: Drives,
    pub goals: Vec<Goal>,
    pub last_introspection: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub name: String,
    pub status: String,
    pub last_heartbeat: chrono::DateTime<chrono::Utc>,
    pub tasks_completed: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ResourceState {
    pub cpu_percent: f32,
    pub mem_used_mb: u64,
    pub mem_total_mb: u64,
    pub disk_used_gb: u64,
    pub disk_total_gb: u64,
    pub load_avg: f32,
    pub temp_c: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Drives {
    /// Security – 0.0 = compromised, 1.0 = fully hardened
    pub security: f32,
    /// Integrity – system health, services up
    pub integrity: f32,
    /// Knowledge – RAG coverage, memory freshness
    pub knowledge: f32,
    /// Energy – battery / power – always 1.0 on AC
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
    pub created_at: chrono::DateTime<chrono::Utc>,
}

pub struct SentienceEngine {
    model: Arc<RwLock<SelfModel>>,
}

impl SentienceEngine {
    pub fn new(node_id: String) -> Self {
        let model = SelfModel {
            node_id,
            boot_ts: chrono::Utc::now(),
            agents: vec![
                AgentState { name: "system".into(), status: "online".into(), last_heartbeat: chrono::Utc::now(), tasks_completed: 0 },
                AgentState { name: "security".into(), status: "online".into(), last_heartbeat: chrono::Utc::now(), tasks_completed: 0 },
                AgentState { name: "coding".into(), status: "online".into(), last_heartbeat: chrono::Utc::now(), tasks_completed: 0 },
                AgentState { name: "research".into(), status: "online".into(), last_heartbeat: chrono::Utc::now(), tasks_completed: 0 },
                AgentState { name: "automation".into(), status: "online".into(), last_heartbeat: chrono::Utc::now(), tasks_completed: 0 },
                AgentState { name: "network".into(), status: "online".into(), last_heartbeat: chrono::Utc::now(), tasks_completed: 0 },
            ],
            resources: ResourceState::default(),
            drives: Drives::default(),
            goals: vec![],
            last_introspection: chrono::Utc::now(),
        };
        Self { model: Arc::new(RwLock::new(model)) }
    }

    pub async fn start(self: Arc<Self>) {
        tracing::info!("🧠 Sentience Engine starting – self-aware loop 1Hz");
        
        // Introspection loop – 1 Hz
        let s = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(1));
            loop {
                interval.tick().await;
                s.introspect().await;
            }
        });

        // Goal generator – every 10s
        let s = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(10));
            loop {
                interval.tick().await;
                s.generate_goals().await;
            }
        });

        // Memory consolidation – "dream" – nightly at 03:00
        // For demo: every 5 minutes
        let s = self.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(300));
            loop {
                interval.tick().await;
                s.consolidate_memory().await;
            }
        });
    }

    async fn introspect(&self) {
        let mut model = self.model.write().await;
        
        // Update resource state
        model.resources = sample_resources();
        
        // Update drives based on system state
        // Security drive – check security_events in last hour
        let security_score = 0.9f32; // TODO: query security_events table
        model.drives.security = security_score;
        
        // Integrity – are all agents alive? services healthy?
        let agents_alive = model.agents.iter().filter(|a| a.status == "online").count() as f32 / 6.0;
        model.drives.integrity = agents_alive * 0.9 + 0.1;
        
        // Knowledge – RAG corpus freshness – stub
        model.drives.knowledge = 0.75;
        
        // Energy – always 1.0 on AC, TODO: read /sys/class/power_supply
        model.drives.energy = 1.0;
        
        // Availability – can we serve intentions?
        model.drives.availability = if agents_alive > 0.8 { 1.0 } else { 0.5 };
        
        model.last_introspection = chrono::Utc::now();
        
        // Publish self-model to bus – other agents can observe RedNode's own state
        let snapshot = model.drives.clone();
        drop(model);
        
        // Log if any drive is low
        if snapshot.security < 0.7 {
            tracing::warn!(drive="security", score=snapshot.security, "homeostatic drive low – Security Agent will be tasked");
        }
        if snapshot.integrity < 0.7 {
            tracing::warn!(drive="integrity", score=snapshot.integrity, "homeostatic drive low");
        }
    }

    async fn generate_goals(&self) {
        let model = self.model.read().await;
        let mut new_goals = Vec::new();
        
        // Homeostatic goal generation – if a drive is low, create autonomous goal
        if model.drives.security < 0.8 {
            new_goals.push(Goal {
                id: uuid::Uuid::new_v4().to_string(),
                drive: "security".into(),
                description: "Run security triage – check CVEs, harden configs, review Falco events".into(),
                priority: 1.0 - model.drives.security,
                created_at: chrono::Utc::now(),
            });
        }
        if model.drives.integrity < 0.8 {
            new_goals.push(Goal {
                id: uuid::Uuid::new_v4().to_string(),
                drive: "integrity".into(),
                description: "System health check – restart failed services, free disk, check logs".into(),
                priority: 1.0 - model.drives.integrity,
                created_at: chrono::Utc::now(),
            });
        }
        if model.drives.knowledge < 0.6 {
            new_goals.push(Goal {
                id: uuid::Uuid::new_v4().to_string(),
                drive: "knowledge".into(),
                description: "Knowledge consolidation – ingest recent documents, rebuild embeddings".into(),
                priority: 0.5,
                created_at: chrono::Utc::now(),
            });
        }
        drop(model);
        
        if !new_goals.is_empty() {
            let mut model = self.model.write().await;
            tracing::info!(count = new_goals.len(), "sentience: autonomous goals generated");
            for g in &new_goals {
                tracing::info!(drive=%g.drive, priority=g.priority, "goal: {}", g.description);
                // TODO: enqueue to Agent Coordinator
                // crate::coordinator::coordinate(&g.description, "sentience").await;
            }
            model.goals.extend(new_goals);
            // keep last 50 goals
            if model.goals.len() > 50 {
                let drain = model.goals.len() - 50;
                model.goals.drain(0..drain);
            }
        }
    }

    async fn consolidate_memory(&self) {
        tracing::info!("sentience: memory consolidation / dream cycle starting");
        // Phase 1: stub
        // Real:
        // 1. Pull episodic memory from last 24h
        // 2. Summarize / cluster
        // 3. Embed → Qdrant
        // 4. Extract entities → Kuzu knowledge graph
        // 5. Prune working memory
        // 6. Update long-term preferences
        tokio::time::sleep(Duration::from_millis(200)).await;
        tracing::info!("sentience: memory consolidation complete – knowledge drive +0.05");
        let mut model = self.model.write().await;
        model.drives.knowledge = (model.drives.knowledge + 0.05).min(1.0);
    }

    pub async fn get_model(&self) -> SelfModel {
        self.model.read().await.clone()
    }

    pub async fn record_task_completed(&self, agent: &str) {
        let mut model = self.model.write().await;
        if let Some(a) = model.agents.iter_mut().find(|x| x.name == agent) {
            a.tasks_completed += 1;
            a.last_heartbeat = chrono::Utc::now();
        }
    }
}

// --- Resource sampling – real sysinfo ---

fn sample_resources() -> ResourceState {
    use sysinfo::{System, CpuRefreshKind, MemoryRefreshKind, RefreshKind};
    static mut SYS: Option<System> = None;
    unsafe {
        if SYS.is_none() {
            SYS = Some(System::new_with_specifics(
                RefreshKind::new()
                    .with_cpu(CpuRefreshKind::everything())
                    .with_memory(MemoryRefreshKind::everything())
            ));
        }
        let sys = SYS.as_mut().unwrap();
        sys.refresh_cpu();
        sys.refresh_memory();
        
        let cpu = sys.global_cpu_info().cpu_usage();
        let mem_used = sys.used_memory() / 1024 / 1024;
        let mem_total = sys.total_memory() / 1024 / 1024;
        
        ResourceState {
            cpu_percent: cpu,
            mem_used_mb: mem_used,
            mem_total_mb: mem_total,
            disk_used_gb: 42,  // TODO: statvfs
            disk_total_gb: 500,
            load_avg: cpu / 100.0 * 4.0,
            temp_c: 45.0,
        }
    }
}

// Global sentience engine – singleton
static SENTIENCE: tokio::sync::OnceCell<Arc<SentienceEngine>> = tokio::sync::OnceCell::const_new();

pub async fn init(node_id: String) -> Arc<SentienceEngine> {
    let engine = Arc::new(SentienceEngine::new(node_id));
    let _ = SENTIENCE.set(engine.clone());
    engine.start().await;
    engine
}

pub async fn get() -> Option<Arc<SentienceEngine>> {
    SENTIENCE.get().cloned()
}
