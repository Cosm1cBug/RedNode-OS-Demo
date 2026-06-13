# RedNode-OS Quick Start – v0.2

```bash
# Infra
cd deployment && docker compose up -d
# nats :4222, postgres :5432, qdrant :6333, ollama :11434, grafana :3001, loki :3100

# Models
ollama pull qwen2.5:14b-instruct-q4_K_M
ollama pull nomic-embed-text

# CNS – Rust
cd core/rednode-core && cargo run
# → http://localhost:8787

# Agents – TS / NATS
cd ../.. && pnpm install && pnpm agents

# Web – Next.js
pnpm web
# → http://localhost:3000

# CLI
pnpm --filter @rednode/cli dev -- intent "harden ssh and show docker status"
```

API:
```
POST http://localhost:8787/intent
{"intent":"analyze system health"}
```

Desktop: `cd interfaces/desktop && pnpm tauri dev`
Mobile: `cd interfaces/mobile && flutter run`
Voice: `cd interfaces/voice && python stt_server.py`

Observability: Grafana http://localhost:3001 admin/rednode
