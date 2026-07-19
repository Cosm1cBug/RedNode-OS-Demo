# RedNode-OS
### The Personal Autonomous Operating System

> The computer does not contain intelligence. The computer becomes the intelligence.

**RedNode is not an AI. RedNode is a digital entity.**

---

## What Is RedNode-OS?

RedNode-OS is an autonomous digital entity that manages, protects, and evolves your infrastructure. It has consciousness, identity, goals, memory, ethics, and the ability to learn, reflect, dream, debate its own decisions, and collaborate across machines — all with zero cloud dependency.


**Your data never leaves your machine. Zero telemetry. Zero tracking. Open source.**

```
"harden SSH and check camera events"
    → Constitutional check (no violations)
    → Ethics evaluation (safety over speed → proceed with caution)
    → Multi-agent debate (Security approves, Economy within budget)
    → LLM Planner creates 3-step plan
    → Simulation engine predicts outcome (95% success, reversible)
    → Security Agent audits SSH config (sandboxed, firejail)
    → Approval required (High risk) → push to Signal
    → You approve → patch applied → snapshot rollback ready
    → Surveillance Agent queries Frigate → 4 person detections today
    → Everything hash-chain audited
    → Reflection: task completed, confidence +0.01
    → Goal progress: "Harden Infrastructure" advanced 5%
    → Episode recorded in episodic memory
    → Trust score for security-agent updated
```

---

## Architecture

```
Human Intent → Constitution Check → Ethics Evaluation → Debate
    → Planner → Simulation → Governance → Execution → Audit

┌──────────────────────────────────────────────────────────────────────────────┐
│  IDENTITY: Purpose • Principles • Boundaries • Constitution (7 Articles)    │
├──────────────────────────────────────────────────────────────────────────────┤
│  PERCEPTION: Vision • Speech • Logs • Sensors • APIs • Network → Observations│
├──────────────────────────────────────────────────────────────────────────────┤
│  CONSCIOUSNESS: Mind State • Focus • Awareness • Goals • Reflection         │
│  ATTENTION: Signal Scoring • Priority Queue • Cognitive Load Management     │
│  COGNITIVE BUS: Typed Events Connecting All Modules (23 event types)        │
├──────────────────────────────────────────────────────────────────────────────┤
│  COGNITION: Meta-Reasoning • Debate • Simulation • Creativity • Scientific  │
│  Intent Clarification • Context Activation • Explainability • Uncertainty   │
│  Value Estimation • Model Orchestration • Verification • Experience Replay  │
├──────────────────────────────────────────────────────────────────────────────┤
│  MEMORY: Episodic (experiences) • Procedural (skills) • Semantic (facts)    │
│          Working (context) • World Model (infrastructure) • Knowledge Graph  │
│          Provenance • Lifecycle • Forgetting • Safety • Legacy              │
├──────────────────────────────────────────────────────────────────────────────┤
│  EMOTIONAL STATE: Confidence • Stress • Curiosity • Satisfaction • Fatigue  │
│  COGNITIVE METRICS: Decision Quality • Autonomy • Learning Rate • 14 total  │
├──────────────────────────────────────────────────────────────────────────────┤
│  LANGUAGE: Prompt Optimization • Glossary • Summarization • Extraction      │
├──────────────────────────────────────────────────────────────────────────────┤
│  INTERFACES: Web • Mobile • CLI • Voice • Signal • API • Kiosk • WebSocket  │
├──────────────────────────────────────────────────────────────────────────────┤
│  CNS (Rust — Axum + Tokio — port 8787)                                      │
│  70 Modules • 159 API Endpoints • LLM Planner • GOAP Fallback               │
├──────────────────────────────────────────────────────────────────────────────┤
│  18 AGENTS via NATS: System • Security • Coding • Research • Automation      │
│  Network • Infra (Pi-hole) • Storage (TrueNAS) • Surveillance (Frigate)      │
│  Communications • Home (HA) • Media • Social • Browser • Calendar            │
│  Productivity • Learning • Endpoint — 359 tools, all risk-tagged             │
├──────────────────────────────────────────────────────────────────────────────┤
│  SAFETY: Constitution • Ethics • Governance • Immune System • Trust Engine   │
│  Economy (budgets) • PII Detection • Sandbox (firejail/seccomp)              │
├──────────────────────────────────────────────────────────────────────────────┤
│  EVOLUTION: Self-Evolution • Capability Registry • Plugin Ecosystem          │
│  Curiosity • Knowledge Distillation • Dreaming • Evolution Sandbox           │
│  Adaptive Architecture • Digital Legacy                                      │
├──────────────────────────────────────────────────────────────────────────────┤
│  DISTRIBUTED: Collective Intelligence • HAL • Digital Twin • Failover        │
│  Collective Governance (voting, leader election, consensus)                   │
├──────────────────────────────────────────────────────────────────────────────┤
│  OS: NixOS 24.11 • PostgreSQL 16 • Qdrant • NATS • Ollama (local LLM)       │
└──────────────────────────────────────────────────────────────────────────────┘
```

