# RedNode-OS Roadmap

> *The computer does not contain intelligence. The computer becomes the intelligence.*
>
> **Current State**: v0.9.0 — 18 agents, 359 tools, Rust CNS, Sentience Engine (self-improvement + introspection), LLM planner, RAG memory, knowledge graph, 13-tab dashboard, Flutter mobile, Tauri desktop, 19-command CLI, voice (Whisper+Piper), Signal bot, threat intel, NVD sync, deep research, verification gate, PID1 init, PII detection, GOAP planning, circuit breaker  
> **Target**: Production deployment, commercial product

---

## Phase 1 – Foundation ✅ (Complete – 4 weeks)

**Status: DONE — Core pipeline is functional**

- [x] CNS Rust core (Axum + Tokio) — intent routing, planning, coordination
- [x] NATS JetStream bus — agent communication backbone
- [x] PostgreSQL 16 + Qdrant + Kuzu memory layer
- [x] 6-agent framework (System, Security, Coding, Research, Automation, Network)
- [x] 23 risk-tagged tools in Tool Registry
- [x] Security Engine — risk assessment, approval gates, deny patterns
- [x] Sandboxed executor — firejail/bubblewrap/unshare + seccomp
- [x] SHA-256 hash-chained audit log (tamper-evident)
- [x] RAG pipeline — Qdrant vector search + Ollama embeddings + 3-tier fallback
- [x] Sentience Engine — self-model, 5 homeostatic drives, goal generation, memory consolidation
- [x] Next.js 14 dashboard — 8-tab SOC console
- [x] Flutter 3.22 mobile app — FCM push, biometric approval, WireGuard, secure storage
- [x] Tauri 2 desktop app
- [x] TypeScript CLI
- [x] NixOS bare-metal configuration — flake.nix, ISO build, hardened kernel
- [x] Docker Compose deployment (NATS, Postgres, Qdrant, Ollama, Grafana stack)
- [x] Security Agent — CVE auto-checker (6h interval), auto-patcher with snapshot rollback, Falco eBPF bridge

**MVP Exit Criteria**: Intent → Plan → Execute → Audit loop ✅ | Local LLM ✅ | Full audit ✅

---

## Phase 2 – Intelligence Layer (4 weeks)

**Goal: Replace keyword planner with LLM-powered reasoning. Add voice. Deepen agent collaboration.**

- [ ] **LLM-Powered Planner** — replace keyword matching with Qwen2.5 ReAct reasoning
  - Intent → LLM → structured PlanStep[] with tool/agent/risk assignment
  - Multi-step reasoning: "analyze this log, find the issue, fix it, verify"
  - Context-aware: planner uses RAG memory for informed decisions
- [ ] **Agent Collaboration Protocol** — agents can delegate to each other via NATS
  - Security Agent requests System Agent to check service status
  - Research Agent enriches Coding Agent with RAG context
  - Coordinator manages dependency chains
- [ ] **Voice Loop** — complete Whisper STT + Piper TTS pipeline
  - Wake word detection (OpenWakeWord)
  - Whisper → intent → CNS → response → Piper
  - Target: voice-to-response < 1.2s
- [ ] **Research Engine** — deep RAG with multi-source synthesis
  - Web search via SearXNG (self-hosted, privacy-first)
  - Document chunking + semantic indexing
  - Knowledge graph entity extraction → Kuzu
- [ ] **Automation Workflows** — Automation Agent DAG engine
  - "Every morning at 8am, pull git repos and run tests"
  - Cron-like scheduling + event-triggered workflows
  - Temporal/Temporalite integration (optional)
- [ ] **Conversation Memory** — multi-turn context within sessions
  - Session-level working memory
  - Cross-session episodic memory
  - User preference learning

---

## Phase 3 – Security Layer (4 weeks)

**Goal: Production-grade SOC. Self-healing. Complete threat pipeline.**

- [ ] **Threat Intel Pipeline** — NVD CVE sync + YARA rules + abuse.ch feeds
  - Offline CVE DB with scheduled sync (consent-based)
  - YARA rule auto-update from community feeds
  - IP reputation scoring
