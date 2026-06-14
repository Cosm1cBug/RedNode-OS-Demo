#!/usr/bin/env bash
# RedNode-OS – Start All Services
# Run this after docker compose up -d and NixOS services are running
#
# Usage:
#   ./scripts/start-all.sh          # start everything
#   ./scripts/start-all.sh --stop   # stop everything
#   ./scripts/start-all.sh --status # check status

set -euo pipefail
cd "$(dirname "$0")/.."

ROOT=$(pwd)
LOG_DIR="${REDNODE_LOG_DIR:-/var/lib/rednode/logs}"
PID_DIR="${REDNODE_PID_DIR:-/tmp/rednode-pids}"

mkdir -p "$LOG_DIR" "$PID_DIR"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

info() { echo -e "${GREEN}[RedNode]${NC} $1"; }
warn() { echo -e "${YELLOW}[RedNode]${NC} $1"; }
err()  { echo -e "${RED}[RedNode]${NC} $1"; }

# ─── Service Management ───

start_service() {
  local name=$1
  local cmd=$2
  local dir=$3
  local pid_file="$PID_DIR/${name}.pid"

  if [ -f "$pid_file" ] && kill -0 "$(cat "$pid_file")" 2>/dev/null; then
    warn "$name already running (PID $(cat "$pid_file"))"
    return
  fi

  info "Starting $name..."
  cd "$dir"
  nohup bash -c "$cmd" > "$LOG_DIR/${name}.log" 2>&1 &
  echo $! > "$pid_file"
  cd "$ROOT"
  info "$name started (PID $(cat "$pid_file"), log: $LOG_DIR/${name}.log)"
}

stop_service() {
  local name=$1
  local pid_file="$PID_DIR/${name}.pid"

  if [ -f "$pid_file" ]; then
    local pid=$(cat "$pid_file")
    if kill -0 "$pid" 2>/dev/null; then
      kill "$pid" 2>/dev/null || true
      info "$name stopped (PID $pid)"
    else
      warn "$name was not running"
    fi
    rm -f "$pid_file"
  else
    warn "$name: no PID file"
  fi
}

check_service() {
  local name=$1
  local pid_file="$PID_DIR/${name}.pid"

  if [ -f "$pid_file" ] && kill -0 "$(cat "$pid_file")" 2>/dev/null; then
    echo -e "  ${GREEN}✅${NC} $name (PID $(cat "$pid_file"))"
  else
    echo -e "  ${RED}❌${NC} $name (not running)"
  fi
}

# ─── Commands ───

do_start() {
  echo ""
  info "🧠 RedNode-OS — Starting All Services"
  echo ""

  # 1. Check Docker infra
  info "Checking Docker infrastructure..."
  if docker compose -f deployment/docker-compose.yml ps --format "{{.Name}}" 2>/dev/null | grep -q rednode; then
    info "Docker services running ✅"
  else
    warn "Docker services not running — starting..."
    docker compose -f deployment/docker-compose.yml up -d
    sleep 3
  fi

  # 2. CNS (Rust core)
  start_service "cns" "cargo run --release 2>&1" "core/rednode-core"
  sleep 2  # wait for CNS to bind port

  # 3. Core agents
  start_service "system-agent" "pnpm --filter @rednode/system-agent dev" "$ROOT"
  start_service "security-agent" "pnpm --filter @rednode/security-agent dev" "$ROOT"
  start_service "coding-agent" "pnpm --filter @rednode/coding-agent dev" "$ROOT"
  start_service "research-agent" "pnpm --filter @rednode/research-agent dev" "$ROOT"
  start_service "automation-agent" "pnpm --filter @rednode/automation-agent dev" "$ROOT"
  start_service "network-agent" "pnpm --filter @rednode/network-agent dev" "$ROOT"

  # 4. Infrastructure agents
  start_service "infra-agent" "pnpm --filter @rednode/infra-agent dev" "$ROOT"
  start_service "storage-agent" "pnpm --filter @rednode/storage-agent dev" "$ROOT"
  start_service "surveillance-agent" "pnpm --filter @rednode/surveillance-agent dev" "$ROOT"
  start_service "comms-agent" "pnpm --filter @rednode/comms-agent dev" "$ROOT"

  # 5. Web dashboard
  start_service "web" "pnpm --filter @rednode/web dev" "$ROOT"

  # 6. Voice (optional)
  if [ "${REDNODE_VOICE:-off}" = "on" ]; then
    start_service "stt" "python interfaces/voice/stt_server.py" "$ROOT"
    start_service "tts" "python interfaces/voice/tts_server.py" "$ROOT"
  else
    info "Voice servers disabled (set REDNODE_VOICE=on to enable)"
  fi

  echo ""
  info "🧠 RedNode-OS — All services started"
  info "  CNS:       http://localhost:8787"
  info "  Dashboard: http://localhost:3000"
  info "  Grafana:   http://localhost:3001"
  info "  Frigate:   http://localhost:5000"
  echo ""
}

do_stop() {
  echo ""
  info "🧠 RedNode-OS — Stopping All Services"
  echo ""

  for svc in tts stt web comms-agent surveillance-agent storage-agent infra-agent \
             network-agent automation-agent research-agent coding-agent security-agent system-agent cns; do
    stop_service "$svc"
  done

  echo ""
  info "All services stopped. Docker infra still running (use 'docker compose down' to stop)."
}

do_status() {
  echo ""
  echo "🧠 RedNode-OS — Service Status"
  echo ""

  # Docker
  echo "  Docker:"
  for svc in nats postgres qdrant ollama mosquitto frigate loki prometheus grafana; do
    if docker ps --format "{{.Names}}" 2>/dev/null | grep -q "rednode-${svc}"; then
      echo -e "    ${GREEN}✅${NC} ${svc}"
    else
      echo -e "    ${RED}❌${NC} ${svc}"
    fi
  done

  echo ""
  echo "  RedNode:"
  check_service "cns"
  for agent in system-agent security-agent coding-agent research-agent automation-agent \
               network-agent infra-agent storage-agent surveillance-agent comms-agent; do
    check_service "$agent"
  done
  check_service "web"
  check_service "stt"
  check_service "tts"

  # API check
  echo ""
  if curl -sf http://localhost:8787/health > /dev/null 2>&1; then
    echo -e "  ${GREEN}✅${NC} CNS API responding"
  else
    echo -e "  ${RED}❌${NC} CNS API not responding"
  fi
  echo ""
}

# ─── Main ───

case "${1:-start}" in
  start)  do_start ;;
  stop)   do_stop ;;
  status) do_status ;;
  restart) do_stop; sleep 2; do_start ;;
  *) echo "Usage: $0 {start|stop|status|restart}" ;;
esac