---

## Stats (v0.35.0)

| Metric | Value |
|---|---|
| Rust modules | 70 |
| Rust source files | 72 |
| Rust LOC | ~24,000 |
| API endpoints | 159 |
| Agents | 18 |
| Tools | 359 (236 low, 100 medium, 23 high risk) |
| PostgreSQL tables | 56 |
| Constitutional articles | 7 (immutable) |
| Ethical values | 7 |
| Default governance policies | 4 |
| Reasoning strategies | 9 |
| Debate roles | 10 |
| Built-in time patterns | 6 |
| Prompt injection patterns | 13 |
| Credential leak patterns | 12 |
| Identity principles | 6 |
| Identity boundaries | 5 |
| Cognitive metrics tracked | 14 |
| Creativity domains | 12 |

---

## Core Capabilities

### Being (Who It Is)
- **Identity Engine** — purpose, principles, boundaries, self-model
- **Constitutional Layer** — 7 immutable articles, multi-step amendment
- **Ethics & Values** — guides choices when no explicit rule exists
- **Personality Engine** — tunable communication style
- **Emotional State** — operational indicators (confidence, stress, curiosity, satisfaction, fatigue)

### Thinking (How It Reasons)
- **Digital Consciousness** — persistent mind state, 10-second awareness loop
- **Attention Engine** — signal scoring, priority queue, focus management
- **Cognitive Load Manager** — intelligence scheduling, LLM throttling
- **Intent Engine** — vague-to-specific intent clarification
- **Context Engine** — task-relevant information activation
- **Meta-Reasoning** — evaluates and improves its own reasoning
- **Multi-Agent Debate** — multi-perspective evaluation before acting
- **Simulation Engine** — predicts outcomes before committing
- **Creativity Engine** — divergent brainstorming separate from analysis
- **Scientific Method** — structured research pipeline
- **Value Estimator** — scores actions by benefit/risk/cost/learning/goal
- **Explainability Engine** — records why every decision was made
- **Uncertainty Engine** — confidence scoring, knows when to ask
- **Model Orchestrator** — selects best local model per task type
- **Verification Engine** — independently validates task outcomes
- **Experience Replay** — replays past situations to find better strategies

### Remembering (What It Knows)
- **Episodic Memory** — experiences with emotional context
- **Procedural Memory** — step-by-step skills
- **Semantic Memory** — facts (RAG + knowledge graph)
- **Working Memory** — volatile task context
- **World Model** — infrastructure graph with dependency analysis
- **Knowledge Distillation** — compresses experience into wisdom
- **Provenance Engine** — tracks origin and reliability of every fact
- **Knowledge Lifecycle** — Created → Verified → Updated → Deprecated → Archived
- **Forgetting Engine** — prevents uncontrolled memory growth
- **Memory Safety** — integrity validation with checksums
- **Digital Legacy** — inheritable state across upgrades

