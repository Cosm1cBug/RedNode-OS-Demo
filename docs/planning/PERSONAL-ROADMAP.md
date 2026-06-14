# RedNode-OS — Where You Actually Stand + Personal Use Roadmap

> **Brutally honest assessment, then a concrete build plan for personal use only.**

---

## Part 1: Where You Actually Stand

### The Honest Truth

You have a **well-architected skeleton with a few solid organs** — not a functional body. The architecture diagram is excellent. The security concepts are real. But most of the "working" subsystems are scaffolding, stubs, or simulations.

**Total codebase: ~4,380 lines across all languages.** For context, a single medium Next.js app is 5,000–15,000 lines.

---

### Component-by-Component Reality Check

```
 COMPONENT                       STATUS              LINES    VERDICT
 ─────────────────────────────── ─────────────────── ──────── ──────────────────────

 RUST CORE (CNS)
 ├── executor.rs                 ✅ REAL              380 L    Production-grade sandboxing.
 │                                                            firejail/bwrap/seccomp, allowlists,
 │                                                            timeout, kill_on_drop. Best code
 │                                                            in the entire repo.
 │
 ├── memory.rs                   🟡 PARTIAL           489 L    Postgres schema + audit log +
 │                                                            Qdrant client + RAG pipeline
 │                                                            all CODED. But: falls back to
 │                                                            static hardcoded strings if
 │                                                            Qdrant/Ollama aren't running.
 │                                                            Kuzu is a stub (returns JSON
 │                                                            saying "build with --features kuzu").
 │
 ├── sentience.rs                🟡 PARTIAL           300 L    Self-model struct + drives +
 │                                                            introspection loop + goal gen
 │                                                            all CODED. But: drives are mostly
 │                                                            hardcoded (security=0.9, knowledge=0.75).
 │                                                            Goals are generated but NEVER
 │                                                            executed (TODO comment at line 214).
 │                                                            Memory consolidation is a 200ms sleep.
 │                                                            Resource sampling uses real sysinfo
 │                                                            crate (CPU/RAM are real).
 │
 ├── api.rs                      ✅ REAL              146 L    All 12 REST endpoints work.
 │                                                            WebSocket handler exists but sends
 │                                                            only "hello" (TODO: forward events).
 │
 ├── coordinator.rs              ✅ REAL               65 L    Actually dispatches to agents via
 │                                                            NATS, falls back to local executor.
 │
 ├── planner.rs                  ❌ STUB               27 L    KEYWORD MATCHING ONLY.
 │                                                            if contains("ssh") && contains("harden")
 │                                                            Five hardcoded if/else blocks.
 │                                                            NOT an LLM planner. NOT intelligent.
 │                                                            This is the single biggest gap.
 │
 ├── intent_router.rs            ❌ PASSTHROUGH          4 L    Just calls coordinator. Does nothing.
 │
 ├── security.rs                 🟡 BASIC              20 L    Risk levels + deny patterns work.
 │                                                            But: only 5 deny patterns. No OPA.
 │                                                            No policy engine. Just a match statement.
 │
 └── bus.rs                      ⚠️ UNSAFE             38 L    Works but uses `static mut` +
                                                              `unsafe` blocks. Not thread-safe.
                                                              Fine for demo, needs rewrite for prod.

 AGENTS (TypeScript)
 ├── shared/agent.ts             ✅ REAL              103 L    Base class actually works — NATS
 │                                                            connect, heartbeat, tool dispatch,
 │                                                            request/reply to Rust executor.
 │
 ├── security-agent              🟡 SIMULATED         313 L    CVE checker: scans dpkg BUT uses
 │   ├── cve.ts                                               LOCAL_CVE_DB with 2 hardcoded CVEs
 │   ├── falco.ts                                             (both "simulated for demo"). Falco:
 │   └── patcher.ts                                           tails real log OR runs simulator
 │                                                            (1 fake event / 90s). Patcher:
 │                                                            "95% simulated success rate" —
 │                                                            doesn't actually run apt upgrade.
 │
 ├── system-agent                ❌ SHELL               25 L    Path traversal check for fs.read,
 │                                                            then falls through to Rust executor.
 │                                                            No agent-specific logic at all.
 │
 ├── coding-agent                ❌ EMPTY                8 L    Constructor + connect + serve.
 │                                                            Zero logic. Declares tools it
 │                                                            doesn't implement.
 │
 ├── automation-agent            ❌ EMPTY                8 L    Same. No workflow engine. No scheduler.
 │
 ├── research-agent              ❌ EMPTY               12 L    handleTool returns null with
 │                                                            comment "Phase 2".
 │
 └── network-agent               ❌ EMPTY               12 L    handleTool returns null with
                                                              comment "validate egress targets".

 INTERFACES
 ├── Web (Next.js)               🟡 FUNCTIONAL        ~400 L   8-tab dashboard, all components
 │                                                            fetch from real API endpoints.
 │                                                            SentiencePanel is 148 lines (most
 │                                                            complete component). But: no auth,
 │                                                            no error boundaries, basic styling.
 │
 ├── Mobile (Flutter)            🟡 FUNCTIONAL        ~994 L   6 pages, biometric auth, FCM push,
 │                                                            WireGuard, secure storage. Most
 │                                                            complete interface. Actually usable
 │                                                            IF the backend works.
 │
 ├── CLI                         ❌ MINIMAL             14 L    3 commands: intent, health, agents.
 │                                                            Works but bare-bones.
 │
 ├── Desktop (Tauri)             ❌ EMPTY               11 L    Returns "rednode-cns ok" string.
 │                                                            No actual desktop integration.
 │
 └── Voice (Python)              ❌ STUB                14 L    Returns "[stt stub]" and {"wav":"stub"}.
                                                              Zero actual Whisper/Piper integration.

 OS / DEPLOYMENT
 ├── NixOS config                ✅ REAL              ~627 L   Comprehensive. Hardened kernel,
 │                                                            LUKS, TPM, services, flake.nix
 │                                                            with ISO build. Looks production-ready
 │                                                            but UNTESTED (no CI, no test infra).
 │
 ├── Docker Compose              ✅ REAL               ~80 L   All 8 services defined. Would work
 │                                                            if you run docker compose up.
 │
 ├── Security configs            🟡 DOCS ONLY         ~100 L   Falco rules, seccomp profile,
 │                                                            policy.json — correct format but
 │                                                            never consumed by code.
 │
 └── Tests                       🟡 EXIST              ~95 L   7 integration tests. They test real
                                                              things (deny list, risk levels, audit
                                                              hash chain, RAG fallback, approvals).
                                                              But: skip if no Postgres. No CI.
```

