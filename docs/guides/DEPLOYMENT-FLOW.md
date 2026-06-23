# RedNode-OS — Deployment Flow: What Happens When You Boot

> This document explains **exactly** what happens at every stage, so you know what's going on behind the scenes.

---

## The Short Answer

After you install NixOS with RedNode's config and reboot:

1. **NixOS boots** → all system services start automatically (PostgreSQL, NATS, Ollama, Docker, etc.)
2. **`rednode-deploy.service` runs** → clones the repo, builds everything, starts RedNode
3. **`rednode-selfheal.service` runs** → monitors every 5 minutes, auto-repairs crashes
4. **You access** `http://YOUR-IP:3000` (dashboard) or `rednode status` (CLI)

**You do NOT need to:**
- Manually `git clone` anything
- Manually run `cargo build`
- Manually run `pnpm install`
- Manually start any service
- Worry about crashes

---

## The Long Answer: Step-by-Step

### Phase 1: NixOS Installation (you do this once)

```bash
# 1. Boot NixOS installer (USB/ISO)
# 2. Partition disk, mount filesystems
# 3. Clone RedNode config:
git clone https://github.com/Cosm1cBug/RedNode-OS-Demo.git ~/RedNode-OS-Demo

# 4. Copy NixOS config:
sudo cp -r ~/RedNode-OS-Demo/os/nixos/* /etc/nixos/

# OR use the flake directly:
cd ~/RedNode-OS-Demo/os/nixos
sudo nixos-rebuild switch --flake .#rednode

# 5. Set your password:
passwd owner

# 6. Reboot
sudo reboot
```

### Phase 2: First Boot — What Happens Automatically

```
BIOS → GRUB/systemd-boot → Linux kernel → systemd
                                              │
                                              ├─ postgresql.service    ✅ starts
                                              ├─ nats.service          ✅ starts
                                              ├─ ollama.service        ✅ starts
                                              ├─ docker.service        ✅ starts
                                              ├─ mosquitto.service     ✅ starts
                                              ├─ grafana.service       ✅ starts
                                              ├─ prometheus.service    ✅ starts
                                              ├─ loki.service          ✅ starts
                                              ├─ qdrant (docker)       ✅ starts
                                              │
                                              └─ rednode-deploy.service (RUNS ONCE)
                                                  │
                                                  ├─ Phase 1: Check network
                                                  ├─ Phase 2: Verify all NixOS services
                                                  ├─ Phase 3: git clone → /var/lib/rednode/source/
                                                  ├─ Phase 4: Detect GPU → pull best Ollama model
                                                  ├─ Phase 5: cargo build --release (CNS)
                                                  ├─ Phase 6: pnpm install + create .env
                                                  └─ Phase 7: Start CNS + 16 agents + dashboard
                                                      │
                                                      └─ Creates /var/lib/rednode/.deploy-complete
                                                         (so it won't re-run on next boot)
```

**Timeline:**
| Step | Duration | What's happening |
|---|---|---|
| NixOS boot | 15-30s | Kernel, systemd, NixOS services |
| Git clone | 10-30s | Cloning ~210 files from GitHub |
| Ollama model pull | 5-20 min | Downloading 4-9 GB LLM model |
| Cargo build | 3-5 min | Compiling 17 Rust modules |
| pnpm install | 30-60s | Installing Node.js dependencies |
| Start services | 10s | CNS + agents + dashboard |
| **Total first boot** | **~10-25 min** | One-time only |

### Phase 3: Continuous Self-Healing (Every Boot After)

After the first boot, `rednode-deploy.service` doesn't run again (the marker file exists).
Instead, `rednode-selfheal.service` runs continuously:

```
rednode-selfheal.service (always running)
    │
    ├─ On start: checks if install was completed
    │   ├─ YES → enters monitoring loop
    │   └─ NO  → runs full install first, then monitors
    │
    ├─ Every 5 minutes: health check
    │   ├─ CNS API responding?        → if not, restart
    │   ├─ PostgreSQL running?        → if not, restart
    │   ├─ NATS running?              → if not, restart
    │   ├─ Ollama running?            → if not, restart
    │   └─ All OK? → log "healthy" and sleep
    │
    └─ Every 24 hours: update check
        ├─ git fetch origin main
        ├─ New commits? → git pull + cargo build + pnpm install + restart
        └─ No changes? → skip
```

---

## What Happens When Things Fail?

### Scenario 1: `cargo build` fails

```
Self-heal detects: Rust binary missing
    │
    ├─ Attempt 1: cargo build --release
    │   └─ FAILED: "could not find openssl"
    │       → Diagnoses: missing openssl dev headers
    │       → Logs diagnosis to /var/lib/rednode/logs/selfheal.log
    │
    ├─ Attempt 2: retry after 10s
    │   └─ Same error → waits 20s
    │
    ├─ Attempt 3: retry after 20s
    │   └─ Same error → waits 40s
    │
    ├─ Attempt 4: "disk full" detected
    │   → Runs: nix-collect-garbage --delete-older-than 3d
    │   → Runs: cargo clean
    │   → Retries build
    │
    ├─ Attempt 5: "compilation error"
    │   → Runs: git pull (fetch latest fixes)
    │   → Retries build
    │
    └─ All 5 failed:
        → Logs error with full diagnosis
        → System continues in DEGRADED mode
        → NixOS services still running
        → Retries again in 5 minutes (watchdog loop)
```

