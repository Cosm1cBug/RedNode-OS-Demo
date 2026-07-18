# v0.9.2 — Web UI as the ONLY Configuration Interface

## How It Works

```
FIRST BOOT:
  Browser → http://rednode:3000/setup
    → Setup wizard: enter Pi-hole URL, TrueNAS API key, etc.
    → All values saved to config.json (secrets encrypted with age)
    → CNS loads config.json at startup
    → Agents fetch config from CNS API at startup
    → Everything works — no .env editing ever needed

CHANGING CONFIG LATER:
  Browser → http://rednode:3000/settings
    → Change Pi-hole URL → Test Connection → Save
    → CNS writes to config.json
    → CNS broadcasts config-changed event via NATS
    → Agents re-fetch config automatically
    → No restart needed

FLOW:
  ┌──────────────┐     ┌─────────────────────────┐
  │  Web UI      │────▶│  CNS API                │
  │  /settings   │     │  POST /config/:service  │
  │  /setup      │     │  GET  /config           │
  └──────────────┘     └────────┬────────────────┘
                                │ read/write
                                ▼
                       ┌─────────────────────────┐
                       │  config.json            │
                       │  (encrypted secrets)    │
                       │  chmod 600              │
                       └────────┬────────────────┘
                                │ serve via API
                                ▼
                       ┌─────────────────────────┐
                       │  Agents                 │
                       │  fetch config on startup│
                       │  re-fetch on NATS signal│
                       │  populate process.env   │
                       └─────────────────────────┘
```

## Key Design: Zero process.env Changes

Agents currently do `process.env.PIHOLE_URL`. Instead of changing all 101 
references, the shared config fetcher:

1. On agent startup: `GET /config` from CNS
2. For each config value: `process.env.PIHOLE_URL = config.pihole.url`
3. Existing agent code works unchanged
4. On NATS `rednode.config.changed` signal: re-fetch and re-populate

## Secret Handling

Secrets are encrypted in config.json using `age` (the same tool RedNode
uses for export/import). The encryption key is at `/var/lib/rednode/config.key`
(generated on first boot, chmod 600).

In the web UI, secrets show as `••••••••` with a "Change" button.
When saved, they're encrypted before writing to config.json.

## Files

| File | What |
|---|---|
| `core/rednode-core/src/config.rs` | Rust config manager — load/save/encrypt/serve |
| `agents/shared/src/config-loader.ts` | TypeScript config fetcher — populates process.env from CNS |
| `interfaces/web/app/settings/page.tsx` | Settings page — forms for every service |
| `interfaces/web/app/setup/page.tsx` | First-boot setup wizard |