- [ ] **eBPF Deep Integration** — beyond Falco
  - Custom eBPF programs for RedNode-specific monitoring
  - Syscall anomaly detection per agent process
  - Network flow analysis within VLANs
- [ ] **Self-Healing Engine** — detect → isolate → patch → verify → report
  - Automatic service restart on agent failure
  - Sentience Engine integrity drive triggers recovery
  - Snapshot → patch → test → rollback-on-failure pipeline
- [ ] **Smart Security Mode v2** — graduated autonomous response
  - Low: log + alert
  - Medium: log + alert + recommend action
  - High: log + alert + auto-execute (with audit)
  - Critical: log + alert + isolate + snapshot + await human approval
- [ ] **Compliance Reporting** — generate security posture reports
  - CIS benchmark scoring
  - Audit log export (JSON, CSV, PDF)
  - SBOM generation for all running software

---

## Phase 4 – Home Infrastructure Integration (6 weeks) 🆕

**Goal: RedNode becomes the central brain for your entire home infrastructure.**

### Phase 4a – Infrastructure Agent + Pi-hole (2 weeks)

- [ ] **Infrastructure Agent** — new agent, 9 tools
  - `pihole.stats` — DNS query statistics, top blocked, top clients
  - `pihole.disable` / `pihole.enable` — temporary disable with timer
  - `pihole.add_block` / `pihole.remove_block` — manage blocklists
  - `pihole.anomaly` — detect unusual DNS query patterns (C2 callbacks)
  - `pihole.groups` — per-VLAN blocking policies
- [ ] Pi-hole v6 REST API integration (`/api/auth`, `/api/stats/summary`, `/api/dns/blocking`)
- [ ] DNS anomaly detection → Security Agent correlation
- [ ] "Focus mode" workflow — block social media during work hours

### Phase 4b – Storage Agent + TrueNAS (2 weeks)

- [ ] **Storage Agent** — new agent, 14 tools
  - `nas.health` — pool health, disk SMART, temperatures
  - `nas.datasets` / `nas.usage` — dataset info, space usage
  - `nas.snapshot_create` / `nas.snapshot_list` / `nas.snapshot_delete`
  - `nas.share_create` / `nas.share_list` — SMB/NFS management
  - `nas.alerts` — active TrueNAS alerts
  - `nas.replicate` — trigger replication jobs
  - `nas.backup_rednode` — backup RedNode memory (Postgres/Qdrant) to TrueNAS
- [ ] TrueNAS REST API v2.0 integration (`/api/v2.0/pool`, `/api/v2.0/zfs/snapshot`)
- [ ] Pre-action snapshots: before any High/Critical tool execution → auto-snapshot TrueNAS
- [ ] Storage health → Sentience Engine integrity drive
- [ ] Nightly RedNode brain backup to TrueNAS

### Phase 4c – Surveillance Agent + Frigate NVR (2 weeks)

- [ ] **Surveillance Agent** — new agent, 12 tools
  - `cam.status` — all cameras online/offline
  - `cam.events` — Frigate detection events (person, car, animal, package)
  - `cam.snapshot` / `cam.clip` — retrieve event snapshots and video clips
  - `cam.search` — search events by camera, object type, time range, zone
  - `cam.alert_config` — configure zone-based alerts
  - `cam.anomaly` — unusual activity detection (time-of-day based)
  - `cam.review` — AI-generated daily review summaries (Frigate v0.17+)
- [ ] Frigate NVR Docker deployment on RedNode server
  - RTSP stream ingestion from standalone NVR (cross-VLAN, port 554)
  - GPU-accelerated detection (TensorRT) or Coral USB
  - MQTT event bridge → NATS → Surveillance Agent
- [ ] Frigate REST API integration (`/api/events`, `/api/stats`, `/api/reviews`)
- [ ] Camera alerts → Security Agent correlation
  - Person at 2am + suspicious DNS queries = escalated CRITICAL alert
- [ ] Push notifications with event snapshots to mobile app (FCM)

### Phase 4d – Cross-System Workflows (1 week)

