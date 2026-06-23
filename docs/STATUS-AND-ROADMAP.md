# RedNode-OS — Where We Stand & What's Possible

> Last updated: v0.7.1

---

## Current System Score: 9.0 / 10

| Category | Score | Status |
|---|---|---|
| **Core Architecture** | 10/10 | Rust CNS, NATS bus, 16 agents, LLM planner, GOAP fallback |
| **Security** | 9/10 | PII detection, sandboxing, audit chain, risk tags, CVE scanning |
| **Memory** | 9/10 | PostgreSQL + Qdrant + knowledge graph, propositions, consolidation |
| **Intelligence** | 9/10 | Sentience Engine, predictive intent, pattern promotion, multi-turn |
| **Infrastructure** | 9/10 | NixOS config, Docker, self-healing, hardware detection |
| **Interfaces** | 9/10 | Web, CLI, Voice, Signal, Mobile, Desktop, API, WebSocket, Kiosk |
| **Agents** | 8/10 | 16 agents, 122 tools — all have logic, need real-hardware testing |
| **Deployment** | 9/10 | Self-healing installer, baked ISO, branded kiosk, Plymouth splash |
| **Docs** | 9/10 | 25 markdown files, 46-question FAQ, step-by-step guides |
| **Real-world testing** | 0/10 | Not yet deployed on actual hardware |

**The remaining 1.0 comes from real-world deployment experience.**

---

## What's Built (Complete Inventory)

### Core (Rust — 5,179 lines, 17 modules)

| Module | Lines | Purpose |
|---|---|---|
| `sentience.rs` | 968 | 5 homeostatic drives, autonomous goals, self-improvement loop |
| `memory.rs` | 1,005 | Propositions, consolidation (JUDGE+CONSOLIDATE), pattern promotion, knowledge graph |
| `init.rs` | 445 | PID 1 mode, signal handling, service supervision, watchdog |
| `executor.rs` | 380 | Sandboxed execution (firejail/bubblewrap), seccomp, timeout enforcement |
| `api.rs` | 374 | Axum HTTP + WebSocket API, 15+ endpoints |
| `planner.rs` | 337 | LLM-based plan generation (Ollama Qwen2.5), structured JSON output |
| `goap.rs` | 248 | A* search planning with preconditions/effects (LLM fallback) |
| `security.rs` | 228 | Risk tagging, approval gates, tool validation |
| `pii.rs` | 227 | 14 PII types, redact/block/log actions |
| `coordinator.rs` | 203 | Parallel step execution, state caching, circuit breaker |
| `intent_router.rs` | 195 | Keyword matching, session memory, reference resolution |
| `memory_optimizer.rs` | 195 | 4 pressure levels, model context management, cache management |
| `events.rs` | 137 | Event types, bus integration, streaming |
| `auth.rs` | 107 | Token-based auth, request validation |
| `bus.rs` | 66 | NATS JetStream connection, pub/sub |
| `main.rs` | 49 | Entry point, runtime selection |
| `lib.rs` | 15 | Module exports |

### Agents (TypeScript — 18 directories, 122 tools)

| Agent | Tools | What It Does |
|---|---|---|
| **system-agent** | 6 | Process management, Docker, filesystem, service control |
| **security-agent** | 9 | CVE scanning (NVD), threat intel (abuse.ch/OTX), dark web OSINT, YARA rules, SSH hardening, auto-patching with btrfs rollback |
| **coding-agent** | 7 | Code analysis, generation, refactoring, testing, git operations |
| **research-agent** | 12 | Web search (SearXNG), knowledge base, PDF/OCR ingestion, weather, news, knowledge graph CRUD |
| **automation-agent** | 4 | Cron workflows, scheduled tasks, event triggers |
| **network-agent** | 10 | pfSense integration, firewall rules, VLAN management, device scanning, VPN, traffic analysis |
| **infra-agent** | 9 | Pi-hole DNS management, block lists, query analysis, anomaly detection |
| **storage-agent** | 14 | TrueNAS API, pool health, SMART monitoring, snapshots, replication, share management |
| **surveillance-agent** | 11 | Frigate NVR, camera events, person detection, anomaly detection, clip retrieval |
| **comms-agent** | 10 | Email (IMAP/SMTP), calendar (CalDAV), contacts, notification digest |
| **productivity-agent** | 10 | Notes, tasks, bookmarks — local productivity suite |
| **media-agent** | 7 | Media library management, playback control |
| **home-agent** | 7 | Home Assistant integration via MQTT, lights, climate, scenes, automations |
| **browser-agent** | 7 | Stealth browsing (15+ user agents, header randomization), scraping, screenshots |
| **social-agent** | 9 | Twitter/Mastodon/Bluesky/LinkedIn posting, analytics, monitoring, DMs |
| **signal-bot** | — | Signal messenger integration, E2EE, owner-only commands |
| **endpoint-agent** | — | Cross-platform (Linux/Windows/macOS) remote monitoring |

