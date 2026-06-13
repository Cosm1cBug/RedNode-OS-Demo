use anyhow::Result;
pub struct Bus { pub nats: Option<async_nats::Client> }
static mut BUS: Option<Bus> = None;

pub async fn connect() -> Result<()> {
    let url = std::env::var("NATS_URL").unwrap_or("nats://127.0.0.1:4222".into());
    match async_nats::connect(&url).await {
        Ok(nc) => { tracing::info!("NATS connected {}", url); unsafe { BUS = Some(Bus{ nats: Some(nc) })}; Ok(()) },
        Err(e) => { tracing::warn!("NATS unavailable (running in local mode): {}", e); unsafe { BUS = Some(Bus{ nats: None })}; Ok(()) }
    }
}

pub fn get_client() -> Option<async_nats::Client> {
    unsafe { BUS.as_ref()?.nats.clone() }
}

pub async fn publish(subject: &str, payload: serde_json::Value) -> Result<()> {
    unsafe {
        if let Some(bus) = &BUS {
            if let Some(nc) = &bus.nats {
                nc.publish(subject.to_string(), serde_json::to_vec(&payload)?.into()).await?;
                return Ok(());
            }
        }
    }
    tracing::debug!(subject, "bus publish (local)");
    Ok(())
}

pub async fn request(subject: &str, payload: serde_json::Value, timeout_ms: u64) -> Result<serde_json::Value> {
    let nc = get_client().ok_or_else(|| anyhow::anyhow!("NATS not connected"))?;
    let resp = tokio::time::timeout(
        std::time::Duration::from_millis(timeout_ms),
        nc.request(subject.to_string(), serde_json::to_vec(&payload)?.into())
    ).await??;
    let v: serde_json::Value = serde_json::from_slice(&resp.payload)?;
    Ok(v)
}
