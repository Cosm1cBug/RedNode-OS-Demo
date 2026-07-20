use rednode_core::{api, bus, events, executor, memory, sentience};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rednode_core=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    tracing::info!("RedNode-OS v0.37.0 – CNS starting – the computer becomes the intelligence");

    // ── 1. Event Bus – must be first, everything publishes to it ──
    events::init();

    // ── 2. Memory – Postgres / Qdrant / Kuzu ──
    let _ = memory::init().await;
    memory::init_vector_graph().await;

    // ── 3. Initialize all module tables (auto-create on fresh DB) ──
    init_all_tables().await;

    // ── 4. Bus – NATS – Central Nervous System ──
    let _ = bus::connect().await;

    // ── 5. Tool Executor NATS service – firejail/bubblewrap + seccomp + audit log ──
    let _ = executor::start_nats_executor().await;

    // ── 6. Sentience Engine – self-aware loop ──
    if std::env::var("REDNODE_SENTIENCE").unwrap_or_else(|_| "on".into()) != "off" {
        let node_id = std::env::var("REDNODE_NODE_ID")
            .unwrap_or_else(|_| gethostname::gethostname().to_string_lossy().into_owned());
        let _sentience = sentience::init(node_id).await;
        tracing::info!(
            "Sentience Engine online – self-model / drives / goal generator / memory consolidation"
        );
    }

    // ── 7. Restore persisted state from all cognitive modules ──
    restore_all_state().await;

    // ── 8. Start consciousness loop ──
    // ── 8. Start multi-speed cognitive clocks ──
    start_cognitive_clocks();

    // ── 9. HTTP API + WebSocket ──
    let app = api::router();
    let addr = SocketAddr::from(([0, 0, 0, 0], 8787));
    tracing::info!("CNS listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

/// Initialize all module tables in PostgreSQL.
/// Safe to call on every boot — uses CREATE TABLE IF NOT EXISTS.
async fn init_all_tables() {
    use rednode_core::*;

    // Phase 3: Consciousness Development (v0.10–v0.16)
    consciousness::init_table().await;
    goals::init_table().await;
    world_model::init_table().await;
    time_intel::init_table().await;
    personality::init_table().await;
    reflection::init_table().await;
    curiosity::init_table().await;
    distillation::init_table().await;
    economy::init_table().await;
    immune::init_table().await;
    governance::init_table().await;
    plugins::init_table().await;
    twin::init_table().await;

    // Phase 4: True Digital Entity (v0.17–v0.24)
    identity::init_table().await;
    constitution::init_table().await;
    meta_reasoning::init_table().await;
    episodic_memory::init_table().await;
    simulation::init_table().await;
    debate::init_table().await;
    trust::init_table().await;
    ethics::init_table().await;
    creativity::init_table().await;
    scientific::init_table().await;
    capability_registry::init_table().await;
    dreaming::init_table().await;
    collective::init_table().await;
    hal::init_table().await;

    // Phase 5: Cognitive Architecture (v0.25–v0.34)
    attention::init_table().await;
    cognitive_load::init_table().await;
    intent_engine::init_table().await;
    context_engine::init_table().await;
    explainability::init_table().await;
    uncertainty::init_table().await;
    value_estimator::init_table().await;
    provenance::init_table().await;
    forgetting::init_table().await;
    knowledge_lifecycle::init_table().await;
    memory_safety::init_table().await;
    emotional_state::init_table().await;
    cognitive_metrics::init_table().await;
    model_orchestrator::init_table().await;
    verification::init_table().await;
    experience_replay::init_table().await;
    evolution_sandbox::init_table().await;
    digital_legacy::init_table().await;
    adaptive_arch::init_table().await;
    collective_governance::init_table().await;

    // Phase 6: Enhancement (v0.37.0)
    cognitive_bus::init_table().await;
    perception::init_table().await;
    language::init_table().await;
    mental_models::init_table().await;

    tracing::info!("All module tables initialized (57 tables)");
}

/// Restore persisted state from PostgreSQL for all cognitive modules.
/// Modules that have no saved state will use their defaults.
async fn restore_all_state() {
    use rednode_core::*;

    consciousness::restore().await;
    goals::restore().await;
    world_model::restore().await;
    time_intel::restore().await;
    personality::restore().await;
    reflection::restore().await;
    curiosity::restore().await;
    distillation::restore().await;
    economy::restore().await;
    immune::restore().await;
    governance::restore().await;
    plugins::restore().await;
    twin::restore().await;
    identity::restore().await;
    constitution::restore().await;
    meta_reasoning::restore().await;
    episodic_memory::restore().await;
    simulation::restore().await;
    debate::restore().await;
    trust::restore().await;
    ethics::restore().await;
    creativity::restore().await;
    scientific::restore().await;
    capability_registry::restore().await;
    dreaming::restore().await;
    collective::restore().await;
    hal::restore().await;
    attention::restore().await;
    cognitive_load::restore().await;
    intent_engine::restore().await;
    context_engine::restore().await;
    explainability::restore().await;
    uncertainty::restore().await;
    value_estimator::restore().await;
    provenance::restore().await;
    forgetting::restore().await;
    knowledge_lifecycle::restore().await;
    memory_safety::restore().await;
    emotional_state::restore().await;
    cognitive_metrics::restore().await;
    model_orchestrator::restore().await;
    verification::restore().await;
    experience_replay::restore().await;
    evolution_sandbox::restore().await;
    digital_legacy::restore().await;
    adaptive_arch::restore().await;
    collective_governance::restore().await;
    cognitive_bus::restore().await;
    perception::restore().await;
    language::restore().await;
    mental_models::restore().await;

    tracing::info!("All module state restored from database");
}

/// Multi-speed cognitive clocks — different subsystems at different cadences
fn start_cognitive_clocks() {
    use rednode_core::*;

    // Fast loop (2s): attention, perception, event processing
    tokio::spawn(async {
        tracing::info!("Fast cognitive clock started (2s)");
        loop {
            attention::tick().await;
            cognitive_bus::tick().await;
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
        }
    });

    // Medium loop (10s): consciousness, context, emotional state
    tokio::spawn(async {
        tracing::info!("Medium cognitive clock started (10s)");
        loop {
            consciousness::tick().await;
            emotional_state::update_from_system().await;
            episodic_memory::tick().await;
            world_model::tick().await;
            economy::tick().await;
            tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;
        }
    });

    // Slow loop (5min): reflection, metrics, capability checks
    tokio::spawn(async {
        tracing::info!("Slow cognitive clock started (5min)");
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(300)).await;
            reflection::tick().await;
            cognitive_metrics::compute().await;
            capability_registry::tick().await;
            time_intel::tick().await;
            immune::tick().await;
        }
    });

    // Background loop (1h): dreaming, research, evolution, calibration
    tokio::spawn(async {
        tracing::info!("Background cognitive clock started (1h)");
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(3600)).await;
            if let Some(trigger) = dreaming::should_dream().await {
                dreaming::start_dream(trigger).await;
            }
            if curiosity::tick().await {
                curiosity::build_exploration_queue().await;
            }
            distillation::tick().await;
            forgetting::tick().await;
        }
    });

    tracing::info!("Multi-speed cognitive clocks active: fast(2s) + medium(10s) + slow(5min) + background(1h)");
}
