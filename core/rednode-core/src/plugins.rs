// RedNode-OS — Plugin Ecosystem
//
// A plugin is a self-contained capability package: agent + tools + config.
// Plugins can be installed, updated, and removed from the dashboard.
//
// Plugin manifest format (JSON):
//   {
//     "name": "weather-plugin",
//     "version": "1.0.0",
//     "description": "Advanced weather monitoring",
//     "author": "user",
//     "agent": "weather-agent",
//     "tools": [ { "name": "weather.forecast", ... } ],
//     "config_schema": { ... },
//     "permissions": ["network", "notifications"],
//     "entry_point": "src/index.ts"
//   }
//
// Safety:
//   - Plugins run in the same sandboxed environment as regular agents
//   - Governance policies apply to plugin tools
//   - Plugin permissions are checked against governance
//   - All plugin actions go through the audit chain
//
// Integration:
//   - evolution.rs handles tool registration
//   - governance.rs checks plugin permissions
//   - Dashboard shows plugin management UI
//
// API:
//   GET    /plugins            — list installed plugins
//   GET    /plugins/:id        — plugin details
//   POST   /plugins/install    — install a plugin from manifest
//   POST   /plugins/:id/enable — enable/disable
//   DELETE /plugins/:id        — uninstall a plugin

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

static PLUGINS: once_cell::sync::Lazy<Arc<RwLock<PluginStore>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(PluginStore::default())));

/// A plugin manifest — describes what the plugin provides
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub agent: String,
    pub tools: Vec<PluginTool>,
    pub config_schema: serde_json::Value,
    pub permissions: Vec<String>,
    pub entry_point: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginTool {
    pub name: String,
    pub description: String,
    pub risk: String,
    pub handler_type: String,
}

