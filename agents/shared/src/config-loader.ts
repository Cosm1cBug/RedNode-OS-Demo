/**
 * RedNode-OS — Config Loader
 *
 * Fetches configuration from CNS API and populates process.env
 * so that existing agent code (process.env.PIHOLE_URL etc.) works
 * WITHOUT any changes.
 *
 * Usage in agent index.ts:
 *   import { loadConfig } from "../../shared/src/config-loader.js";
 *   await loadConfig();  // call BEFORE anything else
 *
 * What it does:
 *   1. GET /config from CNS API
 *   2. Maps service URLs to env var names (pihole.url → PIHOLE_URL)
 *   3. Maps secrets to env var names (pihole.secrets.PIHOLE_PASSWORD → PIHOLE_PASSWORD)
 *   4. Sets process.env for each
 *   5. Subscribes to NATS "rednode.config.changed" for live updates
 *
 * Fallback: if CNS is not reachable, process.env from .env file still works
 */

const CNS_URL = process.env.REDNODE_CNS || "http://localhost:8787";

/**
 * Map from config.json service keys to environment variable names
 */
const SERVICE_ENV_MAP: Record<string, Record<string, string>> = {
  pihole: {
    url: "PIHOLE_URL",
    "secrets.PIHOLE_PASSWORD": "PIHOLE_PASSWORD",
  },
  truenas: {
    url: "TRUENAS_URL",
    "secrets.TRUENAS_API_KEY": "TRUENAS_API_KEY",
  },
  frigate: {
    url: "FRIGATE_URL",
  },
  homeassistant: {
    url: "HOMEASSISTANT_URL",
    "secrets.HOMEASSISTANT_TOKEN": "HOMEASSISTANT_TOKEN",
  },
  ollama: {
    url: "OLLAMA_URL",
  },
  jellyfin: {
    url: "JELLYFIN_URL",
    "secrets.JELLYFIN_API_KEY": "JELLYFIN_API_KEY",
  },
  searxng: {
    url: "SEARXNG_URL",
  },
  pfsense: {
    url: "PFSENSE_URL",
    "secrets.PFSENSE_API_KEY": "PFSENSE_API_KEY",
    "secrets.PFSENSE_API_SECRET": "PFSENSE_API_SECRET",
  },
  nats: {
    url: "NATS_URL",
  },
  qdrant: {
    url: "QDRANT_URL",
  },
};

/**
 * Map from preference keys to environment variable names
 */
const PREF_ENV_MAP: Record<string, string> = {
  notify_quiet_start: "NOTIFY_QUIET_START",
  notify_quiet_end: "NOTIFY_QUIET_END",
  notify_channel: "NOTIFY_CHANNEL",
  predict_min_days: "REDNODE_PREDICT_MIN_DAYS",
  predict_min_logs: "REDNODE_PREDICT_MIN_LOGS",
  voice_enabled: "REDNODE_VOICE",
  voice_wake_word: "WAKE_WORD",
  gui_enabled: "REDNODE_GUI",
  rss_feeds: "RSS_FEEDS",
  rss_check_interval: "RSS_CHECK_INTERVAL",
};

/**
 * Fetch config from CNS and populate process.env
 * Call this at agent startup BEFORE any process.env reads
 */
export async function loadConfig(): Promise<boolean> {
  try {
    const token = process.env.REDNODE_API_TOKEN || "";
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    };
    if (token) headers["Authorization"] = `Bearer ${token}`;

    const res = await fetch(`${CNS_URL}/config`, {
      headers,
      signal: AbortSignal.timeout(5000),
    });

    if (!res.ok) {
      console.warn("[config-loader] CNS /config returned", res.status, "— using .env fallback");
      return false;
    }

    const data = (await res.json()) as any;
    let populated = 0;

    // Populate service URLs and secrets
    if (data.services) {
      for (const [svcKey, svc] of Object.entries(data.services as Record<string, any>)) {
        const mapping = SERVICE_ENV_MAP[svcKey];
        if (!mapping) continue;

        // URL
        if (mapping.url && svc.url) {
          process.env[mapping.url] = svc.url;
          populated++;
        }

        // Secrets (only if they have actual values, not masked)
        if (svc.secrets) {
          for (const [secretKey, secretInfo] of Object.entries(svc.secrets as Record<string, any>)) {
            const envKey = `secrets.${secretKey}`;
            if (mapping[envKey] && secretInfo.value && secretInfo.value !== "••••••••") {
              process.env[mapping[envKey]] = secretInfo.value;
              populated++;
            }
          }
        }
      }
    }

    // Populate preferences
    if (data.preferences) {
      for (const [prefKey, envKey] of Object.entries(PREF_ENV_MAP)) {
        const value = data.preferences[prefKey];
        if (value !== undefined && value !== null) {
          if (prefKey === "voice_enabled" || prefKey === "gui_enabled") {
            process.env[envKey] = value ? "on" : "off";
          } else if (prefKey === "rss_feeds" && Array.isArray(value)) {
            process.env[envKey] = value.join("|");
          } else {
            process.env[envKey] = String(value);
          }
          populated++;
        }
      }
    }

    console.info(`[config-loader] Loaded ${populated} config values from CNS`);
    return true;
  } catch (e: any) {
    console.warn(`[config-loader] CNS not reachable (${e.message}) — using .env fallback`);
    return false;
  }
}

/**
 * Subscribe to config change events via NATS
 * When config changes in the dashboard, agents re-fetch automatically
 */
export async function watchConfigChanges(
  nc: any, // NatsConnection
  onReload?: () => void
): Promise<void> {
  try {
    const sub = nc.subscribe("rednode.config.changed");
    for await (const msg of sub) {
      console.info("[config-loader] 🔄 Config changed — re-fetching from CNS...");
      const loaded = await loadConfig();
      if (loaded) {
        console.info("[config-loader] ✅ Config reloaded successfully");
        if (onReload) onReload();
      }
    }
  } catch (e: any) {
    console.warn(`[config-loader] Config watch failed: ${e.message}`);
  }
}