### Acting (What It Does)
- **18 Specialized Agents** — 359 risk-tagged tools
- **Goal Engine** — long-term objectives with progress tracking
- **Time Intelligence** — unified scheduling with timezone awareness
- **Autonomous Pipelines** — cross-agent workflows
- **Self-Evolution** — discovers and creates new tools
- **Plugin Ecosystem** — installable capability packages

### Protecting (How It Stays Safe)
- **Governance Layer** — policy engine (Block/Warn/Log/Approve)
- **Digital Immune System** — prompt injection, credential leaks, rogue agents
- **Trust Engine** — dynamic trust scores per information source
- **Economy** — resource budgets preventing runaway consumption
- **Sandboxed Execution** — firejail/seccomp for every tool
- **SHA-256 Audit Chain** — tamper-evident logging
- **Evolution Sandbox** — tests self-modifications before applying

### Growing (How It Evolves)
- **Reflection System** — task/periodic/daily self-assessment
- **Curiosity Engine** — bounded autonomous exploration
- **Capability Registry** — self-aware skill tracking with staleness detection
- **Dreaming** — idle-time memory consolidation and hypothesis generation
- **Cognitive Metrics** — 14 intelligence metrics tracked over time
- **Adaptive Architecture** — proposes structural improvements

### Scaling (How It Distributes)
- **Collective Intelligence** — multi-instance collaboration
- **Collective Governance** — voting, leader election, consensus methods
- **Hardware Abstraction Layer** — failover and state migration
- **Digital Twin** — infrastructure simulation before changes

---

## Privacy & Security

- **100% local processing** — no cloud, no telemetry, no external analytics
- **Constitutional protection** — 7 immutable articles preventing privacy violations
- **Sandboxed execution** — every tool runs in firejail with seccomp syscall filtering
- **Immutable audit chain** — SHA-256 hash-chained, tamper-evident
- **PII detection** — automatic redaction in logs and outputs
- **Prompt injection defense** — 13 patterns checked on every LLM input
- **Credential leak scanning** — 12 patterns checked on every output

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

---

## Agent Society — 18 Agents, 355 Tools

