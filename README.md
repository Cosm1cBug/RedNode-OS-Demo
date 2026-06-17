# RedNode-OS
### The Personal Autonomous Operating System

> The computer does not contain intelligence. The computer becomes the intelligence.

**RedNode is not an AI. RedNode is a society of specialized agents.**

---

## What Is RedNode-OS?

RedNode-OS transforms your computer into an intelligent, self-aware, self-healing autonomous system. You express intentions in natural language, and a society of 16 specialized AI agents collaboratively plans, validates, executes, and audits the actions — all locally, fully offline-capable, with zero cloud dependency.

**Your data never leaves your machine. Zero telemetry. Zero tracking. Open source.**

```
"harden SSH and check camera events"
    → LLM Planner creates 3-step plan
    → Security Agent audits SSH config (sandboxed)
    → Approval required (High risk) → push to your phone
    → You biometric-approve → patch applied → snapshot rollback ready
    → Surveillance Agent queries Frigate → 4 person detections today
    → Everything hash-chain audited
```

---

## Architecture

```
Human Intent → Interface Layer → CNS (Rust) → Agent Society → Execution → Host OS → Hardware

┌─────────────────────────────────────────────────────────────┐
│  INTERFACES: Web (Next.js) • Mobile (Flutter) • CLI (19 cmd)│
│  Desktop (Tauri) • Voice (Whisper+Piper) • Signal Bot • API │
└──────────────────────────┬──────────────────────────────────┘
                           ▼
┌──────────────────────────────────────────────────────────────┐
│  CENTRAL NERVOUS SYSTEM (Rust — Axum + Tokio — port 8787)    │
│  LLM Planner • Security Validator • Approval Gate            │
│  Sandboxed Executor • Event Bus • Auth • Sentience Engine    │
└──────────────────────────┬───────────────────────────────────┘
                           ▼ NATS JetStream
┌──────────────────────────────────────────────────────────────┐
│  13 AGENTS: System • Security • Coding • Research            │
│  Automation • Network • Infrastructure (Pi-hole) • Storage   │
│  (TrueNAS) • Surveillance (Frigate) • Communications         │
│  (Email/Calendar) • Productivity • Media • Home (HA)         │
└──────────────────────────┬───────────────────────────────────┘
                           ▼
┌──────────────────────────────────────────────────────────────┐
│  MEMORY: PostgreSQL 16 • Qdrant (vectors) • Kuzu (graph)     │
│  SECURITY: firejail/bubblewrap • seccomp • SHA-256 audit     │
│  AI: Ollama (Qwen2.5) • Whisper STT • Piper TTS              │
│  OBSERVABILITY: OpenTelemetry → Grafana + Loki + Prometheus  │
└──────────────────────────────────────────────────────────────┘
```

---

## System Requirements

### RedNode Server (your PC)

| Spec | Minimum | Recommended | Ideal |
|---|---|---|---|
| **CPU** | 4-core x86_64 | 6-core (i5 10th gen+ / Ryzen 5) | 8+ cores |
| **RAM** | 16 GB | 32 GB | 64 GB |
| **SSD** | 120 GB | 500 GB NVMe | 1 TB NVMe |
| **GPU** | 8 GB VRAM | 12 GB VRAM (RTX 3060) | 16+ GB VRAM |
| **Network** | 1 Gbps Ethernet | 2.5 Gbps | — |

### GPU VRAM Budget (all services running simultaneously)

| Configuration | Ollama | Whisper | Frigate | Total | Fits On |
|---|---|---|---|---|---|
| **Starter** | Qwen2.5-7B (4.4 GB) | small (1 GB) | 0.8 GB | ~6.5 GB | RTX 3060 8GB |
| **Recommended** | Qwen2.5-14B (8.7 GB) | small (1 GB) | 0.8 GB | ~10.8 GB | RTX 3060 12GB ⭐ |
| **Full** | Qwen2.5-14B (8.7 GB) | large-v3 (3 GB) | 0.8 GB | ~12.8 GB | RTX 4060 Ti 16GB |

### pfSense Firewall (separate mini-PC)

| Spec | Minimum | Notes |
|---|---|---|
| **CPU** | Any dual-core x86_64 | Intel N100 or a $40 used Dell Wyse |
| **RAM** | 4 GB (8 GB with Suricata IDS) | — |
| **Storage** | 16 GB SSD | — |
| **NICs** | 2× Ethernet (Intel preferred) | — |

