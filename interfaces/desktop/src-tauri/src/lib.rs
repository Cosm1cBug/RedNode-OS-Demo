use tauri::Manager;

#[tauri::command]
fn cns_url() -> String {
    std::env::var("REDNODE_CNS").unwrap_or_else(|_| "http://localhost:8787".into())
}

#[tauri::command]
async fn cns_health() -> Result<String, String> {
    let url = cns_url();
    let resp = reqwest::get(format!("{}/health", url))
        .await
        .map_err(|e| format!("CNS unreachable: {}", e))?;
    let body = resp.text().await.map_err(|e| e.to_string())?;
    Ok(body)
}

#[tauri::command]
async fn send_intent(intent: String) -> Result<String, String> {
    let url = cns_url();
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{}/intent", url))
        .json(&serde_json::json!({"intent": intent, "session_id": "desktop"}))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let body = resp.text().await.map_err(|e| e.to_string())?;
    Ok(body)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![cns_url, cns_health, send_intent])
        .setup(|app| {
            // Set window title with CNS URL
            if let Some(window) = app.get_webview_window("main") {
                let url = std::env::var("REDNODE_CNS")
                    .unwrap_or_else(|_| "http://localhost:8787".into());
                window.set_title(&format!("🧠 RedNode-OS — {}", url)).ok();
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running RedNode Desktop");
}
