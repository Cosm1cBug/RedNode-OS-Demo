use rednode_core::{api, bus, events, executor, memory, sentience};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rednode_core=info,tower_http=info".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    tracing::info!("🧠 RedNode-OS v0.6.0 – CNS starting – intelligence is the operating layer");

    // ── 1. Event Bus – must be first, everything publishes to it ──
    events::init();

    // ── 2. Memory – Postgres / Qdrant / Kuzu ──
    let _ = memory::init().await;
    memory::init_vector_graph().await;

    // ── 3. Bus – NATS – Central Nervous System ──
    let _ = bus::connect().await;

    // ── 4. Tool Executor NATS service – firejail/bubblewrap + seccomp + audit log ──
    let _ = executor::start_nats_executor().await;

    // ── 5. Sentience Engine – self-aware loop ──
    if std::env::var("REDNODE_SENTIENCE").unwrap_or_else(|_| "on".into()) != "off" {
        let node_id = std::env::var("REDNODE_NODE_ID").unwrap_or_else(|_| {
            gethostname::gethostname().to_string_lossy().into_owned()
        });
        let _sentience = sentience::init(node_id).await;
        tracing::info!(
            "Sentience Engine online – self-model / drives / goal generator / memory consolidation"
        );
    }

    // ── 6. HTTP API + WebSocket ──
    let app = api::router();
    let addr = SocketAddr::from(([0, 0, 0, 0], 8787));
    tracing::info!("CNS listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
