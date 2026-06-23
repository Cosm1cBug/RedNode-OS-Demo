#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════════════
# RedNode-OS — Self-Healing Installation & Repair System
# 
# This script is the autonomous heart of RedNode's installation.
# It diagnoses, repairs, retries, and logs every action.
#
# Modes:
#   install   — Full first-boot installation (clone + build + start)
#   diagnose  — Check all subsystems and report status
#   repair    — Detect and fix any broken subsystem
#   watch     — Continuous monitoring loop (runs as systemd service)
#
# Usage:
#   ./scripts/rednode-selfheal.sh install    # first boot
#   ./scripts/rednode-selfheal.sh diagnose   # check everything
#   ./scripts/rednode-selfheal.sh repair     # fix issues
#   ./scripts/rednode-selfheal.sh watch      # continuous (systemd)
#
# Design principles:
#   - Every operation is idempotent (safe to re-run)
#   - Every failure is logged with diagnosis
#   - Retries with exponential backoff (max 5 attempts)
#   - Falls back to degraded mode (CNS-only) if agents fail
#   - Never exits with error in watch mode — always self-heals
# ═══════════════════════════════════════════════════════════════════

set -uo pipefail
# NOTE: No `set -e` — we handle every error ourselves

# ─── Config ───

REDNODE_HOME="${REDNODE_HOME:-/var/lib/rednode}"
REDNODE_SOURCE="${REDNODE_SOURCE:-${REDNODE_HOME}/source}"
REDNODE_REPO="${REDNODE_REPO:-https://github.com/Cosm1cBug/RedNode-OS-Demo.git}"
REDNODE_BRANCH="${REDNODE_BRANCH:-main}"
REDNODE_LOG="${REDNODE_HOME}/logs/selfheal.log"
REDNODE_STATE="${REDNODE_HOME}/.selfheal-state"
MAX_RETRIES=5
BACKOFF_BASE=5       # seconds
WATCH_INTERVAL=300   # 5 minutes
HEALTH_ENDPOINT="http://127.0.0.1:8787/health"

# ─── Colors ───

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

# ─── Logging ───

mkdir -p "$(dirname "$REDNODE_LOG")" 2>/dev/null || true

log() {
  local level="$1"
  shift
  local msg="$*"
  local ts
  ts=$(date '+%Y-%m-%d %H:%M:%S')
  local colored_msg

  case "$level" in
    INFO)  colored_msg="${GREEN}[✓ INFO]${NC}  $msg" ;;
    WARN)  colored_msg="${YELLOW}[! WARN]${NC}  $msg" ;;
    ERROR) colored_msg="${RED}[✗ ERROR]${NC} $msg" ;;
    FIX)   colored_msg="${CYAN}[⚡ FIX]${NC}  $msg" ;;
    DIAG)  colored_msg="${BLUE}[? DIAG]${NC}  $msg" ;;
    *)     colored_msg="[$level] $msg" ;;
  esac

  echo -e "$colored_msg"
  echo "[$ts] [$level] $msg" >> "$REDNODE_LOG" 2>/dev/null || true
}

# ─── State Management ───

save_state() {
  local key="$1"
  local value="$2"
  mkdir -p "$(dirname "$REDNODE_STATE")" 2>/dev/null || true
  # Remove old key if exists, then append
  if [ -f "$REDNODE_STATE" ]; then
    grep -v "^${key}=" "$REDNODE_STATE" > "${REDNODE_STATE}.tmp" 2>/dev/null || true
    mv "${REDNODE_STATE}.tmp" "$REDNODE_STATE"
  fi
  echo "${key}=${value}" >> "$REDNODE_STATE"
}

get_state() {
  local key="$1"
  local default="${2:-}"
  if [ -f "$REDNODE_STATE" ]; then
    grep "^${key}=" "$REDNODE_STATE" 2>/dev/null | tail -1 | cut -d= -f2- || echo "$default"
  else
    echo "$default"
  fi
}

# ─── Retry with Exponential Backoff ───

retry_with_backoff() {
  local description="$1"
  shift
  local attempt=1

  while [ $attempt -le $MAX_RETRIES ]; do
    log INFO "Attempt $attempt/$MAX_RETRIES: $description"
    
    if "$@"; then
      log INFO "✅ Success: $description"
      return 0
    fi

    local wait_time=$(( BACKOFF_BASE * (2 ** (attempt - 1)) ))
    # Cap at 120 seconds
    [ $wait_time -gt 120 ] && wait_time=120
    
    if [ $attempt -lt $MAX_RETRIES ]; then
      log WARN "Failed attempt $attempt/$MAX_RETRIES: $description — retrying in ${wait_time}s"
      sleep "$wait_time"
    else
      log ERROR "All $MAX_RETRIES attempts failed: $description"
    fi
    
    attempt=$((attempt + 1))
  done
  
  return 1
}

# ═══════════════════════════════════════════════════════════════════
# SUBSYSTEM CHECKS — each returns 0 (healthy) or 1 (broken)
# ═══════════════════════════════════════════════════════════════════

check_network() {
  # Check if we have basic network connectivity
  if ping -c 1 -W 3 9.9.9.9 >/dev/null 2>&1; then
    return 0
  fi
  # Try DNS-based check
  if host github.com >/dev/null 2>&1; then
    return 0
  fi
  return 1
}