---

### Summary Scorecard

| Area | Score | Explanation |
|---|---|---|
| **Architecture / Design** | 9/10 | Excellent. The layered design, agent society model, security-first thinking, memory stack choice — all brilliant. |
| **Executor / Sandbox** | 8/10 | Best code in the repo. Real firejail/bwrap/seccomp, real allowlists, real resource limits. |
| **Memory / RAG** | 6/10 | Code exists for Postgres + Qdrant + Kuzu. But: untested end-to-end with real data. Falls back to hardcoded strings. |
| **Sentience Engine** | 5/10 | Impressive design. Real CPU/RAM sampling. But: drives are mostly hardcoded, goals never execute, "dreaming" is a sleep(200ms). |
| **API / Endpoints** | 7/10 | 12 endpoints, all wired. WebSocket is a stub. No auth. |
| **Planner** | 2/10 | Five if/else keyword matches. This is the brain of the system and it's the weakest part. |
| **Agents** | 3/10 | Only Security Agent has real (simulated) logic. 4 out of 6 agents are empty shells — 8 lines each. |
| **Web Dashboard** | 6/10 | Works if backend is up. Basic but functional. |
| **Mobile App** | 7/10 | Most complete interface. Biometric, FCM, WireGuard — real integrations. |
| **Voice** | 0/10 | Two 7-line files that return hardcoded strings. |
| **Desktop** | 1/10 | One function that returns "ok". |
| **CLI** | 3/10 | Works, but 3 commands in 14 lines. |
| **NixOS / OS** | 7/10 | Comprehensive config, but never built/tested as an actual ISO. |
| **Tests** | 4/10 | 7 tests exist and test real things. No CI. Skip if no infra. |
| **Documentation** | 8/10 | README, ARCHITECTURE, SECURITY, QUICKSTART — all well-written and accurate. |
| **OVERALL** | **5/10** | A well-designed prototype with strong security foundations, but half the "features" are stubs or simulations. You have the skeleton and one strong arm (executor). Everything else needs flesh. |

---

### Where You ARE on the Original Roadmap

```
Phase 1 – Foundation     [██████░░░░] 60% — skeleton done, but agents hollow
Phase 2 – Intelligence   [░░░░░░░░░░]  0% — no LLM planner, no voice, no agent collab
Phase 3 – Security       [██░░░░░░░░] 20% — simulated CVE/Falco, no real self-healing
Phase 4 – Operating      [░░░░░░░░░░]  0% — not started
Phase 5 – 1.0            [░░░░░░░░░░]  0% — not started
```

