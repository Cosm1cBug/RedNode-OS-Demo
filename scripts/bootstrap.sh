#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
echo "🧠 RedNode-OS – Bootstrap v0.2.0"
echo ""
echo "==> 1/4 Infra – docker compose"
cd deployment && docker compose up -d nats postgres qdrant ollama loki prometheus grafana otel-collector && cd ..
sleep 3
echo ""
echo "==> 2/4 Models – ollama"
ollama pull qwen2.5:14b-instruct-q4_K_M || true
ollama pull nomic-embed-text || true
echo ""
echo "==> 3/4 Rust Core"
cd core/rednode-core && cargo build && cd ../..
echo ""
echo "==> 4/4 Agents + Web"
pnpm install
echo ""
echo "=== RedNode-OS ready ==="
echo "CNS:        http://localhost:8787"
echo "Web UI:     http://localhost:3000"
echo "Grafana:    http://localhost:3001  admin/rednode"
echo "NATS:       nats://localhost:4222"
echo "Postgres:   postgres://rednode:rednode@localhost:5432/rednode"
echo "Qdrant:     http://localhost:6333"
echo "Ollama:     http://localhost:11434"
echo ""
echo "Run:"
echo "  cd core/rednode-core && cargo run"
echo "  pnpm agents"
echo "  pnpm web"
echo "  pnpm --filter @rednode/cli dev -- intent \"analyze system health\""
