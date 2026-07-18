# RedNode-OS Roadmap

> *The computer does not contain intelligence. The computer becomes the intelligence.*
>
> **Current State**: v0.9.3 — 18 agents, 359 tools, 21 Rust modules, 24,980 LOC, Sentience Engine, Self-Evolution, LLM planner, GOAP fallback, RAG memory, knowledge graph, 15-tab dashboard, Setup Wizard, Settings page, Flutter mobile, Tauri desktop, CLI, voice (Whisper+Piper+OpenWakeWord), Signal bot, threat intel, NVD sync, IDS integration, predictive maintenance, smart notifications, cross-agent pipelines, PII detection, circuit breaker, self-healing, live ISO, branded kiosk, web-based config management  
> **Target**: v1.0 — Production deployment on real hardware, commercial product

---

## Phase 1 – Foundation ✅ Complete

- [x] CNS Rust core (Axum + Tokio) — intent routing, planning, coordination
- [x] NATS JetStream bus — agent communication backbone
- [x] PostgreSQL 16 + Qdrant + Kuzu memory layer
- [x] 6-agent framework (System, Security, Coding, Research, Automation, Network)
- [x] 23 risk-tagged tools in Tool Registry
- [x] Security Engine — risk assessment, approval gates, 25+ deny patterns
- [x] Sandboxed executor — firejail/bubblewrap/unshare + seccomp
- [x] SHA-256 hash-chained audit log (tamper-evident)
- [x] RAG pipeline — Qdrant vector search + Ollama embeddings + 3-tier fallback
- [x] Sentience Engine — self-model, 5 homeostatic drives, goal generation, memory consolidation
- [x] Next.js 15 dashboard — 13-tab SOC console
- [x] Flutter 3.22 mobile app — FCM push, biometric approval
- [x] Tauri 2 desktop app
- [x] TypeScript CLI (19 commands)
- [x] NixOS bare-metal configuration — flake.nix, ISO build, hardened kernel
- [x] Docker Compose deployment (11 services)
- [x] Security Agent — CVE auto-checker, auto-patcher with snapshot rollback, Falco eBPF bridge

---

## Phase 2 – Intelligence Layer ✅ Complete

- [x] **LLM-Powered Planner** — Qwen2.5 via Ollama, structured JSON PlanStep[] output
  - [x] Dynamic tool context loaded from tools.json at runtime (not hardcoded)
  - [x] GOAP A* search as fallback when LLM is unavailable
  - [x] Keyword matching as final fallback
- [x] **Agent Collaboration** — NATS pub/sub, coordinator dispatches to agents
  - [x] Parallel step execution for independent tasks
  - [x] State caching within sessions
  - [x] Circuit breaker (20 steps, 120s timeout, recursion depth 5)
- [x] **Voice Loop** — Whisper STT + Piper TTS + OpenWakeWord
  - [x] Wake word detection ("hey rednode" or custom)
  - [x] Complete voice_loop.py, stt_server.py, tts_server.py
  - [x] Systemd services for auto-start on NixOS
  - [x] CLI toggle: `rednode voice on/off/status`
- [x] **Research Engine** — SearXNG web search, deep multi-source research
  - [x] Document OCR and PDF ingestion
  - [x] Knowledge graph entity extraction (Kuzu)
  - [x] arXiv paper search, Wikipedia, URL summarization
  - [x] RSS feed aggregation, podcast download/transcription
  - [x] Fact-checking, timeline building, topic comparison
- [x] **Automation Workflows** — DAG engine with built-in workflows
  - [x] Goodnight, morning, focus, leaving workflows
  - [x] Cron-like scheduling + event-triggered workflows
  - [x] 5 cross-agent pipelines (threat, morning, maintenance, presence, IDS)
- [x] **Conversation Memory** — multi-turn context within sessions
  - [x] Per-session history (last 10 turns)
  - [x] Reference resolution ("do that again", "what about cameras?")
  - [x] Predictive intent — learns daily patterns after 7-15 days

---

## Phase 3 – Security Layer ✅ Complete