**You are at roughly Phase 1 — 60% done.** The hardest part (architecture + executor + security model) is done. The most impactful part (making agents actually DO things, LLM planner) is not.

---

---

## Part 2: Personal Use Roadmap

> This roadmap is **for YOU** — to make RedNode a system you actually use daily in your home. No commercial features, no marketplace, no enterprise. Just a personal autonomous OS that manages your infrastructure, your work, and your life.

---

### Week 0 — Get It Running (3 days)

**Goal: Boot the system and see the dashboard. Don't build anything new yet.**

```
Day 1:
  □ Install NixOS on your old PC (bare metal)
  □ Install NVIDIA drivers
  □ docker compose up -d (NATS, Postgres, Qdrant, Ollama, Grafana)
  □ ollama pull qwen2.5:7b (or 14b if 12GB GPU)
  □ ollama pull nomic-embed-text
  □ cargo run (CNS starts on :8787)

Day 2:
  □ pnpm install && pnpm agents (all 6 connect to NATS)
  □ pnpm web (dashboard on :3000)
  □ curl POST /intent {"intent":"show system health"} — see it work
  □ Verify: audit log entries appear in Postgres
  □ Verify: Qdrant collection created
  □ Verify: Sentience Engine logs drives every second

Day 3:
  □ Set up pfSense on mini-PC
  □ Set up Pi-hole on Raspberry Pi
  □ Configure VLANs (10, 20, 30, 40, 50)
  □ RedNode server on VLAN 50
  □ Verify: all devices get DNS from Pi-hole
  □ Verify: cameras on VLAN 30 have zero internet
```

**Exit: You can open http://10.0.50.10:3000 from your workstation and see the dashboard.**

---

### Phase 1 — Complete the Foundation (2 weeks)

**Goal: Make what exists actually work end-to-end. No new features — fix the stubs.**

#### Week 1: Fix the Core

```
□ PLANNER — Replace keyword matching with LLM
  - Call Ollama Qwen2.5 from planner.rs
  - Prompt: "Given this intent, return a JSON array of PlanSteps"
  - System prompt describes available tools + agents + risk levels
  - Falls back to keyword matching if Ollama is down
  - THIS IS THE SINGLE HIGHEST IMPACT CHANGE

□ BUS — Fix unsafe global state
  - Replace `static mut BUS` with Arc<RwLock<Bus>> or OnceCell
  - 15 minutes of work, eliminates undefined behavior

□ SENTIENCE — Wire drives to real data
  - Security drive: query security_events count in last hour
  - Integrity drive: check agent heartbeats (NATS)
  - Knowledge drive: count Qdrant documents
  - Energy drive: read /sys/class/power_supply (if laptop) or UPS
  - Availability drive: check Postgres + Qdrant + Ollama connectivity

□ SENTIENCE — Execute generated goals
  - Uncomment TODO at line 214
  - When goal is generated → call coordinator::coordinate()
  - Low-risk goals: auto-execute
  - High-risk goals: create approval, push to mobile
```

#### Week 2: Flesh Out Agents

```
□ SYSTEM AGENT — Add real handling
  - handleTool for fs.read: format output nicely, add file metadata
  - handleTool for docker.ps: parse output, detect unhealthy containers
  - handleTool for process.list: highlight high-CPU processes
  - handleTool for service.status: return structured health data

□ CODING AGENT — Make code.analyze work
  - handleTool: run clippy / eslint on a given path
  - handleTool: code.test → run cargo test or pnpm test
  - handleTool: code.generate → call Ollama with coding prompt

□ NETWORK AGENT — Make net.status useful
  - handleTool: parse ss output into structured JSON
  - handleTool: dns.check → query Pi-hole API for status
  - handleTool: traffic.analyze → top connections by bandwidth

□ RESEARCH AGENT — Connect to RAG
  - handleTool: research.query → call /memory/query, format results
  - handleTool: kb.ingest → call /memory/ingest with source content

□ WEBSOCKET — Forward real events
  - api.rs ws_handler: subscribe to NATS rednode.* events
  - Forward each event to connected WebSocket clients
  - Dashboard EventStream tab starts showing live data
```

**Exit: You can say "analyze system health" and get a real, LLM-planned, multi-step response with actual system data. Sentience Engine generates and executes its own goals.**

---

### Phase 2 — Your Home Infrastructure (3 weeks)

**Goal: RedNode sees and controls your Pi-hole, TrueNAS, and cameras.**

#### Week 3: Pi-hole + TrueNAS

