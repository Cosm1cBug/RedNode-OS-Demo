# RedNode-OS Architecture

Human Intent → Interface Layer → CNS (Rust) → Agent Society (TS) → Execution Layer → Host Adapter → Hardware

## Central Nervous System – Rust – `core/rednode-core`

- intent_router – NL → structured intention
- planner – ReAct planner
- coordinator – dispatches to NATS `rednode.agent.*`
- security_validator – OPA-style policy, risk Low/Med/High/Critical
- memory – Postgres + Qdrant + Kuzu
- executor – firejail/bubblewrap sandbox
- bus – NATS JetStream
- api – Axum – POST /intent, WS /events
- otel – OpenTelemetry tracing

## Agent Society – TypeScript – NATS

`rednode.agent.system.*`, `rednode.agent.security.*`, `rednode.agent.coding.*`, `rednode.agent.research.*`, `rednode.agent.automation.*`, `rednode.agent.network.*`

## Memory

- PostgreSQL – intentions, tasks, audit_log, security_events, preferences
- Qdrant – vector embeddings, 768d, collection `rednode_docs`
- Kuzu – knowledge graph – Project→Tech→Repo→File→Function

## Execution

Tool Registry – JSON Schema, risk-tagged. Sandboxed execution, audit log hash-chained.

## Observability

OpenTelemetry → OTEL Collector → Loki / Prometheus → Grafana

Security Telemetry: eBPF + Falco

## Interfaces

- Web: Next.js 14 – localhost:3000
- Desktop: Tauri 2 – Rust backend, Next.js frontend
- Mobile: Flutter
- CLI: TS / Commander
- Voice: Python – faster-whisper / Piper
- API: Axum – localhost:8787

All thin clients. CNS is the brain.
