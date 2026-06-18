#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════
# RedNode-OS — First Boot Setup
# Run this ONCE after installing NixOS and booting for the first time.
#
# What it does:
#   1. Detects your GPU and VRAM
#   2. Selects the best LLM model for your hardware
#   3. Pulls the models via Ollama
#   4. Creates your .env from .env.example
#   5. Starts Docker infrastructure
#   6. Builds RedNode CNS (Rust)
#   7. Installs Node.js dependencies
#   8. Starts everything
#   9. Verifies the system is working
#
# Usage:
#   cd ~/RedNode-OS-Demo && ./scripts/setup-first-boot.sh
# ═══════════════════════════════════════════════════════════

set -euo pipefail
cd "$(dirname "$0")/.."
ROOT=$(pwd)

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
BOLD='\033[1m'
NC='\033[0m'

info()  { echo -e "${GREEN}[✓]${NC} $1"; }
warn()  { echo -e "${YELLOW}[!]${NC} $1"; }
err()   { echo -e "${RED}[✗]${NC} $1"; }
step()  { echo -e "\n${BLUE}${BOLD}═══ $1 ═══${NC}\n"; }

echo ""
echo -e "${BOLD}  ╔══════════════════════════════════════════╗${NC}"
echo -e "${BOLD}  ║  🧠 RedNode-OS v0.4.0 — First Boot Setup ║${NC}"
echo -e "${BOLD}  ╚══════════════════════════════════════════╝${NC}"
echo ""

# ═══════════════════════════════════════════
step "Step 1/9: GPU Detection"
# ═══════════════════════════════════════════

GPU_NAME="none"
GPU_VRAM_MB=0
SELECTED_MODEL=""

if command -v nvidia-smi >/dev/null 2>&1; then
  GPU_INFO=$(nvidia-smi --query-gpu=name,memory.total --format=csv,noheader,nounits 2>/dev/null || echo "")
  if [ -n "$GPU_INFO" ]; then
    GPU_NAME=$(echo "$GPU_INFO" | cut -d',' -f1 | xargs)
    GPU_VRAM_MB=$(echo "$GPU_INFO" | cut -d',' -f2 | xargs)
    info "GPU detected: ${BOLD}$GPU_NAME${NC} (${GPU_VRAM_MB} MB VRAM)"
  else
    warn "nvidia-smi found but no GPU detected"
  fi
else
  warn "nvidia-smi not found — checking for AMD..."
  if command -v rocm-smi >/dev/null 2>&1; then
    GPU_NAME=$(rocm-smi --showproductname 2>/dev/null | grep "GPU" | head -1 | xargs || echo "AMD GPU")
    GPU_VRAM_MB=$(rocm-smi --showmeminfo vram 2>/dev/null | grep "Total" | awk '{print $NF}' | head -1 || echo "0")
    GPU_VRAM_MB=$((GPU_VRAM_MB / 1024 / 1024))
    if [ "$GPU_VRAM_MB" -gt 0 ]; then
      info "AMD GPU detected: $GPU_NAME (${GPU_VRAM_MB} MB VRAM)"
    fi
  fi
fi

if [ "$GPU_VRAM_MB" -eq 0 ]; then
  warn "No GPU detected — will use CPU-only mode (slower)"
  GPU_VRAM_MB=0
fi

# ═══════════════════════════════════════════
step "Step 2/9: Model Selection"
# ═══════════════════════════════════════════

WHISPER_MODEL="base"

if [ "$GPU_VRAM_MB" -ge 24000 ]; then
  SELECTED_MODEL="qwen2.5:32b-instruct-q4_K_M"
  WHISPER_MODEL="large-v3"
  info "32B model selected (${GPU_VRAM_MB}MB VRAM — premium tier)"
  info "Whisper: large-v3 (best accuracy)"
elif [ "$GPU_VRAM_MB" -ge 12000 ]; then
  SELECTED_MODEL="qwen2.5:14b-instruct-q4_K_M"
  WHISPER_MODEL="small"
  info "14B model selected (${GPU_VRAM_MB}MB VRAM — recommended tier)"
  info "Whisper: small (good accuracy, fits with 14B)"
elif [ "$GPU_VRAM_MB" -ge 6000 ]; then
  SELECTED_MODEL="qwen2.5:7b-instruct-q4_K_M"
  WHISPER_MODEL="small"
  info "7B model selected (${GPU_VRAM_MB}MB VRAM — starter tier)"
  info "Whisper: small"