- [ ] **"Goodnight" workflow** — Pi-hole strict mode + camera night alerts + IoT firewall block + TrueNAS snapshot + memory consolidation
- [ ] **"I'm leaving" workflow** — all cameras active + WireGuard tunnel + mobile-only alerts
- [ ] **"Focus mode" workflow** — block social media (Pi-hole) + disable notifications + Pomodoro timer
- [ ] **Autonomous incident response** — Frigate + Pi-hole + firewall correlation → isolate device + snapshot evidence + alert owner
- [ ] Sentience Engine drive updates from all infrastructure:
  - Security drive ← Falco + Pi-hole anomalies + Frigate unusual detections
  - Integrity drive ← TrueNAS pool health + disk SMART + camera online status
  - Energy drive ← UPS battery status
  - Availability drive ← Storage capacity + agent health

---

## Phase 5 – Personal Life Agents (8 weeks) 🆕

**Goal: RedNode handles your complete digital life — not just infrastructure.**

### Phase 5a – Communications Agent (2 weeks)

- [ ] 11 tools: `email.fetch`, `email.send`, `email.summarize`, `email.draft`, `email.rules`, `calendar.view`, `calendar.create`, `calendar.conflicts`, `contacts.search`, `chat.bridge`, `notifications.digest`
- [ ] IMAP/JMAP integration for email
- [ ] CalDAV integration for calendar
- [ ] Matrix/Signal bridge for unified messaging
- [ ] Daily morning brief: unread emails + today's calendar + priority tasks

### Phase 5b – Productivity Agent (2 weeks)

- [ ] 11 tools: `notes.create`, `notes.search`, `tasks.create`, `tasks.list`, `tasks.complete`, `docs.generate`, `docs.export`, `clipboard.history`, `focus.mode`, `bookmarks.save`, `bookmarks.search`
- [ ] Local Markdown storage with RAG indexing
- [ ] Pandoc for document export (PDF, DOCX, HTML)
- [ ] Semantic bookmark search

### Phase 5c – Browser Agent (1 week)

- [ ] 7 tools: `browser.open`, `browser.scrape`, `browser.screenshot`, `browser.fill`, `browser.search`, `browser.read`, `browser.automate`
- [ ] SearXNG self-hosted meta-search (no tracking)
- [ ] Playwright for browser automation (sandboxed)

### Phase 5d – Social Media Agent (1 week)

- [ ] 8 tools: `social.post`, `social.schedule`, `social.draft`, `social.analytics`, `social.reply`, `social.monitor`, `social.feed`, `social.dm`
- [ ] Twitter/X API v2, LinkedIn API, Mastodon API, Bluesky AT Protocol

### Phase 5e – Finance, Media, Life Agents (2 weeks)

- [ ] **Finance Agent** — 8 tools: accounts, transactions, budget, alerts, reports, crypto, invoices
- [ ] **Media Agent** — 7 tools: play, library, download, stream, recommend, photos organize/search
- [ ] **Life Management Agent** — 8 tools: health log, trends, habits, recipes, travel, weather, journal, reflect

---

## Phase 6 – Operating Layer (3 weeks)

**Goal: Deep OS integration. Portable identity. Multi-machine.**

- [ ] Host adapters: Linux (NixOS primary), macOS, Windows (WSL2)
- [ ] Portable state export/import: `rednode export` → age-encrypted + ed25519 signed `.rednode` bundle
- [ ] `rednode import` → resume on new hardware < 60s
- [ ] Multi-machine federation — distributed agent society across nodes
- [ ] PID1 mode — `rednode-core` as init system (replaces systemd)
  - `os/nixos/configuration-os.nix` for TRUE OS MODE
- [ ] Package management via agents — RedNode manages software lifecycle

---

## Phase 7 – RedNode-OS 1.0 + Commercial Launch (4 weeks) 🆕

**Goal: Production-ready release. Commercial product. Revenue.**

### Product

- [ ] NixOS ISO with one-click installer + hardware detection
- [ ] Pre-signed Flutter APK/IPA on app stores
- [ ] Tauri desktop auto-updater
- [ ] SBOM + Cosign for all artifacts
- [ ] Security hardening audit (fuzzing, pen-test)
- [ ] Documentation site (Docusaurus/Starlight)