- [x] **Threat Intel Pipeline** — NVD CVE sync + YARA rules + abuse.ch/OTX/ET feeds
  - [x] Offline CVE DB with 6-hour sync interval
  - [x] IOC checking against threat intel database
  - [x] Auto-block malicious IPs on pfSense, domains on Pi-hole
  - [x] Dark web OSINT search via Tor/SearXNG
- [x] **eBPF Integration** — Falco eBPF bridge
  - [x] Log tailing + journalctl fallback
  - [x] SSH brute force detection, kernel panic monitoring
- [x] **Self-Healing Engine** — detect → diagnose → fix → verify → log
  - [x] `rednode-selfheal.sh` — 1,174 lines, 12 subsystem checks
  - [x] 5 retries with exponential backoff
  - [x] Pattern-matched error repair (disk full, port conflict, permissions)
  - [x] 3-strategy source deployment (ISO baked → system closure → git clone)
  - [x] Systemd watchdog with 5-minute health checks
- [x] **Smart Security** — graduated autonomous response
  - [x] Low → Medium → High → Critical risk levels
  - [x] 355 tools risk-tagged (235 low, 97 medium, 23 high)
  - [x] High-risk operations require human approval via Signal/dashboard
- [x] **Compliance** — security posture assessment
  - [x] `sec.compliance_check` — lynis CIS benchmarks
  - [x] `sec.audit_verify` — SHA-256 hash chain integrity
  - [x] Suricata IDS alert processing
  - [x] SSL certificate monitoring
  - [x] fail2ban status/ban/unban
  - [x] Rootkit scanning, password auditing, port scanning
- [x] **PII Detection** — 14 types (credit cards, SSN, email, phone, API keys, Aadhaar, etc.)
  - [x] Auto-redact before memory storage

---

## Phase 4 – Home Infrastructure ✅ Complete

### Phase 4a – Infrastructure Agent + Pi-hole ✅
- [x] 22 tools: stats, top blocked/clients, query log, disable/enable, add/remove block, regex filters, whitelist, CNAME, gravity update, groups, DNS history, anomaly detection, Docker management
- [x] Pi-hole v6 REST API integration
- [x] DNS anomaly detection → Security Agent correlation

### Phase 4b – Storage Agent + TrueNAS ✅
- [x] 24 tools: health, pools, datasets, usage, disks, SMART, snapshots CRUD, shares CRUD, alerts, replication, scrub, quotas, compression stats, rsync jobs, cloud sync, permissions, file search, dedup reports, I/O stats, temperature history, backup
- [x] TrueNAS REST API v2.0 integration
- [x] Predictive maintenance — linear regression on SMART data, predicts failures 30 days ahead

### Phase 4c – Surveillance Agent + Frigate NVR ✅
- [x] 26 tools: camera status/events/snapshots/clips/search/zones/config, person/vehicle/audio detection, recording list/export, timelapse, face register/identify, PTZ control, health check, presence detection (camera + network), anomaly detection, AI review summaries
- [x] Frigate REST API + MQTT event bridge
- [x] Presence detection combining camera + ARP network data

### Phase 4d – Home Agent + Home Assistant ✅
- [x] 19 tools: lights, switches, climate, scenes, automations, device info, history, energy, battery status, door locks, garage doors, vacuum, irrigation, alarm, media player, notifications, logbook
- [x] Home Assistant REST API integration

### Phase 4e – Cross-System Workflows ✅
- [x] Goodnight workflow — Pi-hole strict + cameras armed + TrueNAS snapshot + memory consolidation
- [x] Morning briefing — 12-step pipeline (weather, news, cameras, email, DNS, storage, calendar, tasks)
- [x] Focus mode — block social media DNS
- [x] Leaving workflow — cameras active + alerts
- [x] Threat response pipeline — IOC detect → pfSense block → Pi-hole block → scan → Signal alert
- [x] Predictive maintenance pipeline — nightly SMART collection → trend analysis → failure prediction
- [x] IDS response pipeline — Suricata alert → triage → block/isolate → notify

---

## Phase 5 – Personal Life Agents ✅ Complete

### Phase 5a – Communications Agent ✅
- [x] 18 tools: email fetch/send/summarize/draft/triage/search/archive/unsubscribe, calendar view/create/conflicts/reschedule/availability, contacts search/add/birthday reminders
- [x] IMAP/SMTP integration
- [x] CalDAV integration
- [x] LLM-powered email triage and auto-drafting