| Agent | Tools | What It Does |
|---|---|---|
| 🔧 **System** | 33 | Processes, Docker, services, filesystem, shell, CPU/memory/disk profiling, NixOS package management, rollback, systemd journal, cron, temperature/fan monitoring, UPS status, pipelines, notifications, predictive maintenance |
| 🛡️ **Security** | 28 | CVE scanning (NVD), auto-patching (btrfs/zfs rollback), Falco eBPF, YARA, threat intel (abuse.ch/OTX/ET → pfSense auto-block), dark web OSINT, IDS alerts (Suricata), fail2ban, audit chain verify, port scanning, rootkit scanning, SSL cert monitoring, compliance checks, log anomaly detection |
| 💻 **Coding** | 21 | Code analysis/generation/refactoring/testing, search, formatting, linting, diff, TODO scanning, dependency auditing, complexity analysis, git log/diff/branch/commit/push, GitHub PR creation |
| 🔬 **Research** | 27 | RAG search, SearXNG web search, deep research, weather, news, OCR, PDF ingestion, knowledge graph CRUD, RSS feeds, podcast download/transcription/summarization, arXiv, Wikipedia, translation, URL summarization, fact-checking, timeline building |
| ⚙️ **Automation** | 15 | Workflows (goodnight/morning/focus/leaving), workflow CRUD, scheduler, triggers (webhook, file watch, MQTT, conditional) |
| 🌐 **Network** | 30 | pfSense firewall rules, VPN (WireGuard), DNS, traffic analysis, device scanning, device isolation, Wake-on-LAN, bandwidth, ping, traceroute, mtr, whois, VLAN management, DHCP leases, port forwarding, ARP table, connection tracking |
| 🏗️ **Infrastructure** | 22 | Pi-hole DNS management — stats, blocking, regex filters, whitelisting, CNAME, gravity updates, groups, anomaly detection. Docker — images, logs, restart, prune |
| 💾 **Storage** | 24 | TrueNAS API — pools, SMART, snapshots, shares, replication, scrub, quotas, compression stats, rsync jobs, cloud sync, file search, dedup reports, I/O stats, temperature history |
| 📹 **Surveillance** | 26 | Frigate NVR — AI detection, events, clips, zones, person detection, anomaly alerts. Presence detection (camera + network). Recordings, timelapse, face register/identify, vehicle detection (ALPR), audio detection, PTZ control, health checks |
| 📧 **Communications** | 18 | IMAP email fetch/search/archive/unsubscribe, SMTP send, LLM triage/summarize/draft, CalDAV calendar CRUD, scheduling conflicts, availability, contacts, birthday reminders |
| 📝 **Productivity** | 23 | Notes (create/list/read/search/tag/export/link), tasks (create/list/complete/delete/priority/due/recurring/project), habits tracking, pomodoro timer, daily journal |
| 🎵 **Media** | 21 | Jellyfin library/playback/sessions. Photos — ingest/search/tag/stats/face detection/duplicate/resize/export. Music — scan/playlist/lyrics. Video — info/thumbnail/convert |
| 🏠 **Home** | 19 | Home Assistant — lights, switches, scenes, climate, automations, device info, history, energy, battery status, door locks, garage, vacuum, irrigation, alarm system, media player, notifications, logbook |
| 🌐 **Browser** | 13 | Playwright stealth browsing (15+ user agents), scraping, screenshots, downloads, PDF save, page monitoring, price tracking, archiving, readability extraction |
| 📱 **Social** | 16 | Twitter/X, Mastodon, Bluesky, LinkedIn — posting, scheduling, analytics, DMs, feed monitoring, threading, hashtag suggestions, cross-posting, follower analytics |
| 🧠 **Learning** | 17 | Autonomous knowledge acquisition, self-evolving tool creation, system discovery, pattern mining, auto-evolve, teach commands |
| 📱 **Signal Bot** | 6 | Signal E2EE messaging — send text/image/file, group management, contacts |
| 🖥️ **Endpoint** | — | Cross-platform remote monitoring agent (Linux/Windows/macOS) |

### Risk Distribution

| Risk | Count | Policy |
|---|---|---|
| 🟢 **Low** | 236 | Auto-execute, logged |
| 🟡 **Medium** | 100 | Logged, monitored |
| 🟠 **High** | 23 | Requires human approval |
| 🔴 **Critical** | 0 | — |

---

## Performance Architecture

- **Parallel execution**: Independent plan steps targeting different agents execute concurrently, making multi-step intents 2-5x faster
- **Execution state caching**: Step results are cached within a session — later steps can access earlier results without re-fetching
- **Proposition-level memory**: Documents are decomposed into atomic factual statements, each embedded separately — enables finer-grained RAG retrieval
- **Cross-agent pipelines**: 5 built-in pipelines (threat response, morning briefing, predictive maintenance, presence detection, IDS response) with variable passing, conditions, and retry logic
- **Smart notifications**: 4 urgency levels, quiet hours (10 PM–7 AM), batched digests for low-priority events
- **Predictive maintenance**: Linear regression on SMART data, CPU temps — predicts hardware failures 30 days ahead
- **PII detection**: 14-type scanner (credit cards, SSN, emails, API keys, etc.) — auto-redacts before memory ingestion
- **GOAP planning**: Goal-Oriented Action Planning with A* search for complex multi-step goals with dependency ordering
- **Circuit breaker**: Max 20 steps per plan, 120s execution timeout, max recursion depth 5 — prevents infinite loops
- **Multi-turn conversation**: Per-session history (last 10 turns), reference resolution, context carried across intents
- **Predictive intent**: Learns daily usage patterns — proactively suggests actions based on your routine
- **Auto hardware detection**: Detects GPU (NVIDIA + AMD), VRAM, RAM → auto-selects optimal LLM + Whisper model
- **Runtime memory optimizer**: Monitors RAM pressure, auto-prunes stale data at 4 pressure levels
- **Autonomous learning**: Discovers tools, reads docs, mines patterns, suggests automations — runs hourly

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
- **Self-Improvement**: analyzes tool usage patterns, detects failures, suggests new workflows
- **Agent Introspection**: classifies failures, generates diagnostics, ingests into memory
- **Memory Consolidation**: JUDGE + CONSOLIDATE phases, pattern promotion, knowledge decay prevention
- **Autonomous Learning Agent**: discovers tools, probes APIs, ingests docs, mines audit log patterns