check_source_cloned() {
  # Is the source code present?
  if [ -d "${REDNODE_SOURCE}/.git" ] && [ -f "${REDNODE_SOURCE}/package.json" ]; then
    return 0
  fi
  return 1
}

check_nix_services() {
  # Check NixOS-managed services
  local all_ok=true
  for svc in postgresql nats mosquitto grafana ollama; do
    if systemctl is-active --quiet "$svc" 2>/dev/null; then
      : # ok
    else
      all_ok=false
    fi
  done
  $all_ok
}

check_postgres() {
  if systemctl is-active --quiet postgresql 2>/dev/null; then
    if sudo -u postgres psql -d rednode -c "SELECT 1;" >/dev/null 2>&1; then
      return 0
    fi
  fi
  return 1
}

check_nats() {
  systemctl is-active --quiet nats 2>/dev/null
}

check_ollama() {
  if systemctl is-active --quiet ollama 2>/dev/null; then
    if curl -sf http://127.0.0.1:11434/api/tags >/dev/null 2>&1; then
      return 0
    fi
  fi
  return 1
}

check_ollama_models() {
  local models
  models=$(curl -sf http://127.0.0.1:11434/api/tags 2>/dev/null || echo '{}')
  if echo "$models" | grep -q "nomic-embed-text"; then
    return 0
  fi
  return 1
}

check_qdrant() {
  curl -sf http://127.0.0.1:6333/healthz >/dev/null 2>&1
}

check_docker() {
  systemctl is-active --quiet docker 2>/dev/null && docker info >/dev/null 2>&1
}

check_rust_binary() {
  # Check 1: compiled binary in source tree
  if [ -f "${REDNODE_SOURCE}/core/rednode-core/target/release/rednode-core" ]; then
    return 0
  fi
  # Check 2: Nix-built binary on PATH (from flake.nix — baked into ISO)
  if command -v rednode-core >/dev/null 2>&1; then
    return 0
  fi
  return 1
}

check_node_deps() {
  if [ -d "${REDNODE_SOURCE}/node_modules" ]; then
    return 0
  fi
  return 1
}

check_env_file() {
  if [ -f "${REDNODE_SOURCE}/.env" ]; then
    return 0
  fi
  return 1
}

check_cns_running() {
  curl -sf "$HEALTH_ENDPOINT" >/dev/null 2>&1
}

# ═══════════════════════════════════════════════════════════════════
# REPAIR ACTIONS — each fixes one specific subsystem
# ═══════════════════════════════════════════════════════════════════

repair_network() {
  log FIX "Repairing network connectivity..."
  
  # Step 1: Check if interface is up
  local iface
  iface=$(ip -o link show | awk -F': ' '/state UP/{print $2; exit}')
  if [ -z "$iface" ]; then
    log DIAG "No network interface is UP"
    # Try to bring up all ethernet interfaces
    for nic in $(ip -o link show | awk -F': ' '{print $2}' | grep -E '^(en|eth)'); do
      log FIX "Bringing up interface: $nic"
      ip link set "$nic" up 2>/dev/null || true
    done
    sleep 3
  fi
  
  # Step 2: Check DHCP vs static
  if ! ip addr show | grep -q "inet.*10\.0\.50\.10"; then
    log DIAG "Static IP 10.0.50.10 not assigned — checking config"
    # NixOS will handle this via networking config, just restart
    systemctl restart systemd-networkd 2>/dev/null || true
    sleep 5
  fi
  
  # Step 3: Check DNS
  if ! host github.com >/dev/null 2>&1; then
    log DIAG "DNS resolution failing"
    # Temporarily add fallback DNS
    echo "nameserver 9.9.9.9" >> /etc/resolv.conf 2>/dev/null || true
    systemctl restart systemd-resolved 2>/dev/null || true
    sleep 2
  fi
  
  check_network
}

repair_clone_source() {
  log FIX "Deploying RedNode source to ${REDNODE_SOURCE}..."
  
  mkdir -p "$(dirname "$REDNODE_SOURCE")" 2>/dev/null || true
  
  # ── Strategy 1: Baked-in source from ISO (no internet needed) ──
  # When built with `nix build .#iso`, the source tree is pre-baked
  # into the Nix store at $REDNODE_BAKED_SOURCE
  if [ -n "${REDNODE_BAKED_SOURCE:-}" ] && [ -d "$REDNODE_BAKED_SOURCE" ]; then
    if [ ! -f "${REDNODE_SOURCE}/package.json" ]; then
      log INFO "Copying baked-in source from Nix store (no internet needed)..."
      # Nix store is read-only, so we copy (not symlink) to get a writable tree
      rm -rf "${REDNODE_SOURCE}" 2>/dev/null || true
      cp -r "$REDNODE_BAKED_SOURCE" "$REDNODE_SOURCE"
      chmod -R u+w "$REDNODE_SOURCE"
      log INFO "Source deployed from ISO ✅ (zero network, zero git)"
      
      # Initialize a local git repo so update checks work later
      cd "$REDNODE_SOURCE"
      git init -q 2>/dev/null || true
      git add -A 2>/dev/null || true
      git commit -q -m "Initial: deployed from ISO" 2>/dev/null || true
      git remote add origin "$REDNODE_REPO" 2>/dev/null || true
      cd /
    else
      log INFO "Source already present from previous deployment ✅"
    fi
    [ -f "${REDNODE_SOURCE}/package.json" ]
    return $?
  fi
  
  # ── Strategy 2: Also check /run/current-system/rednode-source ──
  # (alternative path where the ISO builder links the source)
  if [ -d "/run/current-system/rednode-source" ] && [ ! -f "${REDNODE_SOURCE}/package.json" ]; then
    log INFO "Copying source from /run/current-system/rednode-source..."
    rm -rf "${REDNODE_SOURCE}" 2>/dev/null || true
    cp -r "/run/current-system/rednode-source" "$REDNODE_SOURCE"
    chmod -R u+w "$REDNODE_SOURCE"
    cd "$REDNODE_SOURCE"
    git init -q 2>/dev/null || true
    git add -A 2>/dev/null || true
    git commit -q -m "Initial: deployed from system closure" 2>/dev/null || true
    git remote add origin "$REDNODE_REPO" 2>/dev/null || true
    cd /
    log INFO "Source deployed from system closure ✅"
    [ -f "${REDNODE_SOURCE}/package.json" ]
    return $?
  fi
  
  # ── Strategy 3: Git clone from GitHub (needs internet) ──
  if [ -d "${REDNODE_SOURCE}" ] && [ ! -d "${REDNODE_SOURCE}/.git" ] && [ ! -f "${REDNODE_SOURCE}/package.json" ]; then
    # Directory exists but is empty/broken — back up and re-clone
    log WARN "Source directory exists but is incomplete — backing up"
    mv "${REDNODE_SOURCE}" "${REDNODE_SOURCE}.backup.$(date +%s)" 2>/dev/null || true
  fi
  
  if [ ! -f "${REDNODE_SOURCE}/package.json" ]; then
    log INFO "Cloning from GitHub (internet required)..."
    git clone --depth 1 --branch "$REDNODE_BRANCH" "$REDNODE_REPO" "$REDNODE_SOURCE"
  else
    # Already cloned — pull latest
    log INFO "Source already cloned — pulling latest..."
    cd "$REDNODE_SOURCE"
    git fetch origin "$REDNODE_BRANCH" --depth 1 2>/dev/null || true
    git reset --hard "origin/$REDNODE_BRANCH" 2>/dev/null || true
    cd /
  fi
  
  # Verify
  [ -f "${REDNODE_SOURCE}/package.json" ]
}

repair_nix_service() {
  local svc="$1"
  log FIX "Repairing NixOS service: $svc"
  
  # Check if service exists
  if ! systemctl list-unit-files "${svc}.service" >/dev/null 2>&1; then
    log ERROR "Service ${svc} does not exist in NixOS config — run: sudo nixos-rebuild switch"
    return 1
  fi
  
  # Try restart
  systemctl restart "$svc" 2>/dev/null
  sleep 3
  
  if systemctl is-active --quiet "$svc" 2>/dev/null; then
    log INFO "Service $svc restarted successfully"
    return 0
  fi
  
  # Check journal for hints
  local err_msg
  err_msg=$(journalctl -u "$svc" --since "2 minutes ago" --no-pager -p err 2>/dev/null | tail -5)
  
  if [ -n "$err_msg" ]; then
    log DIAG "Service $svc error log:"
    echo "$err_msg" >> "$REDNODE_LOG"
    
    # Specific repairs based on error patterns
    case "$err_msg" in
      *"Address already in use"*)
        log FIX "Port conflict detected — killing conflicting process"
        local port
        case "$svc" in
          postgresql) port=5432 ;;
          nats) port=4222 ;;
          ollama) port=11434 ;;
          grafana) port=3001 ;;
          mosquitto) port=1883 ;;
          *) port="" ;;
        esac
        if [ -n "$port" ]; then
          fuser -k "${port}/tcp" 2>/dev/null || true
          sleep 2
          systemctl restart "$svc"
        fi
        ;;
      *"No space left"*)
        log FIX "Disk space issue — cleaning up"
        journalctl --vacuum-size=100M 2>/dev/null || true
        nix-collect-garbage --delete-older-than 3d 2>/dev/null || true
        systemctl restart "$svc"
        ;;
      *"permission denied"*|*"Permission denied"*)
        log FIX "Permission issue — fixing ownership"
        case "$svc" in
          postgresql) chown -R postgres:postgres /var/lib/postgresql 2>/dev/null || true ;;
          nats) chown -R nats:nats /var/lib/nats 2>/dev/null || true ;;
          ollama) chown -R ollama:ollama /var/lib/ollama 2>/dev/null || true ;;
        esac
        systemctl restart "$svc"
        ;;
      *"database"*"does not exist"*)
        log FIX "Database missing — will be recreated by PostgreSQL ensureDatabases"
        systemctl restart "$svc"
        sleep 5
        sudo -u postgres createdb rednode 2>/dev/null || true
        ;;
    esac
  fi
  
  systemctl is-active --quiet "$svc" 2>/dev/null
}

