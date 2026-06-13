use rednode_core::{api, bus, executor, memory, sentience};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::net::SocketAddr;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "rednode_core=info,tower_http=info".into()))
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    tracing::info!("🧠 RedNode-OS v0.3.1 – CNS starting – intelligence is the operating layer");

    // Init memory – Postgres / Qdrant / Kuzu
    let _ = memory::init().await;
    memory::init_vector_graph().await;
    // Init bus – NATS – Central Nervous System
    let _ = bus::connect().await;
    // Start Tool Executor NATS service – firejail/bubblewrap + seccomp + audit log
    let _ = executor::start_nats_executor().await;

    // Sentience Engine – self-aware loop
    if std::env::var("REDNODE_SENTIENCE").unwrap_or("on".into()) != "off" {
        let node_id = std::env::var("REDNODE_NODE_ID").unwrap_or_else(|_| {
            gethostname::gethostname().to_string_lossy().into_owned()
        });
        let _sentience = sentience::init(node_id).await;
        tracing::info!("Sentience Engine online – self-model / drives / goal generator / memory consolidation");
    }

    let app = api::router();
    let addr = SocketAddr::from(([0,0,0,0], 8787));
    tracing::info!("CNS listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