### Commercial

- [ ] **RedNode Pro** — $9.99/month or $99/year
  - Premium dashboard, all 14 agents pre-built, auto-updates, priority support
- [ ] **RedNode Hardware** — $399–$799 pre-built mini-PC with RedNode-OS
- [ ] **Enterprise Edition** — $49/user/month, multi-user RBAC, compliance reporting
- [ ] **Agent Marketplace** — third-party agents, 30% commission
- [ ] **Consulting** — custom agents, security audits, $150–$300/hour

### Go-To-Market

- [ ] Launch on Hacker News, Reddit (r/selfhosted, r/privacy, r/linux, r/nix), Product Hunt
- [ ] YouTube series: "I Replaced My OS With an AI"
- [ ] Discord community
- [ ] "Build in public" content (Twitter/X, blog)

---

## Exit Criteria

### MVP (Phase 1) ✅

- [x] Intent → Plan → Execute → Audit loop working
- [x] Local LLM (Ollama) running
- [x] Full audit trail, hash-chained
- [x] 6 agents online, 23 tools registered

### v0.5 (End of Phase 3)

- [ ] LLM-powered planner (not keyword matching)
- [ ] Voice loop < 1.2s end-to-end
- [ ] Zero executor escapes in 10k fuzz test
- [ ] RAG P@3 > 0.82
- [ ] Self-healing: auto-recover from agent crash within 30s

### v0.8 (End of Phase 5)

- [ ] 14 agents, 90+ tools operational
- [ ] Pi-hole + TrueNAS + Frigate fully integrated
- [ ] Cross-system workflows functional (goodnight, leaving, focus)
- [ ] Daily email brief + calendar integration working
- [ ] Social media posting + scheduling working

### v1.0 (End of Phase 7)

- [ ] < 45s boot to fully operational
- [ ] Voice < 1.2s response
- [ ] 0 executor escapes / 10k fuzz
- [ ] RAG P@3 > 0.82
- [ ] 72h crash-free continuous operation
- [ ] Audit coverage: 100%
- [ ] Portable export/import working
- [ ] Commercial product launched
- [ ] 100+ GitHub stars, active community

---

## Timeline Summary

| Phase | Duration | Agents | Tools | Key Milestone |
|---|---|---|---|---|
| **1 – Foundation** ✅ | 4 weeks | 6 | 23 | Core pipeline working |
| **2 – Intelligence** | 4 weeks | 6 | 23 | LLM planner + voice + agent collab |
| **3 – Security** | 4 weeks | 6 | 23 | Production SOC + self-healing |
| **4 – Home Infra** 🆕 | 6 weeks | 9 | 58 | Pi-hole + TrueNAS + Frigate + workflows |
| **5 – Personal Life** 🆕 | 8 weeks | 14 | 90 | Email, social, productivity, finance, media |
| **6 – Operating Layer** | 3 weeks | 14 | 90 | Portable identity + multi-machine + PID1 |
| **7 – Launch** 🆕 | 4 weeks | 14 | 90+ | Commercial product + marketplace |
| **TOTAL** | **~33 weeks** | **14** | **90+** | |

---

## Hardware Reference

### pfSense Firewall (Dedicated Mini-PC)

| Spec | Minimum | Recommended | Notes |
|---|---|---|---|
| **CPU** | Any dual-core x86_64 | Intel N100 / Celeron J6412 | Even a 10yr old i3 handles 1Gbps fine |
| **RAM** | 2 GB | 4 GB (8 GB with Suricata IDS) | Not required but nice for 24/7 |
| **Storage** | 16 GB SSD | 32 GB SSD | SSD mandatory — HDD risks corruption |
| **NICs** | 2× Ethernet (Intel preferred) | 2× Intel i225/i226-V 2.5GbE | Avoid Realtek — driver issues on FreeBSD |
| **Power** | ~6–12W idle | Fanless preferred | Silent, always-on |
| **Cost** | ~$30–50 used | ~$120–150 new | Dell Wyse 5070, HP Mini, Lenovo M720q |

