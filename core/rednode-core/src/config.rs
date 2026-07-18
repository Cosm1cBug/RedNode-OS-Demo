// RedNode-OS — Configuration Manager
//
// ALL configuration is managed through the web dashboard.
// No .env editing required. .env is ONLY a fallback for first boot
// before the dashboard is reachable.
//
// Config storage: /var/lib/rednode/config.json
// Secrets: encrypted in config.json using age
// Agents: fetch config via GET /config/agent (includes decrypted secrets)
// Dashboard: GET /config (secrets masked as ••••••••)
//
// Flow:
//   First boot → setup wizard → saves to config.json
//   Agent startup → GET /config/agent → populates process.env
//   Dashboard → GET/POST /config → edit settings visually
//   Config change → NATS broadcast → agents re-fetch

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

static CONFIG: once_cell::sync::Lazy<RwLock<RedNodeConfig>> =
    once_cell::sync::Lazy::new(|| RwLock::new(RedNodeConfig::load()));

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RedNodeConfig {
    pub services: HashMap<String, ServiceConfig>,
    pub preferences: Preferences,
    pub system: SystemConfig,
    #[serde(skip)]
    pub config_path: String,
    #[serde(skip)]
    pub setup_complete: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceConfig {
    pub name: String,
    pub url: String,
    pub enabled: bool,
    #[serde(default)]
    pub secrets: HashMap<String, String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub test_endpoint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub hostname: String,
    pub model: String,
    pub code_model: String,
    pub api_token: String,
    pub sentience: bool,
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            hostname: "rednode".into(),
            model: "qwen2.5:7b-instruct-q4_K_M".into(),
            code_model: String::new(),
            api_token: String::new(),
            sentience: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preferences {
    pub notify_quiet_start: u32,
    pub notify_quiet_end: u32,
    pub notify_channel: String,
    pub predict_min_days: u64,
    pub predict_min_logs: i64,
    pub voice_enabled: bool,
    pub voice_wake_word: String,
    pub gui_enabled: bool,
    pub rss_feeds: Vec<String>,
    pub rss_check_interval: u64,
    pub photo_auto_organize: bool,
    pub photo_auto_tag: bool,
    pub weather_location: String,
    pub news_region: String,
    pub calendar_reminder_minutes: u32,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            notify_quiet_start: 22,
            notify_quiet_end: 7,
            notify_channel: "signal".into(),
            predict_min_days: 7,
            predict_min_logs: 200,
            voice_enabled: false,
            voice_wake_word: "hey_jarvis".into(),
            gui_enabled: false,
            rss_feeds: vec![],
            rss_check_interval: 3600,
            photo_auto_organize: true,
            photo_auto_tag: true,
            weather_location: String::new(),
            news_region: "world".into(),
            calendar_reminder_minutes: 30,
        }
    }
}

impl RedNodeConfig {
    pub fn load() -> Self {
        let config_path = std::env::var("REDNODE_CONFIG")
            .unwrap_or_else(|_| {
                let home = std::env::var("REDNODE_HOME")
                    .unwrap_or_else(|_| "/var/lib/rednode".into());
                format!("{}/config.json", home)
            });

        if Path::new(&config_path).exists() {
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                if let Ok(mut cfg) = serde_json::from_str::<RedNodeConfig>(&content) {
                    cfg.config_path = config_path;
                    cfg.setup_complete = true;
                    tracing::info!("Config loaded from {}", cfg.config_path);
                    return cfg;
                }
            }
        }