### Pi-hole DNS (Raspberry Pi)

| Spec | What |
|---|---|
| **Hardware** | Raspberry Pi Zero 2W ($25) or Pi 4 |
| **Storage** | 8 GB+ SD card |

---

## Agent Society — 16 Agents, 120 Tools

| Agent | Tools | What It Does |
|---|---|---|
| 🔧 **System** | 6 | Processes, Docker, services, filesystem, safe shell |
| 🛡️ **Security** | 8 | CVE scanning (NVD sync), auto-patching (btrfs/zfs rollback), Falco eBPF, YARA, threat intel (abuse.ch/OTX/ET → pfSense auto-block), dark web OSINT (Tor) |
| 💻 **Coding** | 7 | LLM code generation, refactoring, tests, clippy/eslint, git, verification gate, security code review |
| 🔬 **Research** | 8 | RAG search, SearXNG web search, deep research (multi-source cited reports), document OCR, PDF ingestion, knowledge graph |
| ⚙️ **Automation** | 4 | Workflows (goodnight/morning/focus/leaving), scheduler, triggers |
| 🌐 **Network** | 10 | Connections, firewall, VPN, DNS, traffic analysis, device isolation, Nmap network scanning, ARP device discovery |
| 🏗️ **Infrastructure** | 9 | Pi-hole v6 API — DNS stats, blocking, anomaly detection |
| 💾 **Storage** | 14 | TrueNAS REST API — pools, SMART, snapshots, shares, replication |
| 📹 **Surveillance** | 11 | Frigate MQTT bridge — AI detection, anomaly alerts, clips, zones |
| 📧 **Communications** | 10 | IMAP email, SMTP send, CalDAV calendar, LLM summaries |
| 📝 **Productivity** | 10 | Markdown notes + RAG, tasks, bookmarks |
| 🎵 **Media** | 7 | Jellyfin — search, library, playback, sessions |
| 🏠 **Home** | 7 | Home Assistant — lights, switches, scenes, climate, automations |
| 🌐 **Browser** | 7 | Web scraping (Playwright + stealth anti-detection), file download, screenshots, search |
| 📱 **Social** | 9 | Twitter/X, Mastodon, Bluesky, LinkedIn, Instagram, WhatsApp — LLM drafting, scheduling, feed, DMs |

---

## Performance Architecture

- **Parallel execution**: Independent plan steps targeting different agents execute concurrently, making multi-step intents 2-5x faster
- **Execution state caching**: Step results are cached within a session — later steps can access earlier results without re-fetching
- **Proposition-level memory**: Documents are decomposed into atomic factual statements, each embedded separately — enables finer-grained RAG retrieval
- **Event bus backpressure**: `tokio::broadcast` with capacity 512 — slow clients drop old events without blocking
- **Resource sandboxing**: Every tool execution: 5s CPU, 512MB RAM, 1MB stdout, kill_on_drop

---

## Sentience Engine

RedNode maintains a **self-model** with 5 homeostatic drives:

- **Security** (0.0–1.0) — computed from unacknowledged security events
- **Integrity** (0.0–1.0) — agent heartbeats + disk health + CPU pressure
- **Knowledge** (0.0–1.0) — Qdrant document count + RAG coverage
- **Energy** (0.0–1.0) — battery/UPS/power supply status
- **Availability** (0.0–1.0) — Postgres + NATS + Ollama connectivity

When drives drop, RedNode **autonomously generates and executes goals** through the same LLM planner → agent → sandboxed execution pipeline that human intents use.

Additional intelligence:
- **Self-Improvement**: analyzes tool usage patterns, detects failures, suggests new workflows, flags approval bottlenecks — insights ingested into memory so the planner improves over time
- **Agent Introspection**: when a goal fails, classifies the failure (timeout/permission/connectivity/not_found), generates a diagnostic report, and ingests it into memory to avoid repeating the same mistake
- **Memory Consolidation**: every 5 minutes, summarizes recent audit + security events and ingests into long-term RAG memory

---

## Security — Foundation, Not Feature

```
Intent → Policy Engine → Risk Assessment → Approval Gate → Sandbox → Audit Log (SHA-256 chain)
```