### Scenario 2: PostgreSQL crashes at 3 AM

```
Self-heal watchdog (every 5 min):
    │
    ├─ check_postgres → FAILED
    │
    ├─ repair_postgres:
    │   ├─ systemctl restart postgresql
    │   ├─ Wait 3s → check again
    │   ├─ If "Address already in use":
    │   │   → fuser -k 5432/tcp → kill conflicting process
    │   │   → restart postgresql
    │   ├─ If "permission denied":
    │   │   → chown -R postgres:postgres /var/lib/postgresql
    │   │   → restart postgresql
    │   ├─ If "No space left":
    │   │   → journalctl --vacuum-size=100M
    │   │   → nix-collect-garbage
    │   │   → restart postgresql
    │   └─ If database "rednode" missing:
    │       → createdb rednode
    │       → CREATE EXTENSION vector
    │
    └─ Log result → sleep 5 min → check again
```

### Scenario 3: Network is down on boot

```
Self-heal install:
    │
    ├─ check_network → FAILED (no internet)
    │
    ├─ repair_network:
    │   ├─ Check: is any NIC UP?
    │   │   └─ NO → ip link set enp0s31f6 up
    │   ├─ Check: is static IP assigned?
    │   │   └─ NO → systemctl restart systemd-networkd
    │   ├─ Check: DNS working?
    │   │   └─ NO → add 9.9.9.9 to resolv.conf
    │   └─ Retry up to 5 times with backoff
    │
    ├─ Still no network after 5 retries:
    │   → Log error
    │   → Cannot clone repo → installation paused
    │   → BUT: NixOS services still running (Postgres, NATS, Ollama)
    │   → Watchdog retries in 5 minutes
    │   → When network comes back → install resumes automatically
    │
    └─ State saved to /var/lib/rednode/.selfheal-state
        (tracks which phase completed, so it resumes — not restarts)
```

### Scenario 4: Ollama model download interrupted

```
Self-heal:
    │
    ├─ check_ollama_models → FAILED (no model pulled)
    │
    ├─ repair_ollama_models:
    │   ├─ Detect GPU: nvidia-smi → RTX 3060 12GB
    │   ├─ Select: qwen2.5:14b-instruct-q4_K_M
    │   ├─ ollama pull qwen2.5:14b-instruct-q4_K_M
    │   │   └─ Ollama handles resume automatically
    │   │       (partial downloads continue from where they stopped)
    │   ├─ ollama pull nomic-embed-text
    │   └─ Save selected model to state file
    │
    └─ Retries up to 5 times if pull fails (network issues)
```

---

## File Locations on Your Machine

After first boot, everything lives here:

```
/var/lib/rednode/                    ← RedNode's home
├── source/                          ← Git clone of the repo
│   ├── core/rednode-core/           ← Rust CNS source + binary
│   ├── agents/                      ← 18 agent directories
│   ├── web/                         ← Next.js dashboard
│   ├── scripts/                     ← All scripts
│   ├── .env                         ← Your configuration (auto-generated)
│   └── ...
├── logs/
│   ├── selfheal.log                 ← Self-heal system log
│   ├── cns.log                      ← CNS output
│   └── *.log                        ← Agent logs
├── qdrant/                          ← Vector memory data
├── backups/                         ← Automated backups
├── .selfheal-state                  ← Install progress tracker
├── .deploy-complete                 ← Marker: first boot done
└── cns.pid                          ← CNS process ID
```

NixOS-managed services store data in their default locations:
```
/var/lib/postgresql/    ← Structured memory (propositions, audit log)
/var/lib/nats/          ← Message bus (JetStream)
/var/lib/ollama/        ← LLM models
/var/lib/loki/          ← Log aggregation
/var/lib/grafana/       ← Dashboard config
```

---

## CLI Commands (available after first boot)

```bash
# Health check — shows 12 subsystem status
rednode status

# Auto-repair anything broken
rednode repair

# Send intent to the AI
rednode intent "check camera events from today"

# View self-heal logs
rednode logs

# Pull latest code + rebuild
rednode update
```

---

## Troubleshooting

### "First boot is stuck — nothing happening"

```bash
# Check if deploy service is running:
sudo systemctl status rednode-deploy

# Watch its logs in real-time:
sudo journalctl -u rednode-deploy -f

# If it failed, check the log:
sudo cat /var/lib/rednode/logs/selfheal.log
```

### "First boot completed but dashboard doesn't load"

```bash
# Run diagnosis:
rednode status

# If CNS shows ❌:
rednode repair

# Manual check:
curl http://localhost:8787/health
```

### "Model download is very slow"

```bash
# Check Ollama progress:
sudo journalctl -u ollama -f

# Or check the selfheal log for which model was selected:
grep "selected" /var/lib/rednode/logs/selfheal.log
```

### "I want to change the LLM model"

```bash
# Edit .env:
nano /var/lib/rednode/source/.env
# Change REDNODE_MODEL=qwen2.5:14b-instruct-q4_K_M to whatever you want

# Pull the new model:
ollama pull your-new-model

# Restart CNS:
sudo systemctl restart rednode-core
```

### "I want to force re-install from scratch"

```bash
# Remove the completion marker:
sudo rm /var/lib/rednode/.deploy-complete
sudo rm /var/lib/rednode/.selfheal-state

# Reboot — deploy service will run again
sudo reboot

# Or trigger manually:
sudo /etc/rednode/selfheal.sh install
```