elif [ "$GPU_VRAM_MB" -ge 3000 ]; then
  SELECTED_MODEL="qwen2.5:3b-instruct-q4_K_M"
  WHISPER_MODEL="base"
  info "3B model selected (${GPU_VRAM_MB}MB VRAM — minimal tier)"
  info "Whisper: base (lightweight)"
else
  SELECTED_MODEL="qwen2.5:3b-instruct-q4_K_M"
  WHISPER_MODEL="base"
  warn "CPU-only mode: 3B model (will be slow but functional)"
  info "Whisper: base"
fi

echo ""
echo -e "  ${BOLD}GPU:${NC}     $GPU_NAME"
echo -e "  ${BOLD}VRAM:${NC}    ${GPU_VRAM_MB} MB"
echo -e "  ${BOLD}LLM:${NC}     $SELECTED_MODEL"
echo -e "  ${BOLD}Whisper:${NC}  $WHISPER_MODEL"
echo -e "  ${BOLD}Embed:${NC}   nomic-embed-text"
echo ""

read -p "  Use these settings? [Y/n] " -n 1 -r
echo ""
if [[ $REPLY =~ ^[Nn]$ ]]; then
  read -p "  Enter LLM model name (e.g., qwen2.5:7b-instruct-q4_K_M): " SELECTED_MODEL
  read -p "  Enter Whisper model (tiny/base/small/medium/large-v3): " WHISPER_MODEL
fi

# ═══════════════════════════════════════════
step "Step 3/9: Pulling AI Models"
# ═══════════════════════════════════════════

info "Pulling LLM: $SELECTED_MODEL (this may take 5-20 minutes)..."
ollama pull "$SELECTED_MODEL" || {
  err "Failed to pull $SELECTED_MODEL"
  warn "Make sure Ollama is running: systemctl status ollama"
  warn "Retrying in 5 seconds..."
  sleep 5
  ollama pull "$SELECTED_MODEL"
}
info "LLM model ready ✅"

info "Pulling embedding model: nomic-embed-text..."
ollama pull nomic-embed-text
info "Embedding model ready ✅"

# Verify
info "Models installed:"
ollama list

# ═══════════════════════════════════════════
step "Step 4/9: Creating .env Configuration"
# ═══════════════════════════════════════════

if [ -f .env ]; then
  warn ".env already exists — backing up to .env.backup"
  cp .env .env.backup
fi

cp .env.example .env

# Auto-fill detected values
sed -i "s|^REDNODE_MODEL=.*|REDNODE_MODEL=$SELECTED_MODEL|" .env
sed -i "s|^WHISPER_MODEL=.*|WHISPER_MODEL=$WHISPER_MODEL|" .env

# Generate API token
API_TOKEN="rn_$(openssl rand -hex 32 2>/dev/null || python3 -c 'import secrets; print(secrets.token_hex(32))')"
sed -i "s|^REDNODE_API_TOKEN=.*|REDNODE_API_TOKEN=$API_TOKEN|" .env

# Detect hostname
HOSTNAME=$(hostname)
sed -i "s|^REDNODE_HOSTNAME=.*|REDNODE_HOSTNAME=$HOSTNAME|" .env
sed -i "s|^REDNODE_NODE_ID=.*|REDNODE_NODE_ID=$HOSTNAME|" .env

# Detect IP
IP=$(hostname -I 2>/dev/null | awk '{print $1}' || echo "localhost")
sed -i "s|^REDNODE_CNS=.*|REDNODE_CNS=http://$IP:8787|" .env
sed -i "s|^REDNODE_URL=.*|REDNODE_URL=http://$IP:8787|" .env

info ".env created with auto-detected values:"
echo "  Model: $SELECTED_MODEL"
echo "  Hostname: $HOSTNAME"
echo "  IP: $IP"
echo "  API Token: ${API_TOKEN:0:12}..."
echo ""
warn "Edit .env later to add Pi-hole, TrueNAS, email credentials:"
echo "  nano .env"

# ═══════════════════════════════════════════
step "Step 5/9: Starting Docker Infrastructure"
# ═══════════════════════════════════════════

info "Starting NATS, PostgreSQL, Qdrant, Mosquitto, Grafana, SearXNG..."
cd deployment
docker compose up -d nats postgres qdrant mosquitto loki prometheus grafana otel-collector searxng
cd "$ROOT"

info "Waiting for services to initialize..."
sleep 8

