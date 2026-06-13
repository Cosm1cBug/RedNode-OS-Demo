#[tauri::command]
fn cns_health() -> String { "rednode-cns ok".into() }
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .invoke_handler(tauri::generate_handler![cns_health])
    .run(tauri::generate_context!())
    .expect("error while running tauri");
}