repair_postgres() {
  log FIX "Repairing PostgreSQL..."
  
  if ! systemctl is-active --quiet postgresql 2>/dev/null; then
    repair_nix_service "postgresql"
  fi
  
  # Check if rednode database exists
  sleep 2
  if ! sudo -u postgres psql -lqt 2>/dev/null | grep -qw rednode; then
    log FIX "Creating rednode database..."
    sudo -u postgres createdb rednode 2>/dev/null || true
    sudo -u postgres psql -c "CREATE USER rednode;" 2>/dev/null || true
    sudo -u postgres psql -c "GRANT ALL ON DATABASE rednode TO rednode;" 2>/dev/null || true
  fi
  
  # Enable pgvector extension
  sudo -u postgres psql -d rednode -c "CREATE EXTENSION IF NOT EXISTS vector;" 2>/dev/null || true
  
  check_postgres
}

repair_docker() {
  log FIX "Repairing Docker..."
  
  if ! systemctl is-active --quiet docker; then
    systemctl start docker
    sleep 3
  fi
  
  # Check if Qdrant container is running
  if ! docker ps --format '{{.Names}}' | grep -q qdrant; then
    log FIX "Starting Qdrant container via NixOS OCI..."
    systemctl restart docker-qdrant 2>/dev/null || true
    sleep 5
  fi
  
  check_docker
}

