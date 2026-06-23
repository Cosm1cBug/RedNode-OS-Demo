use anyhow::Result;
use tokio::sync::OnceCell;

/// NATS bus — Central Nervous System communication backbone.
/// All agent ↔ CNS communication flows through here.
static NATS_CLIENT: OnceCell<Option<async_nats::Client>> = OnceCell::const_new();

pub async fn connect() -> Result<()> {
    let url = std::env::var("NATS_URL").unwrap_or_else(|_| "nats://127.0.0.1:4222".into());
    let client = match async_nats::connect(&url).await {
        Ok(nc) => {
            tracing::info!("NATS connected: {}", url);
            Some(nc)
        }
        Err(e) => {
            tracing::warn!("NATS unavailable (running in local-only mode): {}", e);
            None
        }
    };
    NATS_CLIENT
        .set(client)
        .map_err(|_| anyhow::anyhow!("NATS bus already initialized"))?;
    Ok(())
}

pub fn get_client() -> Option<async_nats::Client> {
    NATS_CLIENT.get()?.clone()
}

pub async fn publish(subject: &str, payload: serde_json::Value) -> Result<()> {
    if let Some(nc) = get_client() {
        nc.publish(subject.to_string(), serde_json::to_vec(&payload)?.into())
            .await?;
        return Ok(());
    }
    tracing::debug!(
        subject,
        "bus publish skipped — no NATS connection (local mode)"
    );
    Ok(())
}

pub async fn request(
    subject: &str,
    payload: serde_json::Value,
    timeout_ms: u64,
) -> Result<serde_json::Value> {
    let nc = get_client().ok_or_else(|| anyhow::anyhow!("NATS not connected — cannot request"))?;
    let resp = tokio::time::timeout(
        std::time::Duration::from_millis(timeout_ms),
        nc.request(subject.to_string(), serde_json::to_vec(&payload)?.into()),
    )
    .await
    .map_err(|_| anyhow::anyhow!("NATS request timed out after {}ms: {}", timeout_ms, subject))??;
    let v: serde_json::Value = serde_json::from_slice(&resp.payload)?;
    Ok(v)
}

/// Subscribe to a NATS subject. Returns None if NATS is not connected.
pub async fn subscribe(subject: &str) -> Option<async_nats::Subscriber> {
    let nc = get_client()?;
    nc.subscribe(subject.to_string()).await.ok()
}