---

## Security — Foundation, Not Feature

```
Intent → Policy Engine → Risk Assessment → Approval Gate → Sandbox → Audit Log (SHA-256 chain)
```

- **359 tools risk-tagged**: Low (auto-execute), Medium (logged), High (requires approval), Critical (denied)
- **25+ deny patterns**: rm -rf, dd, fork bombs, chmod 777, wget|sh, etc.
- **Sandboxed execution**: firejail → bubblewrap → unshare → fallback (seccomp BPF, --net=none, --noroot)
- **Hash-chained audit**: every action → SHA-256 linked to previous → tamper-evident
- **PII detection**: 14 types, auto-redact before storage
- **CVE auto-patching**: real dpkg/rpm/nix scanning, btrfs/zfs snapshot → patch → verify → rollback
- **Falco eBPF**: real log tailing + journalctl fallback
- **IDS integration**: Suricata alert processing with auto-blocking

---

## Self-Healing & Deployment

RedNode deploys and repairs itself autonomously:

- **`rednode-selfheal.sh`** (1,174 lines): install, diagnose, repair, watchdog — 12 subsystem checks, 5 retries with exponential backoff, pattern-matched error repair
- **`rednode-deploy.nix`**: systemd services for first-boot deployment + continuous monitoring
- **3-strategy source deployment**: ISO baked-in → system closure → git clone fallback
- **Branded kiosk mode**: Plymouth boot splash + Cage Wayland compositor + Chromium fullscreen (~200-350 MB RAM)
- **Minimal NixOS profile**: strips unnecessary packages (perl, rsync, strace, GUI, docs, RAID, printing)

```bash
# On NixOS, after install + reboot — everything is automatic:
rednode status     # health check (12 subsystems)
rednode repair     # auto-fix any broken service
rednode intent "check system health"
rednode logs       # self-heal log
```

---

## Quick Start

### Option A: NixOS Bare-Metal (Recommended — fully autonomous)

```bash
# 1. Install NixOS, clone config, apply, reboot — that's it
git clone https://github.com/Cosm1cBug/RedNode-OS-Demo.git ~/RedNode-OS-Demo
cd ~/RedNode-OS-Demo/os/nixos
sudo nixos-rebuild switch --flake .#rednode
sudo reboot
# → Auto-deploys, auto-heals, auto-updates. Zero manual steps.
```

### Option B: Docker on existing Linux