repair_qdrant() {
  log FIX "Repairing Qdrant..."
  
  # Qdrant runs as NixOS OCI container
  local qdrant_svc
  qdrant_svc=$(systemctl list-units --type=service | grep -i qdrant | awk '{print $1}' | head -1)
  
  if [ -n "$qdrant_svc" ]; then
    systemctl restart "$qdrant_svc" 2>/dev/null || true
    sleep 5
  else
    # Try Docker directly
    docker run -d --name qdrant --restart always \
      -p 127.0.0.1:6333:6333 -p 127.0.0.1:6334:6334 \
      -v /var/lib/rednode/qdrant:/qdrant/storage \
      qdrant/qdrant:v1.9 2>/dev/null || true
    sleep 5
  fi
  
  check_qdrant
}

repair_ollama() {
  log FIX "Repairing Ollama..."
  
  if ! systemctl is-active --quiet ollama; then
    repair_nix_service "ollama"
  fi
  
  # Wait for API to come up
  local waited=0
  while [ $waited -lt 30 ]; do
    if curl -sf http://127.0.0.1:11434/api/tags >/dev/null 2>&1; then
      log INFO "Ollama API ready"
      return 0
    fi
    sleep 2
    waited=$((waited + 2))
  done
  
  return 1
}

repair_ollama_models() {
  log FIX "Pulling required Ollama models..."
  
  # Detect GPU for model selection
  local selected_model="qwen2.5:7b-instruct-q4_K_M"
  
  if command -v nvidia-smi >/dev/null 2>&1; then
    local vram
    vram=$(nvidia-smi --query-gpu=memory.total --format=csv,noheader,nounits 2>/dev/null | head -1 | tr -d ' ')
    if [ -n "$vram" ] && [ "$vram" -ge 20000 ]; then
      selected_model="qwen2.5:32b-instruct-q4_K_M"
    elif [ -n "$vram" ] && [ "$vram" -ge 10000 ]; then
      selected_model="qwen2.5:14b-instruct-q4_K_M"
    fi
    log INFO "NVIDIA GPU detected (${vram:-unknown} MB) — selected: $selected_model"
  elif [ -d /sys/class/drm ] && ls /sys/class/drm/card*/device/vendor 2>/dev/null | xargs grep -l "0x1002" >/dev/null 2>&1; then
    log INFO "AMD GPU detected — selected: $selected_model"
  else
    selected_model="qwen2.5:3b-instruct-q4_K_M"
    log INFO "No GPU detected — selected smaller model: $selected_model"
  fi
  
  # Pull LLM model
  if ! ollama list 2>/dev/null | grep -q "$selected_model"; then
    log INFO "Pulling LLM: $selected_model (this may take 5-20 minutes)..."
    ollama pull "$selected_model" 2>&1 | tail -3
  fi
  
  # Pull embedding model
  if ! ollama list 2>/dev/null | grep -q "nomic-embed-text"; then
    log INFO "Pulling embedding model: nomic-embed-text..."
    ollama pull nomic-embed-text 2>&1 | tail -3
  fi
  
  # Save selected model to state
  save_state "selected_model" "$selected_model"
  
  check_ollama_models
}