# Verify
for svc in nats postgres qdrant; do
  if docker ps --format '{{.Names}}' | grep -q "rednode-${svc}"; then
    info "$svc running ✅"
  else
    err "$svc NOT running ❌"
  fi
done

# ═══════════════════════════════════════════
step "Step 6/9: Building RedNode CNS (Rust)"
# ═══════════════════════════════════════════

info "Building RedNode CNS (first build takes 3-5 minutes)..."
cd core/rednode-core
cargo build --release 2>&1 | tail -5
cd "$ROOT"
info "CNS built ✅"

# ═══════════════════════════════════════════
step "Step 7/9: Installing Node.js Dependencies"
# ═══════════════════════════════════════════

info "Installing pnpm dependencies..."
pnpm install 2>&1 | tail -3
info "Dependencies installed ✅"

# ═══════════════════════════════════════════
step "Step 8/9: Starting Everything"
# ═══════════════════════════════════════════

info "Starting RedNode-OS..."
./scripts/start-all.sh start

# ═══════════════════════════════════════════
step "Step 9/9: Verification"
# ═══════════════════════════════════════════

sleep 5

# Test CNS
if curl -sf http://localhost:8787/health >/dev/null 2>&1; then
  info "CNS API responding ✅"
  HEALTH=$(curl -sf http://localhost:8787/health)
  echo "  $HEALTH"
else
  err "CNS API not responding — check: cargo run --release"
fi

# Test LLM
info "Testing LLM planner..."
RESULT=$(curl -sf -X POST http://localhost:8787/intent \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer $API_TOKEN" \
  -d '{"intent":"show system health"}' 2>/dev/null || echo '{"ok":false}')

if echo "$RESULT" | python3 -c "import sys,json; d=json.load(sys.stdin); sys.exit(0 if d.get('ok') else 1)" 2>/dev/null; then
  info "LLM planner working ✅"
  echo "  Plan steps: $(echo "$RESULT" | python3 -c "import sys,json; print(len(json.load(sys.stdin).get('plan',[])))" 2>/dev/null)"
else
  warn "LLM planner test failed — Ollama may still be loading the model"
  echo "  Wait 30 seconds and try: curl -X POST http://localhost:8787/intent -H 'Content-Type: application/json' -d '{\"intent\":\"health check\"}'"
fi

# Test Sentience
if curl -sf http://localhost:8787/sentience >/dev/null 2>&1; then
  info "Sentience Engine running ✅"
else
  warn "Sentience Engine not responding yet"
fi

echo ""
echo -e "${BOLD}═══════════════════════════════════════════════════${NC}"
echo -e "${BOLD}  🧠 RedNode-OS v0.4.0 — Setup Complete${NC}"
echo -e "${BOLD}═══════════════════════════════════════════════════${NC}"
echo ""
echo -e "  ${BOLD}Dashboard:${NC}    http://$IP:3000"
echo -e "  ${BOLD}CNS API:${NC}      http://$IP:8787"
echo -e "  ${BOLD}Grafana:${NC}      http://$IP:3001  (admin/rednode)"
echo -e "  ${BOLD}API Token:${NC}    ${API_TOKEN:0:20}..."
echo ""
echo -e "  ${BOLD}GPU:${NC}          $GPU_NAME ($GPU_VRAM_MB MB)"
echo -e "  ${BOLD}LLM Model:${NC}    $SELECTED_MODEL"
echo -e "  ${BOLD}Whisper:${NC}      $WHISPER_MODEL"
echo ""
echo -e "  ${BOLD}Next steps:${NC}"
echo "    1. Open dashboard: http://$IP:3000"
echo "    2. Edit .env to add Pi-hole, TrueNAS, email: nano .env"
echo "    3. Set up pfSense + VLANs: cat docs/NETWORK-ARCHITECTURE.md"
echo "    4. Configure cameras: nano deployment/frigate.yml"
echo "    5. Deploy endpoint agents on other PCs: cat scripts/endpoint-install-linux.sh"
echo ""
echo -e "  ${BOLD}Commands:${NC}"
echo "    ./scripts/start-all.sh status    — check all services"
echo "    ./scripts/start-all.sh restart   — restart everything"
echo "    rednode status                    — full system overview"
echo "    rednode intent \"your intent\"      — talk to RedNode"
echo ""
echo -e "  ${GREEN}${BOLD}The computer becomes the intelligence.${NC}"
echo ""