### Phase 5b – Productivity Agent ✅
- [x] 23 tools: notes CRUD/tag/export/link, tasks CRUD/priority/due/recurring/project, habits track/streak, pomodoro timer, journal entry/search, bookmarks save/search
- [x] Local Markdown storage with RAG indexing
- [x] Semantic bookmark search

### Phase 5c – Browser Agent ✅
- [x] 13 tools: read, scrape, screenshot, download, search, links, fill, PDF, monitor, cookie clean, price track, archive, readability
- [x] Playwright stealth mode (15+ user agents, header randomization)
- [x] SearXNG self-hosted meta-search

### Phase 5d – Social Media Agent ✅
- [x] 16 tools: post, draft, schedule, feed, analytics, reply, DM, platforms, thread, hashtags, best time, followers, mentions, block, crosspost, monitor
- [x] Twitter/X, Mastodon, Bluesky, LinkedIn API integration
- [x] LLM-powered content drafting and hashtag suggestions

### Phase 5e – Media Agent ✅
- [x] 21 tools: Jellyfin library/search/sessions/playback, photo ingest/search/tag/stats/faces/duplicate/resize/export, music scan/playlist/lyrics, video info/thumbnail/convert
- [x] Jellyfin REST API integration
- [x] Photo management with file scanning and duplicate detection
- [x] Video processing via ffmpeg/ffprobe

### Phase 5f – Finance + Life Agents ❌ Deferred
- [ ] Finance Agent (budgets, transactions, crypto, invoices)
- [ ] Life Management Agent (health, recipes, travel, habits)
- *Deferred to post-v1.0 — deploy first, add when needed*

---

## Phase 6 – Operating Layer 🟡 Mostly Complete

- [x] **NixOS Bare-Metal** — 10 NixOS modules, declarative configuration
  - [x] configuration.nix, flake.nix, hardware.nix, disk-encryption.nix
  - [x] minimal.nix (stripped NixOS, ~800 MB), kiosk.nix (branded GUI)
  - [x] live.nix (DHCP, auto-login, for testing on any machine)
  - [x] extras.nix (WireGuard VPN, UPS/NUT, Suricata IDS)
  - [x] rednode-deploy.nix (self-healing deployment + CLI)
- [x] **Portable State** — `rednode export` / `rednode import`
  - [x] age-encrypted backup of PostgreSQL + Qdrant + config
  - [x] Resume on new hardware < 60 seconds
- [x] **PID1 Init Mode** — `init.rs` with signal handling, service supervision, watchdog
- [x] **ISO Builder** — 3 variants (standard, live, kiosk)
  - [x] Source code baked into ISO (no git clone needed)
  - [x] Compiled CNS binary baked in
  - [x] zstd level 19 compression (~1.2 GB ISO)
- [x] **Branded Boot** — Plymouth splash with RedNode logo, TTY login banner, MOTD
- [x] **GUI Toggle** — `rednode gui on/off/status` (Cage + Chromium kiosk, ~200-350 MB RAM)
- [x] **Voice Toggle** — `rednode voice on/off/status` (STT + TTS + wake word as systemd services)
- [x] **DHCP + NetworkManager** — works on any network (removed hardcoded static IP)
- [ ] **Multi-machine federation** — distributed agent society across nodes
- [ ] **macOS/Windows host adapters** — currently NixOS/Linux only

---

## Phase 7 – Self-Evolution + Intelligence ✅ NEW (added since original roadmap)

- [x] **Self-Evolution Engine** — `evolution.rs` (414 lines)
  - [x] Discovers CLI tools on the system automatically
  - [x] Reads documentation (man pages, --help)
  - [x] Generates TypeScript handler code
  - [x] Registers tools in tools.json at runtime
  - [x] Injects handlers into agent index.ts
  - [x] Dynamic planner loads tools from tools.json (not hardcoded)
  - [x] LLM can plan with newly evolved tools immediately
- [x] **Learning Agent** — 17 tools for autonomous knowledge acquisition
  - [x] Tool/API discovery, documentation ingestion
  - [x] Audit log pattern mining, workflow suggestions
  - [x] Self-assessment, LLM benchmarking
  - [x] learn.evolve, learn.auto_evolve, learn.teach, learn.list_evolved