- **120 tools risk-tagged**: Low (auto-execute), Medium (logged), High (requires approval), Critical (denied)
- **25+ deny patterns**: rm -rf, dd, fork bombs, chmod 777, wget|sh, etc.
- **Sandboxed execution**: firejail → bubblewrap → unshare → fallback (seccomp BPF, --net=none, --noroot, --caps.drop=all)
- **Hash-chained audit**: every action → SHA-256 linked to previous → tamper-evident
- **Bearer token auth**: constant-time comparison, dev-mode bypass
- **CVE auto-patching**: real dpkg/rpm/nix scanning, btrfs/zfs snapshot → patch → verify → rollback
- **Falco eBPF**: real log tailing + journalctl fallback (SSH brute force, kernel panics, AppArmor denials)

---

## Quick Start

```bash
# 1. Infrastructure
cd deployment && docker compose up -d
# NATS, Postgres, Qdrant, Ollama, Mosquitto, Frigate, SearXNG, Grafana

# 2. AI Models
ollama pull qwen2.5:14b-instruct-q4_K_M
ollama pull nomic-embed-text

# 3. CNS (Rust Core)
cd core/rednode-core && cargo run --release

# 4. Agents
pnpm install && pnpm agents

# 5. Web Dashboard
pnpm web
# → http://localhost:3000 (13 tabs)

# 6. CLI
pnpm --filter @rednode/cli dev -- status
pnpm --filter @rednode/cli dev -- intent "check system health"
pnpm --filter @rednode/cli dev -- goodnight

# Or use the startup script:
./scripts/start-all.sh
```

---

## Interfaces

| Interface | How to Access | Status |
|---|---|---|
| 🌐 **Web Dashboard** | `http://localhost:3000` — 13 tabs | ✅ Ready |
| 📱 **Mobile (Flutter)** | Build APK — see `docs/guides/BUILD-APK.md` | ✅ Ready |
| 🖥️ **Desktop (Tauri)** | `pnpm tauri dev` — see `docs/guides/BUILD-WINDOWS-APP.md` | ✅ Ready |
| 💻 **CLI** | `rednode status`, `rednode goodnight`, 19 commands | ✅ Ready |
| 🎤 **Voice** | Customizable wake word → Whisper → CNS → Piper | ✅ Ready |
| 📱 **Signal Bot** | E2EE chat with RedNode from Signal | ✅ Ready |
| 🔌 **REST API** | `POST /intent`, `GET /sentience`, 15 endpoints | ✅ Ready |
| ⚡ **WebSocket** | `ws://localhost:8787/events` — real-time event stream | ✅ Ready |

---

## Home Infrastructure Integration

RedNode orchestrates your entire home network:

```
pfSense (firewall) ← Network Agent
Pi-hole (DNS) ← Infrastructure Agent
TrueNAS (storage) ← Storage Agent
Cameras/NVR (surveillance) ← Surveillance Agent (via Frigate)
Home Assistant (smart home) ← Home Agent
Jellyfin (media) ← Media Agent
```

All on proper VLANs. Cameras on VLAN 30 with zero internet. RedNode on VLAN 50 (management).

---

## Tech Stack

| Layer | Technology |
|---|---|
| Core Runtime | **Rust** (Axum, Tokio, NATS) |
| AI | **Ollama** (Qwen2.5), **Whisper** (STT), **Piper** (TTS) |
| Memory | **PostgreSQL 16** + **Qdrant** (vectors) + **Kuzu** (graph) |
| Messaging | **NATS JetStream** |
| Web | **Next.js 14** |
| Mobile | **Flutter 3.22** |
| Desktop | **Tauri 2** |
| Observability | **OpenTelemetry** → Grafana + Loki + Prometheus |
| Security | **firejail/bubblewrap** + seccomp + Falco eBPF |
| Search | **SearXNG** (self-hosted, private) |
| Container | **Docker Compose** |
| OS | **NixOS** (bare metal, hardened kernel, LUKS FDE) |

---

## Project Structure