```
□ INFRASTRUCTURE AGENT — new agent
  agents/infra-agent/src/index.ts
  - pihole.stats → GET http://pihole-ip/api/stats/summary
  - pihole.top_blocked → top blocked domains
  - pihole.top_clients → most active devices
  - pihole.disable → POST /api/dns/blocking (with timer)
  - pihole.anomaly → detect unusual query patterns (new domains spike)

□ STORAGE AGENT — new agent
  agents/storage-agent/src/index.ts
  - nas.health → GET https://truenas-ip/api/v2.0/pool
  - nas.disks → GET /api/v2.0/disk (SMART status)
  - nas.usage → GET /api/v2.0/pool/dataset
  - nas.snapshot_create → POST /api/v2.0/zfs/snapshot
  - nas.alerts → GET /api/v2.0/alert/list

□ Wire TrueNAS health into Sentience Engine
  - Integrity drive: pool health + disk SMART
  - Availability drive: storage capacity > 80% → drive drops

□ Nightly RedNode backup
  - Cron: pg_dump → SMB share on TrueNAS
  - Qdrant snapshot → TrueNAS
```

#### Week 4-5: Frigate + Surveillance

```
□ Deploy Frigate NVR on RedNode server (Docker)
  - docker-compose service for Frigate
  - Configure RTSP streams from your NVR
  - GPU passthrough for TensorRT detection
  - MQTT broker (Mosquitto) for Frigate events

□ SURVEILLANCE AGENT — new agent
  agents/surveillance-agent/src/index.ts
  - cam.status → GET frigate-ip:5000/api/stats
  - cam.events → GET /api/events?after=timestamp
  - cam.snapshot → GET /api/camera_name/latest.jpg
  - cam.clip → GET /api/events/:id/clip.mp4

□ MQTT → NATS bridge
  - Subscribe to frigate/events on MQTT
  - Republish to rednode.surveillance.event on NATS
  - Surveillance Agent handles, creates security_events

□ Smart alerts
  - Person at unusual hour → CRITICAL security event
  - Push to mobile with snapshot via FCM
  - Cross-correlate with Pi-hole (device DNS + camera detection)

□ Wire cameras into Sentience Engine
  - Integrity drive: all cameras online?
  - Security drive: any unacknowledged person detections?
```

**Exit: "Show me who was at the front door today" returns Frigate events with clips. "How healthy is my NAS?" returns real pool/disk status. Pi-hole anomalies generate security events.**

---

### Phase 3 — Your Daily Workflows (3 weeks)

**Goal: RedNode handles recurring patterns in your life.**

#### Week 6: Automation Engine

```
□ AUTOMATION AGENT — make it real
  - Implement workflow.create: store workflow as JSON in Postgres
  - Implement workflow.run: execute a sequence of tool calls
  - Implement schedule.add: node-cron for recurring workflows
  - Implement trigger.fire: event-based triggers (Frigate event → action)

□ Built-in workflows:
  - "Goodnight" → Pi-hole strict mode + camera alerts + TrueNAS snapshot
  - "I'm leaving" → all cameras active + WireGuard for remote
  - "Focus mode" → Pi-hole blocks social media for 2 hours
  - "Morning brief" → system health + storage status + overnight events
```

#### Week 7: Voice Interface

```
□ STT server — real Whisper integration
  - Install faster-whisper
  - POST /transcribe accepts audio, returns text
  - Test with microphone on RedNode server

□ TTS server — real Piper integration
  - Install piper-tts
  - POST /speak accepts text, returns WAV audio
  - Play through speakers on RedNode server

□ Voice loop
  - Wake word detection (OpenWakeWord or Porcupine)
  - Wake → record → Whisper → intent → CNS → response → Piper → speak
  - Target: < 2 seconds end-to-end (relax from 1.2s for personal use)
```

#### Week 8: CLI + Desktop Polish

```
□ CLI — expand to be actually useful
  - rednode status → full system overview (agents, drives, storage, cameras)
  - rednode goodnight / rednode focus / rednode morning
  - rednode cameras → list cameras, recent events
  - rednode nas → pool health, disk status
  - rednode pihole → DNS stats, top blocked
  - rednode log → last 20 audit entries
  - rednode memory search <query> → RAG search

□ Desktop (Tauri) — wrap web UI properly
  - Load Next.js dashboard in Tauri webview
  - System tray icon with quick actions
  - Native notifications for approvals and security events
```

**Exit: You wake up, say "good morning" to RedNode, and it tells you overnight camera events, system health, DNS stats, and today's tasks. At night you say "goodnight" and it locks down your network.**

---

### Phase 4 — Your Digital Life (4 weeks)