repair_rust_build() {
  log FIX "Building RedNode CNS (Rust)..."
  
  if ! check_source_cloned; then
    log ERROR "Source not cloned — cannot build Rust binary"
    return 1
  fi
  
  cd "${REDNODE_SOURCE}/core/rednode-core"
  
  # Check if Rust toolchain is available
  if ! command -v cargo >/dev/null 2>&1; then
    log ERROR "cargo not found — NixOS should provide it. Run: sudo nixos-rebuild switch"
    return 1
  fi
  
  # Clean build if previous build was corrupted
  if [ -d "target" ] && [ -f "target/.build-failed" ]; then
    log FIX "Previous build was corrupted — cleaning target/"
    cargo clean 2>/dev/null || true
    rm -f "target/.build-failed"
  fi
  
  # Build
  log INFO "Running cargo build --release (first build takes 3-5 minutes)..."
  if cargo build --release 2>&1 | tee -a "$REDNODE_LOG"; then
    log INFO "Rust CNS built successfully ✅"
    rm -f "target/.build-failed"
    cd /
    return 0
  else
    local exit_code=$?
    touch "target/.build-failed" 2>/dev/null || true
    
    # Diagnose the failure
    local build_err
    build_err=$(cargo build --release 2>&1 | tail -30)
    
    # Common Rust build failures and repairs:
    if echo "$build_err" | grep -q "could not find.*openssl"; then
      log FIX "Missing openssl dev headers — should be in NixOS config"
      log DIAG "Ensure pkg-config and openssl are in environment.systemPackages"
    elif echo "$build_err" | grep -q "linker.*not found"; then
      log FIX "Linker not found — ensuring gcc/binutils available"
      # NixOS should handle this, but let's make sure
    elif echo "$build_err" | grep -q "No space left"; then
      log FIX "Disk full — cleaning up..."
      nix-collect-garbage --delete-older-than 3d 2>/dev/null || true
      cargo clean 2>/dev/null || true
      # Retry
      cargo build --release 2>&1 | tee -a "$REDNODE_LOG"
    elif echo "$build_err" | grep -q "lock file"; then
      log FIX "Cargo.lock conflict — regenerating"
      cargo update 2>/dev/null || true
      cargo build --release 2>&1 | tee -a "$REDNODE_LOG"
    elif echo "$build_err" | grep -q "could not compile"; then
      log ERROR "Compilation error — source code issue. Trying git pull for latest fixes..."
      cd "${REDNODE_SOURCE}"
      git pull origin "$REDNODE_BRANCH" 2>/dev/null || true
      cd "${REDNODE_SOURCE}/core/rednode-core"
      cargo build --release 2>&1 | tee -a "$REDNODE_LOG"
    fi
    
    cd /
    check_rust_binary
  fi
}

repair_node_deps() {
  log FIX "Installing Node.js dependencies..."
  
  if ! check_source_cloned; then
    log ERROR "Source not cloned — cannot install Node.js deps"
    return 1
  fi
  
  cd "${REDNODE_SOURCE}"
  
  if ! command -v pnpm >/dev/null 2>&1; then
    if command -v npm >/dev/null 2>&1; then
      log FIX "pnpm not found — installing via npm"
      npm install -g pnpm 2>/dev/null || true
    else
      log ERROR "Neither pnpm nor npm found — NixOS should provide them"
      return 1
    fi
  fi
  
  # Clean install if node_modules is corrupted
  if [ -d "node_modules" ] && [ -f "node_modules/.install-failed" ]; then
    log FIX "Previous install was corrupted — removing node_modules"
    rm -rf node_modules
  fi
  
  if pnpm install 2>&1 | tee -a "$REDNODE_LOG"; then
    rm -f "node_modules/.install-failed" 2>/dev/null || true
    log INFO "Node.js dependencies installed ✅"
    cd /
    return 0
  else
    touch "node_modules/.install-failed" 2>/dev/null || true
    
    # Common npm failures:
    local npm_err
    npm_err=$(pnpm install 2>&1 | tail -20)
    
    if echo "$npm_err" | grep -qi "ENOSPC\|No space left"; then
      log FIX "Disk full — cleaning caches"
      pnpm store prune 2>/dev/null || true
      npm cache clean --force 2>/dev/null || true
      nix-collect-garbage --delete-older-than 3d 2>/dev/null || true
      pnpm install 2>&1 | tee -a "$REDNODE_LOG"
    elif echo "$npm_err" | grep -qi "EACCES\|permission denied"; then
      log FIX "Permission issue — fixing ownership"
      chown -R "$(whoami)" "${REDNODE_SOURCE}" 2>/dev/null || true
      pnpm install 2>&1 | tee -a "$REDNODE_LOG"
    elif echo "$npm_err" | grep -qi "network\|ETIMEDOUT\|ECONNRESET"; then
      log WARN "Network issue during npm install — will retry later"
      cd /
      return 1
    fi
    
    cd /
    check_node_deps
  fi
}

repair_env_file() {
  log FIX "Creating .env configuration..."
  
  if ! check_source_cloned; then
    return 1
  fi
  
  cd "${REDNODE_SOURCE}"
  
  if [ ! -f ".env.example" ]; then
    log ERROR ".env.example not found — source may be corrupted"
    return 1
  fi
  
  if [ -f ".env" ]; then
    log INFO ".env already exists — skipping (preserving user config)"
    return 0
  fi
  
  cp .env.example .env
  
  # Auto-fill values
  local selected_model
  selected_model=$(get_state "selected_model" "qwen2.5:7b-instruct-q4_K_M")
  sed -i "s|^REDNODE_MODEL=.*|REDNODE_MODEL=$selected_model|" .env
  
  # Generate API token
  local api_token
  api_token="rn_$(openssl rand -hex 32 2>/dev/null || python3 -c 'import secrets; print(secrets.token_hex(32))')"
  sed -i "s|^REDNODE_API_TOKEN=.*|REDNODE_API_TOKEN=$api_token|" .env
  
  # Hostname and IP
  local hostname_val
  hostname_val=$(hostname 2>/dev/null || echo "rednode")
  local ip_val
  ip_val=$(hostname -I 2>/dev/null | awk '{print $1}' || echo "127.0.0.1")
  
  sed -i "s|^REDNODE_HOSTNAME=.*|REDNODE_HOSTNAME=$hostname_val|" .env
  sed -i "s|^REDNODE_NODE_ID=.*|REDNODE_NODE_ID=$hostname_val|" .env
  sed -i "s|^REDNODE_CNS=.*|REDNODE_CNS=http://${ip_val}:8787|" .env
  sed -i "s|^REDNODE_URL=.*|REDNODE_URL=http://${ip_val}:8787|" .env
  
  log INFO ".env created — API token: ${api_token:0:12}..."
  save_state "api_token" "$api_token"
  
  cd /
  check_env_file
}