### RedNode-OS Server (Your Old PC + GPU)

| Spec | Minimum (works) | Recommended (smooth) | Ideal (future-proof) |
|---|---|---|---|
| **CPU** | 4-core x86_64 (i5 4th gen / Ryzen 3) | 6-core (i5 10th gen+ / Ryzen 5 3600) | 8+ cores (i7 / Ryzen 7) |
| **RAM** | 16 GB | 32 GB | 64 GB |
| **Boot SSD** | 120 GB SATA SSD | 500 GB NVMe | 1 TB NVMe |
| **GPU** | 6 GB VRAM (GTX 1060) | 8–12 GB VRAM (RTX 3060 12GB) | 16+ GB VRAM (RTX 4060 Ti 16GB) |
| **Network** | 1 Gbps NIC | 2.5 Gbps | 2.5 Gbps or 10 Gbps |

### GPU VRAM Budget (How It Gets Shared)

| Service | VRAM Used | Notes |
|---|---|---|
| **Ollama — Qwen2.5-7B Q4_K_M** | ~4.4 GB | Good enough for most intents |
| **Ollama — Qwen2.5-14B Q4_K_M** | ~8.7 GB | Better reasoning, needs 10GB+ GPU |
| **Ollama — nomic-embed-text** | ~0.3 GB | Embedding model for RAG |
| **Frigate — TensorRT** | ~0.5–1.0 GB | AI object detection for cameras |
| **TOTAL (7B + Frigate)** | **~5.2 GB** | Fits on 6 GB GPU |
| **TOTAL (14B + Frigate)** | **~9.5 GB** | Needs 12 GB GPU |

### What Model Size for Your GPU

| Your GPU VRAM | LLM Model | Frigate | Performance |
|---|---|---|---|
| **6 GB** (GTX 1060) | Qwen2.5-7B Q4 | ✅ Yes | ~45 tok/s LLM, adequate |
| **8 GB** (RTX 3060 8GB / RTX 4060) | Qwen2.5-7B Q4 | ✅ Yes | ~45 tok/s LLM, comfortable headroom |
| **12 GB** (RTX 3060 12GB) | Qwen2.5-14B Q4 | ✅ Yes | ~20 tok/s LLM, best value |
| **16 GB** (RTX 4060 Ti 16GB) | Qwen2.5-14B Q4 | ✅ Yes | ~20 tok/s LLM, room for larger models |
| **24 GB** (RTX 4090) | Qwen2.5-32B Q4 | ✅ Yes | ~28 tok/s LLM, overkill for home |

---

## Revenue Projection

| Year | Streams | Revenue (Conservative) |
|---|---|---|
| Year 1 | Pro subs (200) + consulting | $30K–$50K |
| Year 2 | Pro (1000) + hardware (300) + enterprise (5 teams) | $200K–$400K |
| Year 3 | Pro (3000) + hardware (1000) + enterprise (20) + marketplace | $800K–$1.5M |

---

## Market Context (June 2026)

- AI OS market: $12.85B (2025) → $107.6B (2033), 30.5% CAGR
- Privacy-preserving AI: growing rapidly → $39.93B (2035)
- Apple Intelligence expanding on-device AI — validates the thesis
- Google Personal Intelligence in Search/Gemini/Chrome — market wants contextual AI
- On-device AI agents (Hermes Desktop, Aion 1.0) emerging — but none are a full OS
- RedNode is the **only** project combining: full OS + multi-agent + security-first + self-aware + 100% local + open-source + portable identity

---

## Competitive Moat

| vs Competitor | RedNode's Advantage |
|---|---|
| Apple Intelligence | Open-source, fully local, not walled-garden |
| Google Gemini | Zero telemetry, zero tracking — Google IS surveillance |
| Microsoft Copilot+ | Offline-first, no cloud dependency |
| Umbrel | Intelligent OS, not just an app launcher |
| Hermes Desktop | Multi-agent society, not single agent. Full OS. Security-first. |

---

*RedNode-OS — The computer becomes the intelligence.*  
*Privacy-first. Self-aware. Autonomous. Yours.*