**Goal: Email, notes, browsing — the personal stuff.**

#### Week 9-10: Communications + Productivity

```
□ COMMUNICATIONS AGENT
  - email.fetch → IMAP connection to your email
  - email.summarize → Ollama summarizes unread emails
  - email.draft → LLM drafts reply, you approve before send
  - calendar.view → CalDAV integration (Nextcloud/Google via DAV)
  - notifications.digest → aggregate all alerts into morning brief

□ PRODUCTIVITY AGENT
  - notes.create → save Markdown notes to /var/lib/rednode/notes/
  - notes.search → RAG search across all notes (ingest into Qdrant)
  - tasks.create / tasks.list / tasks.complete → local task tracker
  - bookmarks.save → save + auto-summarize URLs
```

#### Week 11-12: Browser + Social

```
□ BROWSER AGENT
  - browser.search → SearXNG self-hosted meta-search
  - browser.scrape → Playwright extracts page content
  - browser.read → reader mode for articles

□ SOCIAL MEDIA AGENT (if you need it)
  - social.post → Twitter/X API, LinkedIn, Mastodon
  - social.draft → LLM drafts post, you approve
  - social.analytics → engagement metrics
```

**Exit: "Summarize my emails" works. "Save a note about RedNode architecture" creates a searchable note. "Search the web for NixOS GPU passthrough" returns SearXNG results without tracking.**

---

### Phase 5 — Harden + Polish (2 weeks)

**Goal: Make it reliable enough to trust daily.**

```
Week 13:
  □ Add authentication to API (JWT or session-based)
  □ Add error boundaries to all web components
  □ Add reconnection logic to all agents (auto-restart on NATS disconnect)
  □ Add health checks: if Ollama is down → degrade gracefully, don't crash
  □ Write 20+ more integration tests
  □ Set up systemd services for all agents (auto-start on boot)

Week 14:
  □ Portable state export/import
    - rednode export → tar + age-encrypt Postgres dump + Qdrant snapshot + configs
    - rednode import → restore everything on new hardware
  □ UPS monitoring (NUT integration) → Energy drive
  □ Grafana dashboards for RedNode metrics
  □ Documentation: personal setup guide for your exact hardware
```

**Exit: RedNode starts on boot, survives reboots, recovers from crashes, and you can move your entire setup to new hardware with one command.**

---

### Timeline Summary

| Phase | Duration | What You Get |
|---|---|---|
| **Week 0** | 3 days | Everything running, VLANs configured |
| **Phase 1** | 2 weeks | LLM planner, real agents, live sentience |
| **Phase 2** | 3 weeks | Pi-hole + TrueNAS + Frigate integrated |
| **Phase 3** | 3 weeks | Workflows, voice, CLI, desktop |
| **Phase 4** | 4 weeks | Email, notes, browsing, social |
| **Phase 5** | 2 weeks | Auth, reliability, portable state |
| **TOTAL** | **~14 weeks** | Personal autonomous OS for daily use |

---

### What This Roadmap Does NOT Include (Commercial Stuff)

These are **excluded** from your personal roadmap — they're only needed for selling:

- ❌ RedNode Pro subscription tier
- ❌ Pre-built hardware product
- ❌ Enterprise multi-user RBAC
- ❌ Agent Marketplace
- ❌ App Store APK/IPA signing
- ❌ Marketing website / Product Hunt launch
- ❌ Consulting services setup
- ❌ SBOM / Cosign for distribution
- ❌ One-click installer for other people
- ❌ Documentation for external users
- ❌ Revenue tracking / analytics

You can always add these later if you decide to commercialize. The personal version is the foundation for the commercial one — not the other way around.

---

### The One Thing to Do First

If you do nothing else this week, do this:

**Replace `planner.rs` with an LLM call to Ollama.**

Those 27 lines of keyword matching are the single biggest bottleneck. Once the planner is intelligent, every intention you type becomes meaningful — and every other fix multiplies in value. Without it, RedNode is a fancy tool executor. With it, RedNode starts thinking.

```rust
// The 27-line planner you have now:
if s.contains("ssh") && s.contains("harden") { ... }
if s.contains("docker") { ... }

// What it should be (pseudocode):
let prompt = format!(
    "You are RedNode-OS planner. Given this intent: '{}'\n\
     Available tools: {:?}\n\
     Return a JSON array of steps.",
    intent, TOOL_REGISTRY
);
let response = ollama::generate(&prompt).await;
let steps: Vec<PlanStep> = serde_json::from_str(&response);
```

That one change transforms RedNode from a demo into a system you'd actually want to talk to.