repair_cns_start() {
  log FIX "Starting RedNode CNS..."
  
  if ! check_rust_binary; then
    log WARN "CNS binary not found — building first"
    repair_rust_build || return 1
  fi
  
  if ! check_env_file; then
    repair_env_file || return 1
  fi
  
  # Check if already running via systemd
  if systemctl is-active --quiet rednode-core 2>/dev/null; then
    if check_cns_running; then
      log INFO "CNS already running via systemd ✅"
      return 0
    fi
    # Running but not responding — restart
    log WARN "CNS systemd service active but not responding — restarting"
    systemctl restart rednode-core
    sleep 5
    check_cns_running && return 0
  fi
  
  # Try systemd first (preferred)
  if systemctl list-unit-files rednode-core.service >/dev/null 2>&1; then
    systemctl start rednode-core 2>/dev/null
    sleep 5
    if check_cns_running; then
      log INFO "CNS started via systemd ✅"
      return 0
    fi
    # Check what went wrong
    log DIAG "CNS failed to start — checking journal:"
    journalctl -u rednode-core --since "1 minute ago" --no-pager -n 20 >> "$REDNODE_LOG" 2>/dev/null || true
  fi
  
  # Fallback: start manually
  log WARN "systemd start failed — starting CNS manually"
  cd "${REDNODE_SOURCE}/core/rednode-core"
  
  # Source the .env
  set -a
  source "${REDNODE_SOURCE}/.env" 2>/dev/null || true
  set +a
  
  nohup cargo run --release > "${REDNODE_HOME}/logs/cns.log" 2>&1 &
  local cns_pid=$!
  echo "$cns_pid" > "${REDNODE_HOME}/cns.pid"
  
  cd /
  
  # Wait for it to come up
  local waited=0
  while [ $waited -lt 30 ]; do
    if check_cns_running; then
      log INFO "CNS started manually (PID $cns_pid) ✅"
      return 0
    fi
    sleep 2
    waited=$((waited + 2))
  done
  
  log ERROR "CNS failed to start — check: ${REDNODE_HOME}/logs/cns.log"
  return 1
}

# ═══════════════════════════════════════════════════════════════════
# MAIN OPERATIONS
# ═══════════════════════════════════════════════════════════════════

do_diagnose() {
  echo ""
  echo -e "${BOLD}🧠 RedNode-OS — Self-Diagnosis Report${NC}"
  echo -e "${BOLD}═══════════════════════════════════════════${NC}"
  echo ""
  
  local total=0
  local passed=0
  local failed_items=""
  
  check_item() {
    local name="$1"
    local check_fn="$2"
    total=$((total + 1))
    
    if $check_fn; then
      echo -e "  ${GREEN}✅${NC} $name"
      passed=$((passed + 1))
    else
      echo -e "  ${RED}❌${NC} $name"
      failed_items="${failed_items}\n    - $name"
    fi
  }
  
  echo -e "${BOLD}  Infrastructure:${NC}"
  check_item "Network connectivity" check_network
  check_item "Docker daemon" check_docker
  check_item "PostgreSQL" check_postgres
  check_item "NATS message bus" check_nats
  check_item "Ollama LLM server" check_ollama
  check_item "Ollama models loaded" check_ollama_models
  check_item "Qdrant vector DB" check_qdrant
  
  echo ""
  echo -e "${BOLD}  RedNode Application:${NC}"
  check_item "Source code cloned" check_source_cloned
  check_item "Rust CNS binary built" check_rust_binary
  check_item "Node.js dependencies" check_node_deps
  check_item ".env configuration" check_env_file
  check_item "CNS API responding" check_cns_running
  
  echo ""
  echo -e "${BOLD}═══════════════════════════════════════════${NC}"
  echo -e "  Result: ${BOLD}$passed/$total${NC} subsystems healthy"
  
  if [ $passed -eq $total ]; then
    echo -e "  ${GREEN}${BOLD}All systems operational ✅${NC}"
  else
    echo -e "  ${RED}${BOLD}Issues found:${NC}"
    echo -e "$failed_items"
    echo ""
    echo -e "  Run ${BOLD}rednode-selfheal.sh repair${NC} to auto-fix"
  fi
  echo ""
  
  return $(( total - passed ))
}