- [x] **Smart Notifications** — `notifications.rs` (249 lines)
  - [x] 4 urgency levels (critical/high/normal/low)
  - [x] Quiet hours (configurable, default 10 PM–7 AM)
  - [x] Batched digests for low-priority events
- [x] **Predictive Maintenance** — `predict.rs` (271 lines)
  - [x] Linear regression on SMART attributes, CPU temps
  - [x] Predicts failures 30 days ahead
  - [x] 5 status levels: healthy → degrading → warning → critical → failure imminent
- [x] **Cross-Agent Pipelines** — `pipelines.rs` (430 lines)
  - [x] 5 built-in pipelines with variable passing, conditions, retry logic
  - [x] 300-second circuit breaker
- [x] **Web-Based Configuration** — no .env editing required
  - [x] `config.rs` — config manager (config.json, secrets, service testing)
  - [x] Setup Wizard page (`/setup`) — 5-step first-boot UI
  - [x] Settings page (`/settings`) — service list + config panels + test buttons
  - [x] Config loader in agents — fetches from CNS, populates process.env
  - [x] NATS broadcast on config change — agents re-fetch automatically
- [x] **Predictive Intent Timing** — only activates after 7+ days uptime OR 200+ audit log entries

---

## Phase 8 – v1.0 Release 🔴 Not Started

**Goal: Production-ready release. Deploy on real hardware. Commercial product.**

### Deployment
- [ ] Build and test ISO on real hardware
- [ ] Connect all homelab services via setup wizard
- [ ] Run for 30+ days continuously, fix what breaks
- [ ] Voice latency tuning on real hardware (target < 1.2s)
- [ ] Security hardening audit (fuzzing, pen-testing)

### Product Polish
- [ ] 287 additional tools (documented in TOOLS-EXPANSION-AND-BACKPROPAGATION.md)
- [ ] Pre-signed Flutter APK/IPA on app stores
- [ ] Tauri desktop auto-updater
- [ ] Documentation site (Docusaurus/Starlight)
- [ ] Custom LLM fine-tuning on user's own data (guide exists, needs testing)

### Commercial
- [ ] **RedNode Pro** — $9.99/month or $99/year
  - Premium dashboard, auto-updates, priority support
- [ ] **RedNode Hardware** — $399–$799 pre-built mini-PC with RedNode-OS
- [ ] **Enterprise Edition** — $49/user/month, multi-user RBAC, compliance reporting
- [ ] **Agent Marketplace** — third-party agents, 30% commission
- [ ] **Consulting** — custom agents, security audits, $150–$300/hour

### Go-To-Market
- [ ] Launch on Hacker News, Reddit (r/selfhosted, r/privacy, r/linux, r/nix), Product Hunt
- [ ] YouTube series: "I Replaced My OS With an AI"
- [ ] Discord community
- [ ] "Build in public" content

---

## Exit Criteria

### v0.5 ✅ (Achieved)
- [x] LLM-powered planner (not keyword matching)
- [x] Voice loop functional
- [x] RAG memory operational
- [x] Self-healing: auto-recover from agent crash

### v0.9 ✅ (Current)
- [x] 18 agents, 359 tools operational
- [x] Pi-hole + TrueNAS + Frigate + Home Assistant fully integrated
- [x] Cross-system workflows functional
- [x] Email + calendar + contacts integration
- [x] Social media posting + scheduling
- [x] Self-evolution engine
- [x] Web-based configuration (no .env editing)
- [x] Live ISO for testing on any machine
- [x] 3 ISO variants (standard, live, kiosk)

### v1.0 (Target)
- [ ] Deployed on real hardware for 30+ days
- [ ] Voice < 1.2s end-to-end response
- [ ] 0 executor escapes in 10k fuzz test
- [ ] 72h crash-free continuous operation
- [ ] All homelab services connected and tested
- [ ] Predictive intent active and useful
- [ ] Commercial product launched
- [ ] 100+ GitHub stars, active community

---

## Timeline Summary

