#!/usr/bin/env bash
# ═══════════════════════════════════════════════
# RedNode-OS — Hardware Detection & Auto-Configuration
# Detects GPU (NVIDIA/AMD/CPU-only), VRAM, RAM, CPU cores
# Selects optimal LLM model, Whisper model, and memory limits
# Outputs: JSON config to stdout, or writes to .env
#
# Usage:
#   ./scripts/rednode-hardware-detect.sh          # print config
#   ./scripts/rednode-hardware-detect.sh --apply   # write to .env
# ═══════════════════════════════════════════════

set -euo pipefail

# ─── GPU Detection ───

GPU_VENDOR="none"  # "nvidia", "amd", "none"
GPU_NAME="none"
GPU_VRAM_MB=0
GPU_DRIVER=""

# Try NVIDIA first
if command -v nvidia-smi >/dev/null 2>&1; then
  GPU_INFO=$(nvidia-smi --query-gpu=name,memory.total,driver_version --format=csv,noheader,nounits 2>/dev/null || echo "")
  if [ -n "$GPU_INFO" ]; then
    GPU_VENDOR="nvidia"
    GPU_NAME=$(echo "$GPU_INFO" | cut -d',' -f1 | xargs)
    GPU_VRAM_MB=$(echo "$GPU_INFO" | cut -d',' -f2 | xargs)
    GPU_DRIVER=$(echo "$GPU_INFO" | cut -d',' -f3 | xargs)
  fi
fi

# Try AMD ROCm
if [ "$GPU_VENDOR" = "none" ] && command -v rocm-smi >/dev/null 2>&1; then
  AMD_NAME=$(rocm-smi --showproductname 2>/dev/null | grep -i "gpu\|card" | head -1 | sed 's/.*: //' | xargs || echo "")
  if [ -n "$AMD_NAME" ]; then
    GPU_VENDOR="amd"
    GPU_NAME="$AMD_NAME"
    # Get VRAM from rocm-smi
    AMD_VRAM=$(rocm-smi --showmeminfo vram 2>/dev/null | grep "Total" | awk '{print $NF}' | head -1 || echo "0")
    # Convert from bytes to MB if needed
    if [ "$AMD_VRAM" -gt 1000000 ]; then
      GPU_VRAM_MB=$((AMD_VRAM / 1024 / 1024))
    else
      GPU_VRAM_MB=$AMD_VRAM
    fi
    GPU_DRIVER=$(rocm-smi --showdriverversion 2>/dev/null | grep "Driver" | awk '{print $NF}' || echo "unknown")
  fi
fi

# Try AMD via lspci (no ROCm installed)
if [ "$GPU_VENDOR" = "none" ]; then
  AMD_PCI=$(lspci 2>/dev/null | grep -i "VGA\|3D\|Display" | grep -i "AMD\|ATI\|Radeon" | head -1 || echo "")
  if [ -n "$AMD_PCI" ]; then
    GPU_VENDOR="amd"
    GPU_NAME=$(echo "$AMD_PCI" | sed 's/.*: //')
    # Estimate VRAM from card name (rough heuristic)
    if echo "$GPU_NAME" | grep -qi "7900\|6900\|6800"; then GPU_VRAM_MB=16384;
    elif echo "$GPU_NAME" | grep -qi "7800\|6700"; then GPU_VRAM_MB=12288;
    elif echo "$GPU_NAME" | grep -qi "7600\|6600"; then GPU_VRAM_MB=8192;
    elif echo "$GPU_NAME" | grep -qi "6500\|6400"; then GPU_VRAM_MB=4096;
    else GPU_VRAM_MB=8192; fi # default guess
    GPU_DRIVER="(install ROCm for accurate detection)"
  fi
fi

# ─── CPU Detection ───

CPU_MODEL=$(grep "model name" /proc/cpuinfo 2>/dev/null | head -1 | sed 's/.*: //' || echo "unknown")
CPU_CORES=$(nproc 2>/dev/null || echo "4")

# ─── RAM Detection ───

RAM_TOTAL_MB=$(($(grep MemTotal /proc/meminfo 2>/dev/null | awk '{print $2}' || echo "16000000") / 1024))
RAM_TOTAL_GB=$((RAM_TOTAL_MB / 1024))

# ─── Model Selection ───

LLM_MODEL=""
WHISPER_MODEL=""
OLLAMA_ACCELERATION=""

# Set Ollama acceleration based on GPU vendor
case "$GPU_VENDOR" in
  nvidia) OLLAMA_ACCELERATION="cuda" ;;
  amd)    OLLAMA_ACCELERATION="rocm" ;;
  *)      OLLAMA_ACCELERATION="" ;;
esac

# Select LLM model based on VRAM
if [ "$GPU_VRAM_MB" -ge 24000 ]; then
  LLM_MODEL="qwen2.5:32b-instruct-q4_K_M"
  WHISPER_MODEL="large-v3"