/// An installed plugin
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugin {
    pub id: String,
    pub manifest: PluginManifest,
    pub enabled: bool,
    pub installed_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub config: serde_json::Value,
    pub status: PluginStatus,
    pub tool_count: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum PluginStatus {
    Active,
    Disabled,
    Error,
    Installing,
    Uninstalling,
}

/// The plugin store
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginStore {
    pub plugins: Vec<InstalledPlugin>,
    pub total_installed: u64,
    pub total_uninstalled: u64,
}

impl Default for PluginStore {
    fn default() -> Self {
        Self {
            plugins: Vec::new(),
            total_installed: 0,
            total_uninstalled: 0,
        }
    }
}

fn gen_id() -> String {
    format!("plugin_{}", chrono::Utc::now().timestamp_millis())
}

// ─── Public API ───

/// List all installed plugins
pub async fn list() -> Vec<InstalledPlugin> {
    PLUGINS.read().await.plugins.clone()
}

/// Get a specific plugin by ID
pub async fn get(id: &str) -> Option<InstalledPlugin> {
    PLUGINS.read().await.plugins.iter().find(|p| p.id == id).cloned()
}

/// Install a plugin from its manifest
pub async fn install(manifest: PluginManifest, config: serde_json::Value) -> Result<InstalledPlugin, String> {
    let mut store = PLUGINS.write().await;

    // Check if already installed
    if store.plugins.iter().any(|p| p.manifest.name == manifest.name) {
        return Err(format!("Plugin '{}' is already installed", manifest.name));
    }

    // Check permissions against governance
    for perm in &manifest.permissions {
        let allowed = match perm.as_str() {
            "network" | "notifications" | "filesystem_read" | "llm" => true,
            "filesystem_write" | "shell" | "docker" => {
                tracing::warn!(
                    plugin = %manifest.name,
                    permission = %perm,
                    "Plugin requests elevated permission"
                );
                true // Allow but warn — governance policies will gate actual usage
            }
            _ => {
                tracing::warn!(
                    plugin = %manifest.name,
                    permission = %perm,
                    "Plugin requests unknown permission"
                );
                false
            }
        };
        if !allowed {
            return Err(format!("Permission '{}' denied for plugin '{}'", perm, manifest.name));
        }
    }

    let tool_count = manifest.tools.len();
    let name = manifest.name.clone();

    let plugin = InstalledPlugin {
        id: gen_id(),
        manifest,
        enabled: true,
        installed_at: Utc::now(),
        updated_at: Utc::now(),
        config,
        status: PluginStatus::Active,
        tool_count,
        error: None,
    };

    store.plugins.push(plugin.clone());
    store.total_installed += 1;

    tracing::info!(name = %name, tools = tool_count, "Plugin installed");

    crate::events::emit(serde_json::json!({
        "type": "plugin_installed",
        "name": name,
        "tools": tool_count,
        "ts": Utc::now().to_rfc3339(),
    }));

    Ok(plugin)
}

/// Enable or disable a plugin
pub async fn set_enabled(id: &str, enabled: bool) -> bool {
    let mut store = PLUGINS.write().await;
    if let Some(plugin) = store.plugins.iter_mut().find(|p| p.id == id) {
        plugin.enabled = enabled;
        plugin.status = if enabled { PluginStatus::Active } else { PluginStatus::Disabled };
        plugin.updated_at = Utc::now();
        return true;
    }
    false
}

/// Uninstall a plugin
pub async fn uninstall(id: &str) -> bool {
    let mut store = PLUGINS.write().await;
    let before = store.plugins.len();
    store.plugins.retain(|p| p.id != id);
    if store.plugins.len() < before {
        store.total_uninstalled += 1;
        return true;
    }
    false
}

/// Get active plugin count
pub async fn active_count() -> usize {
    PLUGINS.read().await.plugins.iter()
        .filter(|p| p.status == PluginStatus::Active)
        .count()
}

/// Get all tools from active plugins
pub async fn get_plugin_tools() -> Vec<PluginTool> {
    let store = PLUGINS.read().await;
    store.plugins.iter()
        .filter(|p| p.status == PluginStatus::Active)
        .flat_map(|p| p.manifest.tools.clone())
        .collect()
}

// ─── Persistence ───

pub async fn persist() {
    let store = PLUGINS.read().await.clone();
    if let Some(pool) = crate::memory::pool() {
        let json = match serde_json::to_value(&store) {
            Ok(v) => v,
            Err(e) => {
                tracing::warn!("Failed to serialize plugins: {}", e);
                return;
            }
        };

        let _ = sqlx::query(
            "INSERT INTO plugin_store (id, state, updated_at) \
             VALUES (1, $1, NOW()) \
             ON CONFLICT (id) DO UPDATE SET state = $1, updated_at = NOW()"
        )
        .bind(&json)
        .execute(pool)
        .await;
    }
}

pub async fn restore() {
    if let Some(pool) = crate::memory::pool() {
        let row: Option<(serde_json::Value,)> = sqlx::query_as(
            "SELECT state FROM plugin_store WHERE id = 1"
        )
        .fetch_optional(pool)
        .await
        .unwrap_or(None);

        if let Some((json,)) = row {
            if let Ok(restored) = serde_json::from_value::<PluginStore>(json) {
                let mut store = PLUGINS.write().await;
                *store = restored;
                tracing::info!(
                    plugins = store.plugins.len(),
                    active = store.plugins.iter().filter(|p| p.status == PluginStatus::Active).count(),
                    "Plugin store restored"
                );
            }
        }
    }
}

pub async fn init_table() {
    if let Some(pool) = crate::memory::pool() {
        let _ = sqlx::query(
            "CREATE TABLE IF NOT EXISTS plugin_store (\
                id INTEGER PRIMARY KEY, \
                state JSONB NOT NULL, \
                updated_at TIMESTAMPTZ DEFAULT NOW()\
            )"
        )
        .execute(pool)
        .await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_store() {
        let store = PluginStore::default();
        assert!(store.plugins.is_empty());
        assert_eq!(store.total_installed, 0);
    }

    #[test]
    fn test_plugin_serialization() {
        let manifest = PluginManifest {
            name: "test-plugin".into(),
            version: "1.0.0".into(),
            description: "A test plugin".into(),
            author: "tester".into(),
            agent: "test-agent".into(),
            tools: vec![PluginTool {
                name: "test.hello".into(),
                description: "Say hello".into(),
                risk: "low".into(),
                handler_type: "shell".into(),
            }],
            config_schema: serde_json::json!({}),
            permissions: vec!["network".into()],
            entry_point: "src/index.ts".into(),
        };
        let json = serde_json::to_string(&manifest).unwrap();
        assert!(json.contains("test-plugin"));
        assert!(json.contains("test.hello"));
    }

    #[test]
    fn test_plugin_status_eq() {
        assert_eq!(PluginStatus::Active, PluginStatus::Active);
        assert_ne!(PluginStatus::Active, PluginStatus::Disabled);
    }
}