### Infrastructure

| Component | Config | Purpose |
|---|---|---|
| **NixOS** | 8 .nix modules (1,433 lines) | Operating system, all services declarative |
| **PostgreSQL 16** | + pgvector | Structured memory, audit log, knowledge graph |
| **NATS JetStream** | NixOS native | Agent message bus |
| **Qdrant v1.9** | Docker/OCI | Vector memory for semantic search |
| **Ollama** | NixOS native | Local LLM (GPU-accelerated) |
| **Mosquitto** | NixOS native | MQTT for Frigate + Home Assistant |
| **Grafana** | NixOS native | Dashboard for observability |
| **Prometheus** | NixOS native | Metrics collection |
| **Loki** | NixOS native | Log aggregation |
| **SearXNG** | Docker | Private web search |
| **Docker** | For Qdrant + Frigate | Container runtime |

### Interfaces (9 total)

| Interface | Technology | Status |
|---|---|---|
| Web Dashboard | Next.js 15 + React 19 | ✅ Ready |
| CLI | 19 commands | ✅ Ready |
| Voice | Whisper STT + Piper TTS + wake word | ✅ Ready |
| Signal Bot | signal-cli, E2EE | ✅ Ready |
| Mobile | Flutter (APK) | ✅ Ready |
| Desktop | Tauri (Windows/macOS/Linux) | ✅ Ready |
| REST API | Axum (15+ endpoints) | ✅ Ready |
| WebSocket | Real-time event stream | ✅ Ready |
| Kiosk | Cage + Chromium + Plymouth | ✅ Ready |

### Self-Healing & Deployment

| Component | Lines | Purpose |
|---|---|---|
| `rednode-selfheal.sh` | 1,174 | Install, diagnose, repair, watchdog — 12 subsystem checks |
| `rednode-deploy.nix` | 243 | systemd services for auto-deploy + continuous monitoring |
| `minimal.nix` | 132 | Stripped NixOS (remove perl/rsync/strace/GUI/docs/RAID/printing) |
| `kiosk.nix` | 239 | Branded boot splash + Cage kiosk + auto-login |
| `setup-first-boot.sh` | 269 | Interactive first-boot for non-NixOS installs |
| `start-all.sh` | 192 | Start/stop/status for all services |
| `rednode-hardware-detect.sh` | 197 | GPU/VRAM/RAM detection, model auto-selection |
| `rednode-export.sh` | 129 | age-encrypted backup of entire RedNode state |
| `rednode-import.sh` | 117 | Restore from backup — resume anywhere in <60s |

---

## What You Can Add — The Complete Possibility Map

### 🏠 Tier 1: Home Infrastructure (immediate value, your existing hardware)

| Feature | Difficulty | What It Does | Agent |
|---|---|---|---|
| **pfSense auto-threat-blocking** | Easy | Security Agent detects threat → Network Agent auto-adds pfSense firewall rule → blocks IP across all VLANs | security + network |
| **Pi-hole analytics dashboard** | Easy | "Show me DNS anomalies from the last 24 hours" — infra-agent already has the tools | infra |
| **TrueNAS snapshot scheduler** | Easy | Auto-daily snapshots of critical datasets, auto-prune old ones, alert on SMART failures | storage |
| **Camera person recognition** | Medium | Frigate + face detection → "who was at the front door at 3 PM?" (local-only, no cloud) | surveillance |
| **UPS monitoring** | Easy | NUT (Network UPS Tools) → alert on power events, auto-graceful-shutdown on low battery | New: power-agent or system-agent extension |
| **Printer/scanner management** | Easy | CUPS integration, "print this PDF", "scan document and OCR" | New: office-agent or system-agent extension |
| **Bandwidth monitoring** | Easy | NetFlow/sFlow from pfSense → "which device used the most bandwidth today?" | network |
| **Wake-on-LAN** | Easy | "Wake up my desktop" → sends WOL magic packet | network or home |
| **Automated backups to TrueNAS** | Easy | Daily encrypted backup of RedNode state to TrueNAS NFS share | storage |