| Phase | Status | Agents | Tools | Key Milestone |
|---|---|---|---|---|
| **1 – Foundation** | ✅ Done | 6 | 23 | Core pipeline working |
| **2 – Intelligence** | ✅ Done | 6 | 23 | LLM planner + voice + memory |
| **3 – Security** | ✅ Done | 7 | 40+ | SOC + self-healing + compliance |
| **4 – Home Infra** | ✅ Done | 12 | 130+ | Pi-hole + TrueNAS + Frigate + HA |
| **5 – Personal Life** | ✅ Done | 17 | 280+ | Email, social, productivity, media |
| **6 – Operating Layer** | 🟡 90% | 18 | 359 | NixOS, ISO, portable, kiosk |
| **7 – Self-Evolution** | ✅ Done | 18 | 359 | Auto-learn, evolve, config UI |
| **8 – v1.0 Launch** | 🔴 Next | 18 | 359+ | Real hardware, commercial |

---

## What's NOT Built (and Why)

| Feature | Reason | When |
|---|---|---|
| Finance Agent | Deploy first, add when needed | Post v1.0 |
| Life Management Agent | Deploy first, add when needed | Post v1.0 |
| Multi-machine federation | Single-machine covers 95% of use cases | Post v1.0 |
| macOS/Windows host adapters | NixOS is the target OS | Post v1.0 |
| Emotional intelligence | Rejected — deploy first, iterate on real problems | Maybe never |
| 287 additional tools | Documented and ready, add incrementally | Ongoing |
| Obsidian vault sync | Documented, needs implementation | Post v1.0 |
| Continuous online LLM learning | Risky (bad feedback degrades model) | Research |

---

## Hardware Reference

### RedNode-OS Server

| Spec | Minimum | Recommended | Ideal |
|---|---|---|---|
| **CPU** | 4-core x86_64 | 6-core (i5 10th gen+ / Ryzen 5) | 8+ cores |
| **RAM** | 16 GB | 32 GB | 64 GB |
| **SSD** | 120 GB | 500 GB NVMe | 1 TB NVMe |
| **GPU** | None (CPU-only mode) | 12 GB VRAM (RTX 3060) | 16+ GB VRAM |
| **Network** | 1 Gbps Ethernet | 2.5 Gbps | — |

### GPU VRAM Budget

| Configuration | LLM | Whisper | Frigate | Total | GPU |
|---|---|---|---|---|---|
| **CPU-only** | Qwen 3B (CPU) | small (CPU) | — | 0 GB | None |
| **Starter** | Qwen 7B (4.4 GB) | small (1 GB) | 0.8 GB | ~6.5 GB | RTX 3060 8GB |
| **Recommended** | Qwen 14B (8.7 GB) | small (1 GB) | 0.8 GB | ~10.8 GB | RTX 3060 12GB ⭐ |
| **Full** | Qwen 14B (8.7 GB) | large-v3 (3 GB) | 0.8 GB | ~12.8 GB | RTX 4060 Ti 16GB |

---

## Revenue Projection

| Year | Streams | Revenue (Conservative) |
|---|---|---|
| Year 1 | Pro subs (200) + consulting | $30K–$50K |
| Year 2 | Pro (1000) + hardware (300) + enterprise (5 teams) | $200K–$400K |
| Year 3 | Pro (3000) + hardware (1000) + enterprise (20) + marketplace | $800K–$1.5M |

---

## Competitive Position (July 2026)

| vs Competitor | RedNode's Advantage |
|---|---|
| Apple Intelligence | Open-source, fully local, not walled-garden, self-evolving |
| Google Gemini | Zero telemetry, zero tracking, 359 tools vs generic chat |
| Microsoft Copilot+ | Offline-first, no cloud, self-healing, homelab integration |
| Umbrel/CasaOS | Intelligent OS with 18 AI agents, not just app launcher |
| Open WebUI | Full OS with security, cameras, networking — not just LLM chat |
| Home Assistant | AI-powered intent planning + 17 other agents beyond smart home |

---

*RedNode-OS v0.9.3 — 359 tools, 18 agents, 21 Rust modules, 24,980 lines*  
*The computer becomes the intelligence. Privacy-first. Self-aware. Self-evolving. Yours.*