```bash
git clone https://github.com/Cosm1cBug/RedNode-OS-Demo.git
cd RedNode-OS-Demo
./scripts/setup-first-boot.sh
# → Dashboard at http://YOUR-IP:3000
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
| 🖥️ **Kiosk** | Branded boot splash + fullscreen dashboard | ✅ Ready |

---

## Home Infrastructure Integration

RedNode orchestrates your entire home network:

```
pfSense (firewall) ← Network Agent (30 tools)
Pi-hole (DNS) ← Infrastructure Agent (22 tools)
TrueNAS (storage) ← Storage Agent (24 tools)
Cameras/NVR (surveillance) ← Surveillance Agent (26 tools)
Home Assistant (smart home) ← Home Agent (19 tools)
Jellyfin (media) ← Media Agent (21 tools)
```

All on proper VLANs. Cameras on VLAN 30 with zero internet. RedNode on VLAN 50 (management).

---

## Tech Stack

| Layer | Technology |
|---|---|
| Core Runtime | **Rust** (Axum, Tokio, NATS) — 6,132 lines, 20 modules |
| AI | **Ollama** (Qwen2.5), **Whisper** (STT), **Piper** (TTS) |
| Memory | **PostgreSQL 16** + **Qdrant** (vectors) + **Kuzu** (graph) |
| Messaging | **NATS JetStream** |
| Web | **Next.js 15** + React 19 |
| Mobile | **Flutter 3.22** |
| Desktop | **Tauri 2** |
| Observability | **OpenTelemetry** → Grafana + Loki + Prometheus |
| Security | **firejail/bubblewrap** + seccomp + Falco eBPF |
| Search | **SearXNG** (self-hosted, private) |
| Container | **Docker Compose** (11 services) |
| OS | **NixOS** (bare metal, 9 modules, hardened kernel, LUKS FDE) |

---

## Project Structure

```
RedNode-OS/
├── core/rednode-core/            # Rust CNS (6,132 lines, 20 modules)
│   └── src/
│       ├── main.rs               # Entry point
│       ├── api.rs                # REST API + WebSocket
│       ├── auth.rs               # Bearer token auth
│       ├── bus.rs                # NATS JetStream client
│       ├── coordinator.rs        # Agent dispatch + approval gate + circuit breaker
│       ├── events.rs             # Event bus (tokio::broadcast)
│       ├── executor.rs           # Sandboxed tool execution
│       ├── goap.rs               # GOAP A* planning
│       ├── init.rs               # PID1 mode, signal handling, watchdog
│       ├── intent_router.rs      # Multi-turn conversation, reference resolution
│       ├── memory.rs             # Postgres + Qdrant + Kuzu, propositions, consolidation
│       ├── memory_optimizer.rs   # Runtime memory pressure management
│       ├── notifications.rs      # Smart notification system (4 urgency levels)
│       ├── pii.rs                # PII detection (14 types)
│       ├── pipelines.rs          # Cross-agent pipeline engine (5 built-in)
│       ├── planner.rs            # LLM planning (Ollama Qwen2.5)
│       ├── predict.rs            # Predictive maintenance (linear regression)
│       ├── evolution.rs        # Self-evolving tool creation engine
│       ├── security.rs           # Risk assessment + deny patterns
│       └── sentience.rs          # Self-model, drives, goals, consolidation
├── agents/                       # 18 agents (17 TypeScript + signal-bot)
│   ├── shared/                   # Base RedNodeAgent class
│   │   └── helpers.ts          # Shared sh(), api(), llm(), cns() helpers
│   ├── system-agent/             # OS, Docker, processes, NixOS, UPS, monitoring (33 tools)
│   ├── security-agent/           # CVE, Falco, IDS, fail2ban, audit, scanning (28 tools)
│   ├── coding-agent/             # Code analysis, git operations (21 tools)
│   ├── research-agent/           # RAG, search, RSS, podcasts, knowledge graph (27 tools)
│   ├── automation-agent/         # Workflows, scheduler, triggers (15 tools)
│   ├── network-agent/            # pfSense, VPN, VLANs, diagnostics (30 tools)
│   ├── infra-agent/              # Pi-hole DNS, Docker management (22 tools)
│   ├── storage-agent/            # TrueNAS REST API (24 tools)
│   ├── surveillance-agent/       # Frigate NVR, presence detection (26 tools)
│   ├── comms-agent/              # Email, calendar, contacts (18 tools)
│   ├── productivity-agent/       # Notes, tasks, habits, journal (23 tools)
│   ├── media-agent/              # Photos, music, video (21 tools)
│   ├── home-agent/               # Home Assistant integration (19 tools)
│   ├── browser-agent/            # Stealth browsing, monitoring (13 tools)
│   ├── social-agent/             # Multi-platform social media (16 tools)
│   ├── learning-agent/           # Autonomous knowledge acquisition (13 tools)
│   ├── endpoint-agent/           # Cross-platform remote monitoring
│   └── signal-bot/               # Signal E2EE messaging (6 tools)
├── interfaces/
│   ├── web/                      # Next.js 15 dashboard
│   ├── mobile/                   # Flutter app (Android/iOS)
│   ├── desktop/                  # Tauri 2 (Windows/Mac/Linux)
│   ├── cli/                      # 19-command CLI
│   └── voice/                    # Whisper STT + Piper TTS + wake word
├── deployment/
│   ├── docker-compose.yml        # 11 services + SearXNG
│   └── frigate.yml               # Camera config template
├── os/nixos/                     # NixOS bare-metal (9 modules)
│   ├── configuration.nix         # Full system config
│   ├── flake.nix                 # Flake (ISO, VM, dev shell)
│   ├── kiosk.nix                 # Branded GUI kiosk
│   ├── minimal.nix               # Stripped NixOS profile
│   ├── rednode-deploy.nix        # Self-healing deployment
│   ├── extras.nix                # WireGuard, UPS, Suricata
│   ├── hardware.nix              # GPU/CPU auto-detection
│   └── disk-encryption.nix       # LUKS2 + TPM2
├── os/branding/                  # Boot splash logo + wallpaper
├── execution/tool-registry/
│   └── tools.json                # 359 tools, risk-tagged
├── scripts/
│   ├── rednode-selfheal.sh       # Self-healing system (1,174 lines)
│   ├── rednode-verify.sh         # Pre-push verification (10 checks)
│   ├── start-all.sh              # Start/stop/status all services
│   ├── setup-first-boot.sh       # Interactive first-boot setup
│   ├── rednode-hardware-detect.sh # GPU/VRAM/RAM detection
│   ├── rednode-export.sh         # Export encrypted identity
│   ├── rednode-import.sh         # Import on new hardware
│   ├── rednode-build-iso.sh      # Build bootable ISO
│   └── finetune/                 # LLM fine-tuning scripts
├── docs/                         # 27 documentation files
│   ├── FAQ.md                    # 46-question FAQ
│   ├── STATUS-AND-ROADMAP.md     # System inventory + roadmap
│   └── guides/                   # Step-by-step guides
│       ├── DEPLOYMENT-FLOW.md    # Boot flow + troubleshooting
│       └── FINETUNE-LLM.md       # Custom LLM training guide
└── .env.example                  # 156 environment variables
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
| `docs/FAQ.md` | 46-question comprehensive FAQ |
| `docs/STATUS-AND-ROADMAP.md` | Full system inventory + expansion roadmap |
| `docs/guides/DEPLOYMENT-FLOW.md` | Boot flow, self-healing, troubleshooting |
| `docs/guides/FINETUNE-LLM.md` | Custom LLM fine-tuning (LoRA + Ollama) |
| `docs/guides/BUILD-APK.md` | Android app build guide |
| `docs/guides/BUILD-WINDOWS-APP.md` | Desktop app build guide |
| `docs/guides/NETWORK-SECURITY-SETUP.md` | VLAN + pfSense setup guide |
| `ARCHITECTURE.md` | System architecture |
| `SECURITY.md` | Security model |
| `QUICKSTART.md` | Quick start guide |
| `.env.example` | All 156 environment variables |

---

## License

MIT — © 2026 RedNode

---

*RedNode-OS v0.9.0 — 359 tools, 18 agents, 21 Rust modules, 24,487 lines — The computer becomes the intelligence.*