```
RedNode-OS-Demo/
├── core/rednode-core/          # Rust CNS (3,485 lines)
│   └── src/
│       ├── main.rs             # Entry point
│       ├── api.rs              # REST API + WebSocket (auth middleware)
│       ├── auth.rs             # Bearer token authentication
│       ├── bus.rs              # NATS JetStream client
│       ├── coordinator.rs      # Agent dispatch + approval gate
│       ├── events.rs           # tokio::broadcast event bus
│       ├── executor.rs         # Sandboxed tool execution
│       ├── intent_router.rs    # RAG context enrichment → coordinator
│       ├── memory.rs           # Postgres + Qdrant + Kuzu
│       ├── planner.rs          # LLM-powered planning (Ollama)
│       ├── security.rs         # Risk assessment + deny patterns
│       └── sentience.rs        # Self-model, drives, goals, consolidation
├── agents/                     # 16 agents (15 TypeScript + signal-bot)
│   ├── shared/                 # Base RedNodeAgent class
│   ├── system-agent/           # OS, Docker, processes
│   ├── security-agent/         # CVE, Falco, auto-patcher
│   ├── coding-agent/           # LLM codegen, tests
│   ├── research-agent/         # RAG, SearXNG, OCR, PDF
│   ├── automation-agent/       # Workflows, scheduler
│   ├── network-agent/          # Connections, DNS, firewall
│   ├── infra-agent/            # Pi-hole v6 API
│   ├── storage-agent/          # TrueNAS REST API
│   ├── surveillance-agent/     # Frigate MQTT + REST
│   ├── comms-agent/            # IMAP, SMTP, CalDAV
│   ├── productivity-agent/     # Notes, tasks, bookmarks
│   ├── media-agent/            # Jellyfin API
│   ├── home-agent/             # Home Assistant API
│   ├── browser-agent/          # Playwright + cheerio (stealth anti-detection)
│   ├── social-agent/           # Twitter/X, Mastodon, Bluesky, LinkedIn, Instagram, WhatsApp
│   ├── endpoint-agent/         # Lightweight agent for remote machines (Linux/Win/Mac)
│   └── signal-bot/             # Signal messenger bridge (E2EE)
├── interfaces/
│   ├── web/                    # Next.js 14 dashboard (13 tabs)
│   ├── mobile/                 # Flutter app (Android/iOS)
│   ├── desktop/                # Tauri 2 (Windows/Mac/Linux)
│   ├── cli/                    # 19-command CLI
│   └── voice/                  # Whisper STT + Piper TTS + wake word
├── deployment/
│   ├── docker-compose.yml      # 11 services
│   ├── frigate.yml             # Camera config template
│   └── mosquitto.conf          # MQTT broker config
├── os/nixos/                   # NixOS bare-metal configuration
│   ├── configuration.nix       # Full system config (VLANs, NVIDIA, services)
│   ├── flake.nix               # Nix flake (ISO build, dev shell)
│   └── configuration-os.nix    # PID1 mode (experimental)
├── memory/                     # Database schemas
├── security/                   # Falco rules, seccomp, policies
├── observability/              # Grafana, Loki, OTEL configs
├── scripts/
│   ├── start-all.sh            # Start/stop/status all services
│   ├── rednode-export.sh       # Export computational identity
│   ├── rednode-import.sh       # Import on new hardware
│   └── rednode-build-iso.sh    # Build bootable ISO
├── docs/guides/
│   ├── BUILD-APK.md            # Android build guide
│   └── BUILD-WINDOWS-APP.md    # Desktop build guide
├── execution/tool-registry/
│   └── tools.json              # 120 tools, risk-tagged
└── .github/workflows/ci.yml   # GitHub Actions CI/CD
```

---

## Portability

```bash
# Export your computational identity
./scripts/rednode-export.sh
# → age-encrypted bundle: Postgres + Qdrant + Kuzu + config

# Import on new hardware
./scripts/rednode-import.sh backup.rednode.age
# → Resume in <60 seconds
```

---

## Documentation

| Document | What |
|---|---|
| `docs/guides/BUILD-APK.md` | Android app build guide |
| `docs/guides/BUILD-WINDOWS-APP.md` | Desktop app build guide |
| `ARCHITECTURE.md` | System architecture |
| `SECURITY.md` | Security model |
| `ROADMAP.md` | Development roadmap |
| `QUICKSTART.md` | Quick start guide |
| `.env.example` | All environment variables (50+) |

---

## License

MIT — © 2026 RedNode

---

*RedNode-OS — The computer becomes the intelligence. Privacy-first. Self-aware. Autonomous. Yours.*