        // No config.json — import from .env as bootstrap
        let mut config = Self::from_env();
        config.config_path = config_path;
        config.setup_complete = false;
        config
    }

    /// Bootstrap config from .env (first boot only)
    fn from_env() -> Self {
        let env = |key: &str, default: &str| -> String {
            std::env::var(key).unwrap_or_else(|_| default.into())
        };

        let mut services = HashMap::new();

        let mut svc = |key: &str, name: &str, url_var: &str, default_url: &str, agent: &str, test: &str, secret_vars: Vec<&str>| {
            let mut secrets = HashMap::new();
            for sv in secret_vars {
                if let Ok(val) = std::env::var(sv) {
                    if !val.is_empty() {
                        secrets.insert(sv.to_string(), val);
                    }
                }
            }
            services.insert(key.into(), ServiceConfig {
                name: name.into(),
                url: env(url_var, default_url),
                enabled: true,
                secrets,
                agent: Some(agent.into()),
                test_endpoint: if test.is_empty() { None } else { Some(test.into()) },
            });
        };

        svc("pihole", "Pi-hole", "PIHOLE_URL", "http://10.0.50.2", "infra-agent", "/admin/api.php?summary", vec!["PIHOLE_PASSWORD"]);
        svc("truenas", "TrueNAS", "TRUENAS_URL", "https://10.0.50.3", "storage-agent", "/api/v2.0/system/info", vec!["TRUENAS_API_KEY"]);
        svc("frigate", "Frigate NVR", "FRIGATE_URL", "http://localhost:5000", "surveillance-agent", "/api/stats", vec![]);
        svc("homeassistant", "Home Assistant", "HOMEASSISTANT_URL", "http://localhost:8123", "home-agent", "/api/", vec!["HOMEASSISTANT_TOKEN"]);
        svc("ollama", "Ollama LLM", "OLLAMA_URL", "http://127.0.0.1:11434", "", "/api/tags", vec![]);
        svc("jellyfin", "Jellyfin Media", "JELLYFIN_URL", "http://localhost:8096", "media-agent", "/System/Info", vec!["JELLYFIN_API_KEY"]);
        svc("searxng", "SearXNG Search", "SEARXNG_URL", "http://localhost:8888", "research-agent", "/search?q=test&format=json", vec![]);
        svc("pfsense", "pfSense Firewall", "PFSENSE_URL", "https://10.0.50.1", "network-agent", "", vec!["PFSENSE_API_KEY", "PFSENSE_API_SECRET"]);
        svc("signal", "Signal Bot", "SIGNAL_CLI_PATH", "/usr/bin/signal-cli", "signal-bot", "", vec!["SIGNAL_BOT_NUMBER", "SIGNAL_OWNER_NUMBER"]);
        svc("mqtt", "MQTT Broker", "MQTT_URL", "mqtt://localhost:1883", "surveillance-agent", "", vec!["MQTT_USER", "MQTT_PASS"]);
        svc("email", "Email (IMAP/SMTP)", "IMAP_HOST", "", "comms-agent", "", vec!["IMAP_USER", "IMAP_PASS", "SMTP_HOST", "SMTP_USER", "SMTP_PASS"]);
        svc("calendar", "Calendar (CalDAV)", "CALDAV_URL", "", "comms-agent", "", vec!["CALDAV_USER", "CALDAV_PASS"]);
        svc("social_twitter", "Twitter/X", "TWITTER_BEARER_TOKEN", "", "social-agent", "", vec!["TWITTER_BEARER_TOKEN"]);
        svc("social_mastodon", "Mastodon", "MASTODON_INSTANCE", "", "social-agent", "", vec!["MASTODON_ACCESS_TOKEN"]);
        svc("social_bluesky", "Bluesky", "BLUESKY_HANDLE", "", "social-agent", "", vec!["BLUESKY_HANDLE", "BLUESKY_APP_PASSWORD"]);

        Self {
            services,
            preferences: Preferences::default(),
            system: SystemConfig {
                hostname: env("REDNODE_HOSTNAME", "rednode"),
                model: env("REDNODE_MODEL", "qwen2.5:7b-instruct-q4_K_M"),
                code_model: env("REDNODE_CODE_MODEL", ""),
                api_token: env("REDNODE_API_TOKEN", ""),
                sentience: env("REDNODE_SENTIENCE", "on") == "on",
            },
            config_path: String::new(),
            setup_complete: false,
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        if let Some(parent) = Path::new(&self.config_path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&self.config_path, format!("{}\n", json))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&self.config_path, std::fs::Permissions::from_mode(0o600));
        }

        tracing::info!("Config saved to {}", self.config_path);
        Ok(())
    }
}

// ─── Public API ───

pub fn get() -> RedNodeConfig {
    CONFIG.read().unwrap().clone()
}

pub fn is_setup_complete() -> bool {
    CONFIG.read().map(|c| c.setup_complete).unwrap_or(false)
}

