# RedNode-OS Quick Start

## Option A: One-Command Start (recommended)

```bash
# 1. Start infrastructure
cd deployment && docker compose up -d
# NATS, Postgres, Qdrant, Ollama, Mosquitto, Frigate, SearXNG, Grafana

# 2. Pull AI models
ollama pull qwen2.5:14b-instruct-q4_K_M
ollama pull nomic-embed-text

# 3. Start everything
./scripts/start-all.sh
# → CNS: http://localhost:8787
# → Dashboard: http://localhost:3000
# → Grafana: http://localhost:3001
```

## Option B: Step-by-Step

```bash
# 1. Infrastructure
cd deployment && docker compose up -d && cd ..

# 2. Models
ollama pull qwen2.5:14b-instruct-q4_K_M
ollama pull nomic-embed-text

# 3. CNS (Rust core)
cd core/rednode-core && cargo run --release &
cd ../..

# 4. Agents (16 agents)
pnpm install
pnpm agents &

# 5. Web dashboard (13 tabs)
pnpm web &

# 6. CLI (19 commands)
pnpm --filter @rednode/cli dev -- status
pnpm --filter @rednode/cli dev -- intent "check system health"
pnpm --filter @rednode/cli dev -- goodnight

# 7. Voice (optional)
cd interfaces/voice
python stt_server.py &   # port 8081
python tts_server.py &   # port 8082
python voice_loop.py     # wake word → listen → speak

# 8. Signal Bot (optional)
pnpm --filter @rednode/signal-bot dev
```

## Configuration

```bash
cp .env.example .env
vim .env   # Set your Pi-hole, TrueNAS, email, etc. credentials
```

## API

```bash
# Submit intent
curl -X POST http://localhost:8787/intent \
  -H "Content-Type: application/json" \
  -d '{"intent":"analyze system health"}'

# Check sentience
curl http://localhost:8787/sentience | jq .model.drives

# Search memory
curl "http://localhost:8787/memory/query?q=docker" | jq .results
```

## Endpoints

| Method | Path | What |
|---|---|---|
| GET | /health | Node status + uptime |
| POST | /intent | Submit natural language intent |
| GET | /events | WebSocket real-time event stream |
| GET | /sentience | Self-model, drives, goals |
| GET | /agents/status | 16 agents with heartbeat tracking |
| GET | /audit | Hash-chained audit log |
| GET | /approvals | Pending approval queue |
| POST | /approvals/:id/approve | Approve/deny an action |
| GET | /memory/query?q= | RAG semantic search |
| POST | /memory/ingest | Ingest document into memory |
| GET | /security/events | Security event feed |
| POST | /security/events | Log security event |
| GET | /kg/query?q= | Knowledge graph query |
| POST | /kg/entity | Add entity to knowledge graph |

## Build Guides

- **Android APK**: `docs/guides/BUILD-APK.md`
- **Desktop (Windows/Mac/Linux)**: `docs/guides/BUILD-WINDOWS-APP.md`
- **ISO (bare metal)**: `docs/guides/BUILD-ISO.md`