### 🔒 Tier 2: Advanced Security (leverage your security stack)

| Feature | Difficulty | What It Does | Agent |
|---|---|---|---|
| **Intrusion Detection System (IDS)** | Medium | Suricata/Snort on pfSense → RedNode correlates alerts → auto-blocks attackers | security + network |
| **Log aggregation & alerting** | Medium | Collect logs from pfSense, Pi-hole, TrueNAS, cameras → unified timeline → anomaly detection | system + security |
| **SSL certificate management** | Easy | Auto-renew Let's Encrypt certs (or self-signed), alert on expiry | security |
| **Vulnerability scanning** | Medium | Nightly scan of all network devices for known vulns (Nmap + NVD) | security + network |
| **Honeypot deployment** | Medium | Deploy lightweight honeypot on VLAN 40 (guest) → alert on any connection | security |
| **Fail2ban orchestration** | Easy | Centralized fail2ban across all services, with RedNode correlation | security |
| **Tor hidden service** | Medium | Expose specific RedNode interfaces via Tor .onion — access from anywhere, zero port forwarding | network |
| **WireGuard VPN auto-config** | Medium | One-command VPN setup → access RedNode dashboard from anywhere | network |

### 🧠 Tier 3: Intelligence & Automation (make it smarter)

| Feature | Difficulty | What It Does |
|---|---|---|
| **Multi-model routing** | Medium | Route different intents to different LLM models — code tasks to CodeLlama, chat to Qwen, reasoning to DeepSeek |
| **RAG over your documents** | Easy | Ingest your personal PDFs, notes, bookmarks → "what did that contract say about payment terms?" (research-agent already has kb.ingest) |
| **Automated research assistant** | Medium | "Research X topic" → multi-source search → summarize → save to knowledge base → brief me tomorrow |
| **Email auto-triage** | Medium | Classify incoming email (urgent/normal/spam), draft replies, summarize daily digest |
| **Smart notifications** | Medium | Context-aware alerts — don't wake me at 3 AM for a non-critical camera event; batch and deliver at 7 AM |
| **Habit tracking** | Easy | Learn your patterns from audit log, suggest optimizations — "You usually check cameras at 9 AM but missed today" |
| **Conversation memory** | Already built | Multi-turn sessions with reference resolution — "what did I ask about yesterday?" |
| **Predictive maintenance** | Medium | Track disk SMART trends, RAM errors, CPU thermals → predict failures before they happen |
| **Natural language automation** | Medium | "Every weekday at 8 AM, check cameras, check weather, summarize email, and send me a Signal message" |

### 💻 Tier 4: Development & Productivity

| Feature | Difficulty | What It Does |
|---|---|---|
| **Git repository watcher** | Easy | Monitor your GitHub repos for PRs, issues, CI failures — "anything new on my projects?" |
| **Code deployment pipeline** | Medium | "Deploy my latest code to production" → git pull → build → test → deploy → verify |
| **Local CI/CD runner** | Medium | Run tests, linting, builds on your local machine — no GitHub Actions credits |
| **Documentation generator** | Easy | "Generate API docs for this project" → analyzes code → generates markdown |
| **Time tracking** | Easy | Track time spent on projects, generate reports — all local |
| **Clipboard sync** | Easy | Sync clipboard between your devices via RedNode (encrypted, local) |

### 🏡 Tier 5: Smart Home Deep Integration

| Feature | Difficulty | What It Does |
|---|---|---|
| **Scene learning** | Medium | Learn which lights/climate settings you prefer at different times → auto-adjust |
| **Presence detection** | Medium | Combine camera + phone + network presence → "is anyone home?" → auto-arm/disarm |
| **Energy monitoring** | Medium | Track power consumption per device (smart plugs), optimize schedules |
| **Voice announcements** | Easy | Piper TTS → broadcast to speakers — "Package delivered at front door" |
| **Appliance monitoring** | Easy | Smart plug power draw analysis → "washing machine just finished" |
| **Garden/plant monitoring** | Easy | Soil moisture sensors → "water the tomatoes" or auto-watering |