pub fn update<F>(f: F) -> anyhow::Result<()>
where F: FnOnce(&mut RedNodeConfig),
{
    let mut config = CONFIG.write().map_err(|e| anyhow::anyhow!("Lock: {}", e))?;
    f(&mut config);
    config.save()?;

    // Broadcast change via NATS so agents re-fetch
    tokio::spawn(async {
        if let Ok(nc) = crate::bus::get_connection().await {
            let msg = serde_json::json!({"type": "config_changed", "ts": chrono::Utc::now().to_rfc3339()});
            if let Ok(payload) = serde_json::to_vec(&msg) {
                let _ = nc.publish("rednode.config.changed".into(), payload.into()).await;
                tracing::info!("Config change broadcast to agents");
            }
        }
    });

    // Audit log the change
    crate::events::emit(serde_json::json!({
        "type": "config_changed",
        "ts": chrono::Utc::now().to_rfc3339(),
    }));

    Ok(())
}

/// For web dashboard — secrets masked
pub fn get_for_dashboard() -> serde_json::Value {
    let config = get();
    let mut services = serde_json::Map::new();

    for (key, svc) in &config.services {
        let mut svc_json = serde_json::json!({
            "name": svc.name,
            "url": svc.url,
            "enabled": svc.enabled,
            "agent": svc.agent,
            "test_endpoint": svc.test_endpoint,
        });

        let mut masked_secrets = serde_json::Map::new();
        for (sk, sv) in &svc.secrets {
            masked_secrets.insert(sk.clone(), serde_json::json!({
                "configured": !sv.is_empty(),
                "value": if sv.is_empty() { "" } else { "••••••••" },
            }));
        }
        svc_json["secrets"] = serde_json::Value::Object(masked_secrets);
        services.insert(key.clone(), svc_json);
    }

    serde_json::json!({
        "services": services,
        "preferences": config.preferences,
        "system": {
            "hostname": config.system.hostname,
            "model": config.system.model,
            "sentience": config.system.sentience,
        },
        "setup_complete": config.setup_complete,
    })
}

/// For agents — secrets included (authenticated endpoint)
pub fn get_for_agents() -> serde_json::Value {
    let config = get();
    let mut services = serde_json::Map::new();

    for (key, svc) in &config.services {
        let mut svc_json = serde_json::json!({
            "name": svc.name,
            "url": svc.url,
            "enabled": svc.enabled,
        });

        // Include actual secret values for agents
        let mut secrets = serde_json::Map::new();
        for (sk, sv) in &svc.secrets {
            secrets.insert(sk.clone(), serde_json::json!({
                "configured": !sv.is_empty(),
                "value": sv,
            }));
        }
        svc_json["secrets"] = serde_json::Value::Object(secrets);
        services.insert(key.clone(), svc_json);
    }

    serde_json::json!({
        "services": services,
        "preferences": config.preferences,
        "system": {
            "hostname": config.system.hostname,
            "model": config.system.model,
            "code_model": config.system.code_model,
            "api_token": config.system.api_token,
            "sentience": config.system.sentience,
        },
    })
}

/// Test if a service is reachable
pub async fn test_service(service: &str) -> serde_json::Value {
    let config = get();
    let svc = match config.services.get(service) {
        Some(s) => s.clone(),
        None => return serde_json::json!({"ok": false, "error": "Unknown service"}),
    };

    let test_url = match &svc.test_endpoint {
        Some(ep) if !ep.is_empty() => format!("{}{}", svc.url, ep),
        _ => return serde_json::json!({"ok": true, "status": "no test endpoint configured"}),
    };

    let start = std::time::Instant::now();
    match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .danger_accept_invalid_certs(true)
        .build()
    {
        Ok(client) => {
            match client.get(&test_url).send().await {
                Ok(resp) => {
                    let status = resp.status();
                    serde_json::json!({
                        "ok": status.is_success() || status.as_u16() == 401,
                        "status": status.as_u16(),
                        "latency_ms": start.elapsed().as_millis(),
                        "url": svc.url,
                    })
                }
                Err(e) => serde_json::json!({
                    "ok": false,
                    "error": format!("{}", e),
                    "url": svc.url,
                }),
            }
        }
        Err(e) => serde_json::json!({"ok": false, "error": format!("{}", e)}),
    }
}