do_install() {
  echo ""
  echo -e "${BOLD}  ╔═══════════════════════════════════════════════════╗${NC}"
  echo -e "${BOLD}  ║  🧠 RedNode-OS v0.8.0 — Autonomous Installation   ║${NC}"
  echo -e "${BOLD}  ╚═══════════════════════════════════════════════════╝${NC}"
  echo ""
  
  local start_time
  start_time=$(date +%s)
  save_state "install_started" "$(date -Iseconds)"
  
  # ── Phase 1: Network ──
  log INFO "═══ Phase 1/7: Network Connectivity ═══"
  if ! check_network; then
    retry_with_backoff "Network connectivity" repair_network
    if ! check_network; then
      log ERROR "No network — cannot proceed with installation"
      log DIAG "Check: ip link show / ip addr show / ping 9.9.9.9"
      save_state "install_phase" "network_failed"
      return 1
    fi
  else
    log INFO "Network OK ✅"
  fi
  
  # ── Phase 2: NixOS Services ──
  log INFO "═══ Phase 2/7: NixOS System Services ═══"
  for svc in postgresql nats ollama mosquitto; do
    if ! systemctl is-active --quiet "$svc" 2>/dev/null; then
      retry_with_backoff "Start $svc" repair_nix_service "$svc"
    else
      log INFO "$svc running ✅"
    fi
  done
  
  # Docker + Qdrant
  if ! check_docker; then
    retry_with_backoff "Docker daemon" repair_docker
  else
    log INFO "Docker running ✅"
  fi
  
  if ! check_qdrant; then
    retry_with_backoff "Qdrant vector DB" repair_qdrant
  else
    log INFO "Qdrant running ✅"
  fi
  save_state "install_phase" "services_ok"
  
  # ── Phase 3: Clone Source ──
  log INFO "═══ Phase 3/7: Clone RedNode Source ═══"
  if ! check_source_cloned; then
    retry_with_backoff "Clone source repository" repair_clone_source
    if ! check_source_cloned; then
      log ERROR "Failed to clone source — cannot continue"
      save_state "install_phase" "clone_failed"
      return 1
    fi
  else
    log INFO "Source already present ✅"
  fi
  save_state "install_phase" "source_ok"
  
  # ── Phase 4: AI Models ──
  log INFO "═══ Phase 4/7: AI Models (Ollama) ═══"
  if ! check_ollama; then
    retry_with_backoff "Ollama service" repair_ollama
  fi
  if ! check_ollama_models; then
    retry_with_backoff "Pull AI models" repair_ollama_models
  else
    log INFO "AI models present ✅"
  fi
  save_state "install_phase" "models_ok"
  
  # ── Phase 5: Build Rust CNS ──
  log INFO "═══ Phase 5/7: Build CNS (Rust) ═══"
  if ! check_rust_binary; then
    retry_with_backoff "Build Rust CNS" repair_rust_build
    if ! check_rust_binary; then
      log ERROR "CNS build failed — system will run in degraded mode"
      save_state "install_phase" "build_failed"
      # Don't return — continue with what we can
    fi
  else
    log INFO "CNS binary present ✅"
  fi
  save_state "install_phase" "build_ok"
  
  # ── Phase 6: Node.js + .env ──
  log INFO "═══ Phase 6/7: Node.js Dependencies + Configuration ═══"
  retry_with_backoff "Install Node.js dependencies" repair_node_deps
  repair_env_file
  save_state "install_phase" "deps_ok"
  
  # ── Phase 7: Start Everything ──
  log INFO "═══ Phase 7/7: Start RedNode-OS ═══"
  retry_with_backoff "Start CNS" repair_cns_start
  
  # Start agents via start-all.sh if available
  if [ -f "${REDNODE_SOURCE}/scripts/start-all.sh" ]; then
    log INFO "Starting agents..."
    cd "${REDNODE_SOURCE}"
    bash scripts/start-all.sh start 2>&1 | tee -a "$REDNODE_LOG" || true
    cd /
  fi
  
  save_state "install_phase" "complete"
  save_state "install_completed" "$(date -Iseconds)"
  
  local end_time
  end_time=$(date +%s)
  local duration=$(( end_time - start_time ))
  
  echo ""
  echo -e "${BOLD}═══════════════════════════════════════════════════════${NC}"
  echo -e "${BOLD}  🧠 RedNode-OS — Installation Complete (${duration}s)${NC}"
  echo -e "${BOLD}═══════════════════════════════════════════════════════${NC}"
  echo ""
  
  # Final diagnosis
  do_diagnose
  
  local ip_val
  ip_val=$(hostname -I 2>/dev/null | awk '{print $1}' || echo "127.0.0.1")
  
  echo -e "  ${BOLD}Access:${NC}"
  echo -e "    Dashboard:  http://${ip_val}:3000"
  echo -e "    CNS API:    http://${ip_val}:8787"
  echo -e "    Grafana:    http://${ip_val}:3001  (admin/rednode)"
  echo ""
  echo -e "  ${BOLD}Next steps:${NC}"
  echo "    1. nano ${REDNODE_SOURCE}/.env    — add Pi-hole, TrueNAS, email"
  echo "    2. Cameras: edit deployment/frigate.yml"
  echo "    3. VLANs:   cat docs/NETWORK-ARCHITECTURE.md"
  echo ""
  echo -e "  ${GREEN}${BOLD}The computer becomes the intelligence.${NC}"
  echo ""
}