### 🌐 Tier 6: Network & Communications

| Feature | Difficulty | What It Does |
|---|---|---|
| **Matrix/Element chat server** | Medium | Self-hosted encrypted chat — alternative/complement to Signal |
| **Nextcloud integration** | Medium | Self-hosted file sync, contacts, calendar — replace Google Drive |
| **RSS/news aggregator** | Easy | Pull RSS feeds → AI summarize → daily briefing |
| **Podcast downloader** | Easy | Auto-download subscribed podcasts, transcribe with Whisper, summarize |
| **Website uptime monitor** | Easy | Monitor your websites/services → alert on downtime |
| **DNS-over-HTTPS** | Easy | Run DoH server → all devices get encrypted DNS via Pi-hole |

### 🎨 Tier 7: Media & Creative

| Feature | Difficulty | What It Does |
|---|---|---|
| **Photo management** | Medium | Auto-organize photos by date/location/face, searchable — "show me photos from December" |
| **Music server** | Easy | Navidrome/Subsonic → stream your music library |
| **Video transcription** | Medium | Whisper on local video files → searchable transcripts |
| **Image generation** | Medium | Stable Diffusion local → "generate a logo for my project" (needs GPU VRAM) |
| **Audiobook library** | Easy | Audiobookshelf → manage + listen to audiobooks |

### 🔧 Tier 8: System & Hardware

| Feature | Difficulty | What It Does |
|---|---|---|
| **Multi-node cluster** | Hard | Run RedNode on multiple machines, distributed agents, shared memory |
| **GPU passthrough** | Medium | Dedicate GPU to specific tasks (LLM vs Frigate) dynamically |
| **ZFS pool management** | Medium | If using ZFS instead of btrfs — auto-scrub, auto-expand, health monitoring |
| **USB device manager** | Easy | Detect plugged USB devices, auto-mount, auto-backup |
| **Firmware update manager** | Medium | Track firmware versions of network devices → alert on available updates |
| **Temperature/fan control** | Easy | Monitor CPU/GPU temps → adjust fan curves → alert on thermal throttling |

---

## Recommended First Additions After Deployment

Based on your homelab (pfSense, Pi-hole, TrueNAS, cameras, Home Assistant), here's the highest-value sequence:

### Week 1: Core Integration
1. **Connect pfSense API** — edit `.env` with pfSense credentials → Network Agent starts managing firewall
2. **Connect Pi-hole API** — edit `.env` → Infra Agent starts monitoring DNS
3. **Connect TrueNAS API** — edit `.env` → Storage Agent starts monitoring pools/SMART
4. **Connect cameras to Frigate** — edit `deployment/frigate.yml` → Surveillance Agent starts processing events

### Week 2: Automation
5. **Set up morning workflow** — "Every day at 7:30 AM: check cameras, check weather, summarize email, send Signal"
6. **Auto-threat-blocking** — connect security agent threat intel to network agent firewall rules
7. **Automated TrueNAS snapshots** — daily snapshots + weekly pruning

### Week 3: Intelligence
8. **Ingest your documents** — PDFs, notes, bookmarks into the knowledge base
9. **Let patterns build** — after 7 days, predictive intent kicks in
10. **Fine-tune voice** — adjust wake word, Piper voice, response latency

### Month 2+: Expand
11. WireGuard VPN for remote access
12. RSS/news aggregation
13. Email auto-triage
14. Photo management
15. Multi-model LLM routing

---

## What's NOT Built (and Why)

| Feature | Reason |
|---|---|
| **Emotional intelligence** | Rejected — deploy first, iterate on real problems |
| **GUI installer TUI** | Deferred — NixOS + self-heal handles installation |
| **Multi-user auth** | Single-owner system — multi-user is future |
| **Appliance ISO with models baked in** | ISO would be ~6.2 GB — model download on first boot is fine |
| **ARM64 / Raspberry Pi** | Ollama performance on ARM is poor — x86_64 only for now |
| **Social media API keys** | Agent built, needs your credentials to activate |
| **Kubernetes** | Overkill for personal system — one machine, systemd + Docker |