elif [ "$GPU_VRAM_MB" -ge 12000 ]; then
  LLM_MODEL="qwen2.5:14b-instruct-q4_K_M"
  WHISPER_MODEL="small"
elif [ "$GPU_VRAM_MB" -ge 6000 ]; then
  LLM_MODEL="qwen2.5:7b-instruct-q4_K_M"
  WHISPER_MODEL="small"
elif [ "$GPU_VRAM_MB" -ge 3000 ]; then
  LLM_MODEL="qwen2.5:3b-instruct-q4_K_M"
  WHISPER_MODEL="base"
else
  LLM_MODEL="qwen2.5:3b-instruct-q4_K_M"
  WHISPER_MODEL="base"
fi

# ─── Memory Optimization Settings ───
# Based on available RAM, set optimal limits for each service

if [ "$RAM_TOTAL_GB" -ge 64 ]; then
  PG_SHARED_BUFFERS="512MB"
  PG_EFFECTIVE_CACHE="4GB"
  QDRANT_RAM_MB=2048
  EVENT_BUS_CAP=2048
  NATS_MAX_MEM=512000000
  MEM_PROFILE="large"
elif [ "$RAM_TOTAL_GB" -ge 32 ]; then
  PG_SHARED_BUFFERS="256MB"
  PG_EFFECTIVE_CACHE="1GB"
  QDRANT_RAM_MB=1024
  EVENT_BUS_CAP=1024
  NATS_MAX_MEM=256000000
  MEM_PROFILE="recommended"
elif [ "$RAM_TOTAL_GB" -ge 16 ]; then
  PG_SHARED_BUFFERS="128MB"
  PG_EFFECTIVE_CACHE="512MB"
  QDRANT_RAM_MB=512
  EVENT_BUS_CAP=512
  NATS_MAX_MEM=128000000
  MEM_PROFILE="standard"
else
  PG_SHARED_BUFFERS="64MB"
  PG_EFFECTIVE_CACHE="256MB"
  QDRANT_RAM_MB=256
  EVENT_BUS_CAP=256
  NATS_MAX_MEM=64000000
  MEM_PROFILE="minimal"
fi

# ─── Output ───

if [ "${1:-}" = "--json" ]; then
  cat << EOF
{
  "gpu": {
    "vendor": "$GPU_VENDOR",
    "name": "$GPU_NAME",
    "vram_mb": $GPU_VRAM_MB,
    "driver": "$GPU_DRIVER"
  },
  "cpu": {
    "model": "$CPU_MODEL",
    "cores": $CPU_CORES
  },
  "ram": {
    "total_mb": $RAM_TOTAL_MB,
    "total_gb": $RAM_TOTAL_GB
  },
  "models": {
    "llm": "$LLM_MODEL",
    "whisper": "$WHISPER_MODEL",
    "embed": "nomic-embed-text",
    "ollama_acceleration": "$OLLAMA_ACCELERATION"
  },
  "memory_profile": "$MEM_PROFILE",
  "memory_settings": {
    "pg_shared_buffers": "$PG_SHARED_BUFFERS",
    "pg_effective_cache": "$PG_EFFECTIVE_CACHE",
    "qdrant_ram_mb": $QDRANT_RAM_MB,
    "event_bus_capacity": $EVENT_BUS_CAP,
    "nats_max_memory": $NATS_MAX_MEM
  }
}
EOF
elif [ "${1:-}" = "--apply" ]; then
  # Write to .env
  ENV_FILE="${2:-.env}"
  if [ -f "$ENV_FILE" ]; then
    sed -i "s|^REDNODE_MODEL=.*|REDNODE_MODEL=$LLM_MODEL|" "$ENV_FILE"
    sed -i "s|^WHISPER_MODEL=.*|WHISPER_MODEL=$WHISPER_MODEL|" "$ENV_FILE"
  fi
  echo "Applied: LLM=$LLM_MODEL, Whisper=$WHISPER_MODEL, Acceleration=$OLLAMA_ACCELERATION, Profile=$MEM_PROFILE"
else
  echo "🧠 RedNode-OS — Hardware Detection"
  echo ""
  echo "  GPU:      $GPU_NAME ($GPU_VENDOR)"
  echo "  VRAM:     ${GPU_VRAM_MB} MB"
  echo "  Driver:   $GPU_DRIVER"
  echo "  CPU:      $CPU_MODEL ($CPU_CORES cores)"
  echo "  RAM:      ${RAM_TOTAL_GB} GB"
  echo ""
  echo "  LLM:      $LLM_MODEL"
  echo "  Whisper:  $WHISPER_MODEL"
  echo "  Accel:    $OLLAMA_ACCELERATION"
  echo "  Profile:  $MEM_PROFILE"
fi