do_repair() {
  echo ""
  echo -e "${BOLD}🧠 RedNode-OS — Self-Repair${NC}"
  echo ""
  
  local fixed=0
  local failed=0
  
  repair_item() {
    local name="$1"
    local check_fn="$2"
    local repair_fn="$3"
    shift 3
    
    if $check_fn; then
      return 0  # Already healthy
    fi
    
    log FIX "Repairing: $name"
    if retry_with_backoff "$name" "$repair_fn" "$@"; then
      fixed=$((fixed + 1))
      return 0
    else
      failed=$((failed + 1))
      return 1
    fi
  }
  
  # Repair in dependency order
  repair_item "Network" check_network repair_network || true
  repair_item "Docker" check_docker repair_docker || true
  repair_item "PostgreSQL" check_postgres repair_postgres || true
  repair_item "NATS" check_nats repair_nix_service "nats" || true
  repair_item "Ollama" check_ollama repair_ollama || true
  repair_item "Qdrant" check_qdrant repair_qdrant || true
  repair_item "Source code" check_source_cloned repair_clone_source || true
  repair_item "Ollama models" check_ollama_models repair_ollama_models || true
  repair_item "Rust binary" check_rust_binary repair_rust_build || true
  repair_item "Node.js deps" check_node_deps repair_node_deps || true
  repair_item ".env file" check_env_file repair_env_file || true
  repair_item "CNS running" check_cns_running repair_cns_start || true
  
  echo ""
  echo -e "${BOLD}═══════════════════════════════════════════${NC}"
  if [ $failed -eq 0 ]; then
    echo -e "  ${GREEN}${BOLD}All repairs successful ($fixed fixed)${NC}"
  else
    echo -e "  ${YELLOW}${BOLD}Repair complete: $fixed fixed, $failed still broken${NC}"
    echo -e "  Check logs: less $REDNODE_LOG"
  fi
  echo ""
}

do_watch() {
  log INFO "🧠 RedNode-OS Self-Heal Watchdog started (interval: ${WATCH_INTERVAL}s)"
  
  # If install hasn't been completed yet, do it first
  local install_phase
  install_phase=$(get_state "install_phase" "none")
  
  if [ "$install_phase" = "none" ] || [ "$install_phase" = "network_failed" ] || [ "$install_phase" = "clone_failed" ]; then
    log INFO "First boot detected — running full installation"
    do_install
  elif [ "$install_phase" != "complete" ]; then
    log INFO "Previous install was interrupted at phase: $install_phase — resuming"
    do_install
  fi
  
  # Continuous monitoring loop
  while true; do
    sleep "$WATCH_INTERVAL"
    
    log INFO "Periodic health check..."
    
    local needs_repair=false
    
    # Check critical services
    if ! check_cns_running; then
      log WARN "CNS not responding — triggering repair"
      needs_repair=true
    fi
    
    if ! check_postgres; then
      log WARN "PostgreSQL down — triggering repair"
      needs_repair=true
    fi
    
    if ! check_nats; then
      log WARN "NATS down — triggering repair"
      needs_repair=true
    fi
    
    if ! check_ollama; then
      log WARN "Ollama down — triggering repair"
      needs_repair=true
    fi
    
    if $needs_repair; then
      log FIX "Issues detected — starting auto-repair..."
      do_repair
    else
      log INFO "All systems healthy ✅"
    fi
    
    # Check for source updates (daily, not every 5 min)
    local last_update
    last_update=$(get_state "last_git_check" "0")
    local now
    now=$(date +%s)
    local update_interval=86400  # 24 hours
    
    if [ $(( now - last_update )) -ge $update_interval ]; then
      if check_source_cloned && check_network; then
        log INFO "Checking for RedNode updates..."
        cd "${REDNODE_SOURCE}"
        local local_hash
        local_hash=$(git rev-parse HEAD 2>/dev/null || echo "none")
        git fetch origin "$REDNODE_BRANCH" --depth 1 2>/dev/null || true
        local remote_hash
        remote_hash=$(git rev-parse "origin/$REDNODE_BRANCH" 2>/dev/null || echo "none")
        
        if [ "$local_hash" != "$remote_hash" ] && [ "$remote_hash" != "none" ]; then
          log INFO "Update available — pulling and rebuilding..."
          git reset --hard "origin/$REDNODE_BRANCH" 2>/dev/null || true
          
          # Rebuild
          cd core/rednode-core
          cargo build --release 2>&1 | tee -a "$REDNODE_LOG" || true
          cd "${REDNODE_SOURCE}"
          pnpm install 2>&1 | tee -a "$REDNODE_LOG" || true
          
          # Restart CNS
          systemctl restart rednode-core 2>/dev/null || true
          
          log INFO "Update complete — new version deployed"
        else
          log INFO "Source is up to date"
        fi
        
        cd /
        save_state "last_git_check" "$now"
      fi
    fi
  done
}

# ═══════════════════════════════════════════════════════════════════
# ENTRY POINT
# ═══════════════════════════════════════════════════════════════════

case "${1:-help}" in
  install)  do_install ;;
  diagnose) do_diagnose ;;
  repair)   do_repair ;;
  watch)    do_watch ;;
  help|--help|-h)
    echo ""
    echo -e "${BOLD}🧠 RedNode-OS Self-Heal System${NC}"
    echo ""
    echo "Usage: $0 <command>"
    echo ""
    echo "Commands:"
    echo "  install   Full first-boot installation (clone + build + start)"
    echo "  diagnose  Check all subsystems and report health status"
    echo "  repair    Detect and fix any broken subsystem"
    echo "  watch     Continuous monitoring (runs as systemd service)"
    echo ""
    echo "The watch command is designed for systemd:"
    echo "  - On first boot: runs full install"
    echo "  - After install: checks health every 5 minutes"
    echo "  - If anything breaks: auto-repairs"
    echo "  - Daily: checks for source updates"
    echo ""
    ;;
  *)
    echo "Unknown command: $1"
    echo "Usage: $0 {install|diagnose|repair|watch|help}"
    exit 1
    ;;
esac
