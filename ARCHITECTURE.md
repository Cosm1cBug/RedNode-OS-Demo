# RedNode-OS Architecture

```
Human Intent → Interface Layer → CNS (Rust) → Agent Society (TS) → Execution Layer → Host OS → Hardware
```

## Central Nervous System — Rust — `core/rednode-core`

16 modules:

| Module | Lines | What It Does |
|---|---|---|
| `planner.rs` | 337 | LLM-powered planning via Ollama (Qwen2.5) with keyword fallback |
| `sentience.rs` | 686 | Self-model, 5 homeostatic drives (real data), autonomous goal execution, memory consolidation |
| `memory.rs` | 550+ | PostgreSQL + Qdrant vectors + Kuzu/Postgres knowledge graph, RAG pipeline, entity extraction |
| `executor.rs` | 380 | Sandboxed tool execution (firejail/bubblewrap/seccomp) with resource limits |
| `api.rs` | 350+ | 17 REST endpoints + real-time WebSocket event streaming |
| `security.rs` | 220+ | 122 tools risk-tagged, 25+ deny patterns, path traversal + injection protection |
| `events.rs` | 137 | `tokio::broadcast` event bus — all modules publish, WebSocket subscribes |
| `auth.rs` | 107 | Bearer token middleware with constant-time comparison |
| `coordinator.rs` | 97 | Plan execution: security check → approval gate → NATS dispatch → audit |
| `intent_router.rs` | 56 | RAG context enrichment before planning |
| `bus.rs` | 66 | NATS JetStream client (safe, OnceCell) |
| `init.rs` | 445 | PID1 mode: mount filesystems, supervise services, signal handling, watchdog |
| `pii.rs` | 227 | PII detection pipeline: 14 types, auto-redact/block/log before memory ingestion |
| `goap.rs` | 248 | Goal-Oriented Action Planning: A* search with preconditions, costs, dependency ordering |
| `main.rs` | 49 | Entry point: events → memory → bus → executor → sentience → API |
| `lib.rs` | 12 | Module declarations |

## Agent Society — 16 Agents — TypeScript — NATS

Each agent connects to NATS, subscribes to `rednode.agent.{name}.task`, and dispatches tool calls to the Rust executor via `rednode.tool.exec`.

| Agent | Subject | Tools | Integration |
|---|---|---|---|
| System | `rednode.agent.system.*` | 6 | OS, Docker, processes, filesystem |
| Security | `rednode.agent.security.*` | 7 | CVE (NVD sync), Falco eBPF, threat intel (abuse.ch/OTX), auto-patcher |
| Coding | `rednode.agent.coding.*` | 5 | Ollama codegen, clippy, tests, git |
| Research | `rednode.agent.research.*` | 8 | RAG, SearXNG, OCR, PDF, knowledge graph |
| Automation | `rednode.agent.automation.*` | 4 | Workflows, scheduler, triggers |
| Network | `rednode.agent.network.*` | 8 | Connections, firewall, DNS, VPN, device isolation |
| Infrastructure | `rednode.agent.infra.*` | 9 | Pi-hole v6 API |
| Storage | `rednode.agent.storage.*` | 14 | TrueNAS REST API v2.0 |
| Surveillance | `rednode.agent.surveillance.*` | 11 | Frigate MQTT + REST API |
| Communications | `rednode.agent.comms.*` | 10 | IMAP, SMTP, CalDAV |
| Productivity | `rednode.agent.productivity.*` | 10 | Notes, tasks, bookmarks |
| Media | `rednode.agent.media.*` | 7 | Jellyfin API |
| Home | `rednode.agent.home.*` | 7 | Home Assistant REST API |
| Browser | `rednode.agent.browser.*` | 7 | Playwright + cheerio (stealth) |
| Social | `rednode.agent.social.*` | 9 | Twitter/X, Mastodon, Bluesky, LinkedIn, Instagram, WhatsApp |
| Signal Bot | (standalone) | — | E2EE messaging via signal-cli |

## Memory

- **PostgreSQL 16** — intentions, audit_log (SHA-256 hash-chained), security_events, approvals, documents, knowledge graph (kg_entities, kg_relationships)
- **Qdrant** — 768-dimensional vector embeddings, cosine similarity, collection `rednode_docs`
- **Kuzu** (optional, `--features kuzu`) — embedded graph DB, Cypher queries. Falls back to Postgres JSON tables.
- **Ollama** — `nomic-embed-text` for embeddings, `qwen2.5` for LLM planning/generation

## Execution

Tool Registry (`tools.json`) — 122 tools, each with name, agent, risk level, description. Executor runs commands inside firejail/bubblewrap sandbox with seccomp BPF, resource limits, and timeout. Every execution is hash-chain audited.

### Parallel Execution

The coordinator groups plan steps by agent. Steps targeting **different agents run concurrently** via `tokio::spawn`. Steps targeting the **same agent run sequentially** (agents handle one task at a time). Results are sorted back to original plan order before returning.

### State Caching

Within a single intent execution, each step's result is cached in a shared `Arc<RwLock<HashMap>>`. Later steps receive all previous results via `_state_cache` in their args — no redundant re-fetching.

### Proposition-Level Memory

When a document is ingested, a background task extracts 3-8 atomic factual propositions via the LLM, embeds each separately in Qdrant with `type: "proposition"` and a `parent_doc` reference. RAG search returns fine-grained facts alongside whole-document matches.

## Event Bus

`tokio::broadcast` channel (capacity 512). Publishers: sentience, coordinator, API handlers. Subscribers: WebSocket clients (dashboard). 9 typed emitters: intent, plan, tool_result, drives, goal, security_event, agent_heartbeat, approval_needed.

## Observability

OpenTelemetry → OTEL Collector → Loki (logs) + Prometheus (metrics) → Grafana (dashboards). Security telemetry: Falco eBPF + threat intel feeds.

## Interfaces

- **Web**: Next.js 14 — 13-tab dashboard (localhost:3000)
- **Mobile**: Flutter 3.22 — biometric approvals, FCM push, WireGuard
- **Desktop**: Tauri 2 — native window (~8 MB)
- **CLI**: TypeScript — 19 commands
- **Voice**: Whisper STT + Piper TTS + customizable wake word
- **Signal Bot**: E2EE messaging via signal-cli
- **REST API**: Axum — 17 endpoints (localhost:8787)
- **WebSocket**: Real-time event stream (ws://localhost:8787/events)

All interfaces are thin clients. Intelligence lives in the CNS.
