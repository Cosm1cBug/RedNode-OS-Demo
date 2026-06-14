# RedNode-OS — START HERE

> **Your situation**: NixOS-comfortable, Rust+TS proficient, all network hardware ready (pfSense mini-PC, Raspberry Pi, managed switch), need to pick a PC for RedNode.
>
> **This is your Day 1 through Week 3 execution plan. Concrete. Ordered. No fluff.**

---

## Step 0: Pick Your RedNode PC (Today — 30 minutes)

You have multiple PCs. Pick based on this checklist:

```
THE PC YOU CHOOSE MUST HAVE:
  ✅ x86_64 CPU, 4+ cores
  ✅ 16 GB+ RAM (32 GB ideal)
  ✅ An SSD (any size, 120 GB minimum)
  ✅ A PCIe slot that fits your NVIDIA GPU
  ✅ A working ethernet port

CHECK YOUR GPU:
  Run on the candidate PC:
    $ lspci | grep -i nvidia
    $ nvidia-smi  (if drivers installed)

  What you need to know:
    GPU model → how much VRAM → which LLM you can run

    6 GB (GTX 1060)     → Qwen2.5-7B   → works, not great
    8 GB (RTX 3060 8GB) → Qwen2.5-7B   → comfortable
    12 GB (RTX 3060)    → Qwen2.5-14B  → sweet spot ⭐
    16 GB (RTX 4060 Ti) → Qwen2.5-14B  → plenty of room
    24 GB (RTX 4090)    → Qwen2.5-32B  → overkill

  CHECK RAM:
    $ free -h
    Need: 16 GB minimum, 32 GB recommended

  CHECK SSD:
    $ lsblk
    Need: SSD (not HDD), 120 GB+
```

**Decision made? Write it down and move on.**

---

## Step 1: Install NixOS on the RedNode PC (Day 1 — 2 hours)

You're comfortable with NixOS, so this is straightforward. Don't use RedNode's `configuration.nix` yet — start with a clean minimal install, then layer RedNode on top.

```bash
# 1. Download NixOS 24.05 minimal ISO
# 2. Flash to USB, boot, install with:
#    - UEFI boot
#    - ext4 (not ZFS/btrfs for now — keep it simple)
#    - swap = RAM size (for hibernation later)
#    - Username: your name
#    - Enable flakes in configuration.nix:
#      nix.settings.experimental-features = [ "nix-command" "flakes" ];

# 3. After reboot, install NVIDIA drivers:
# In /etc/nixos/configuration.nix:
services.xserver.videoDrivers = [ "nvidia" ];
hardware.nvidia = {
  modesetting.enable = true;
  open = false;  # use proprietary for CUDA
  nvidiaSettings = true;
};
hardware.graphics.enable = true;

# 4. Rebuild and reboot:
sudo nixos-rebuild switch
reboot

# 5. Verify GPU:
nvidia-smi
# Should show your GPU model, driver version, VRAM
```

---

## Step 2: Install RedNode Dependencies (Day 1 — 1 hour)

```bash
# Add to configuration.nix environment.systemPackages:
environment.systemPackages = with pkgs; [
  # Build tools
  rustc cargo clippy rustfmt pkg-config openssl
  nodejs_22 pnpm

  # RedNode infrastructure
  docker docker-compose

  # AI
  ollama

  # Tools
  git curl wget vim htop btop
  natscli
  firejail  # for sandboxed executor

  # Python (for voice later)
  python312
  python312Packages.pip
];

# Enable Docker:
virtualisation.docker.enable = true;
users.users.YOURNAME.extraGroups = [ "docker" ];

# Enable Ollama:
services.ollama = {
  enable = true;
  acceleration = "cuda";
  host = "0.0.0.0";
  port = 11434;
};

# Rebuild:
sudo nixos-rebuild switch

# Verify:
docker --version
ollama --version
cargo --version
pnpm --version
nvidia-smi
```

---

## Step 3: Pull AI Models (Day 1 — 30 min, download time varies)

```bash
# Pick ONE based on your GPU VRAM:

# If 6-8 GB VRAM:
ollama pull qwen2.5:7b-instruct-q4_K_M

# If 12+ GB VRAM:
ollama pull qwen2.5:14b-instruct-q4_K_M

# Always pull the embedding model:
ollama pull nomic-embed-text

# Verify:
ollama list
ollama run qwen2.5:7b "What is NixOS?"
# Should respond in 1-3 seconds on GPU
```

---

## Step 4: Start Infrastructure (Day 1 — 30 min)

```bash
cd ~/RedNode-OS-Demo/deployment
docker compose up -d

# Verify everything is running:
docker ps
# Should see: rednode-nats, rednode-postgres, rednode-qdrant,
#             rednode-loki, rednode-prometheus, rednode-grafana,
#             rednode-otel

# Test connections:
curl http://localhost:4222  # NATS (will error but means it's listening)
psql postgres://rednode:rednode@localhost:5432/rednode -c "SELECT 1;"
curl http://localhost:6333/collections  # Qdrant
curl http://localhost:11434/api/tags    # Ollama (already running via NixOS)
curl http://localhost:3001              # Grafana (admin/rednode)
```

---

## Step 5: Build and Run the CNS (Day 2 — 1 hour)

```bash
cd ~/RedNode-OS-Demo/core/rednode-core

# Build (first build takes 2-5 minutes):
cargo build

# Run:
cargo run
# Should see:
#   🧠 RedNode-OS v0.3.1 – CNS starting
#   Postgres connected
#   NATS connected
#   Qdrant ...
#   Sentience Engine online
#   CNS listening on http://0.0.0.0:8787

# Test in another terminal:
curl http://localhost:8787/health
# → {"ok":true,"node":"rednode-cns","version":"0.2.0"}

curl -X POST http://localhost:8787/intent \
  -H "Content-Type: application/json" \
  -d '{"intent":"show system health"}'
# → {"ok":true,"intent":"show system health","plan":[...],"results":[...]}

curl http://localhost:8787/sentience
# → {"ok":true,"sentience":true,"model":{...drives...}}
```

**⚠️ At this point, the planner is keyword-based. "show system health" works because it contains "system" and "health". Random intents will fall through to research.query. That's fine — we fix this in Step 8.**

---

## Step 6: Start Agents + Web UI (Day 2 — 30 min)

```bash
# Terminal 1: agents
cd ~/RedNode-OS-Demo
pnpm install
pnpm agents
# Should see 6 agents connecting to NATS

# Terminal 2: web
pnpm web
# → http://localhost:3000

# Open in browser: http://localhost:3000
# You should see the 8-tab dashboard
# Click through each tab — verify data loads

# Test from mobile (if on same network):
# http://REDNODE-IP:3000
```

---

## Step 7: Set Up Your Network (Day 2-3)

You have all the hardware. Do this in order:

```
1. pfSense mini-PC:
   □ Flash pfSense ISO to USB
   □ Install on mini-PC
   □ WAN port → ISP router (bridge mode)
   □ LAN port → managed switch (trunk, all VLANs)
   □ Configure VLANs:
     - VLAN 10: 10.0.10.0/24 (Trusted — your devices)
     - VLAN 20: 10.0.20.0/24 (IoT)
     - VLAN 30: 10.0.30.0/24 (Cameras — BLOCK ALL INTERNET)
     - VLAN 40: 10.0.40.0/24 (Guest — internet only)
     - VLAN 50: 10.0.50.0/24 (Management — RedNode, Pi-hole, TrueNAS)
   □ DHCP for each VLAN
   □ DNS for all DHCP scopes → 10.0.50.2 (Pi-hole, set up next)
   □ Firewall rules (see RedNode-Network-Architecture.md)

2. Raspberry Pi (Pi-hole):
   □ Flash Raspberry Pi OS Lite
   □ Install Pi-hole: curl -sSL https://install.pi-hole.net | bash
   □ Static IP: 10.0.50.2
   □ Connect to switch on VLAN 50 port
   □ Upstream DNS: Quad9 (9.9.9.9) or Cloudflare (1.1.1.1)
   □ Test: nslookup google.com 10.0.50.2

3. Managed switch:
   □ Configure VLAN tagging on trunk port (to pfSense)
   □ Set access ports per VLAN (see network architecture doc)
   □ Move RedNode server to VLAN 50 port → static IP 10.0.50.10
   □ Move TrueNAS to VLAN 50 port → static IP 10.0.50.3
   □ Move NVR to VLAN 30 port → static IP 10.0.30.2

4. Verify:
   □ From your workstation (VLAN 10): can reach http://10.0.50.10:3000
   □ From your workstation: can reach http://10.0.50.2/admin (Pi-hole)
   □ From your workstation: can reach TrueNAS at https://10.0.50.3
   □ Cameras (VLAN 30): ping 8.8.8.8 FAILS (no internet — correct!)
   □ All devices getting DNS from Pi-hole
```

---

## Step 8: THE CRITICAL FIX — Replace the Planner (Day 4 — 3-4 hours)

**This is the single most important code change. Everything else is incremental. This is transformational.**

```bash
# Edit: core/rednode-core/src/planner.rs
# Replace the entire file:
```

Here's what you replace it with (conceptual structure — you'll adapt):

```rust
use crate::security::Risk;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PlanStep {
    pub tool: String,
    pub agent: String,
    pub args: serde_json::Value,
    pub risk: Risk,
}

// The tool registry as context for the LLM
const TOOL_CONTEXT: &str = r#"
Available tools (name | agent | risk):
- fs.read | system-agent | low — read a file
- process.list | system-agent | low — list running processes
- docker.ps | system-agent | low — list docker containers
- service.status | system-agent | low — check systemd service
- shell.run_safe | system-agent | medium — run allowlisted command
- sec.triage | security-agent | low — check system logs for warnings
- sec.cve_check | security-agent | low — scan for CVEs
- sec.ssh_audit | security-agent | medium — audit SSH config
- sec.harden_ssh | security-agent | high — harden SSH config
- sec.patch | security-agent | high — apply security patches
- code.analyze | coding-agent | low — analyze code quality
- code.test | coding-agent | medium — run tests
- research.query | research-agent | low — search knowledge base
- net.status | network-agent | low — show network connections
- firewall.rules | network-agent | high — manage firewall rules
- dns.check | network-agent | low — check DNS status
"#;

pub async fn plan(intent: &str) -> Vec<PlanStep> {
    // Try LLM-powered planning first
    match plan_with_llm(intent).await {
        Ok(steps) if !steps.is_empty() => return steps,
        Ok(_) => tracing::warn!("LLM returned empty plan, falling back"),
        Err(e) => tracing::warn!("LLM planner failed: {}, falling back to keyword", e),
    }
    // Fallback: keyword matching (your current code)
    plan_keyword_fallback(intent)
}

async fn plan_with_llm(intent: &str) -> anyhow::Result<Vec<PlanStep>> {
    let ollama_url = std::env::var("OLLAMA_URL")
        .unwrap_or("http://127.0.0.1:11434".into());

    let prompt = format!(
        "You are the RedNode-OS planner. Given a user's intent, select \
         the right tools to fulfill it.\n\n\
         {}\n\n\
         User intent: \"{}\"\n\n\
         Respond with ONLY a JSON array of steps. Each step:\n\
         {{\"tool\": \"tool.name\", \"agent\": \"agent-name\", \
         \"args\": {{}}, \"risk\": \"low|medium|high|critical\"}}\n\n\
         Rules:\n\
         - Use the minimum number of steps needed\n\
         - Order steps logically\n\
         - Only use tools from the list above\n\
         - If unsure, use research.query\n\n\
         JSON array only, no explanation:",
        TOOL_CONTEXT, intent
    );

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()?;

    let model = std::env::var("REDNODE_MODEL")
        .unwrap_or("qwen2.5:7b-instruct-q4_K_M".into());

    let resp = client.post(format!("{}/api/generate", ollama_url))
        .json(&serde_json::json!({
            "model": model,
            "prompt": prompt,
            "stream": false,
            "options": {
                "temperature": 0.1,
                "num_predict": 512
            }
        }))
        .send()
        .await?;

    let body: serde_json::Value = resp.json().await?;
    let response_text = body["response"].as_str().unwrap_or("[]");

    // Extract JSON from response (LLM might wrap it in markdown)
    let json_str = extract_json_array(response_text);
    let steps: Vec<PlanStep> = serde_json::from_str(&json_str)?;

    tracing::info!(
        intent, steps = steps.len(),
        "LLM planner produced {} steps", steps.len()
    );
    Ok(steps)
}

fn extract_json_array(text: &str) -> String {
    // Find the first [ and last ] to extract JSON array
    if let Some(start) = text.find('[') {
        if let Some(end) = text.rfind(']') {
            return text[start..=end].to_string();
        }
    }
    "[]".to_string()
}

fn plan_keyword_fallback(intent: &str) -> Vec<PlanStep> {
    // Your existing keyword matching — kept as fallback
    let s = intent.to_lowercase();
    if s.contains("ssh") && s.contains("harden") {
        return vec![
            PlanStep { tool: "sec.ssh_audit".into(), agent: "security-agent".into(),
                       args: serde_json::json!({}), risk: Risk::Medium },
            PlanStep { tool: "sec.harden_ssh".into(), agent: "security-agent".into(),
                       args: serde_json::json!({}), risk: Risk::High },
        ];
    }
    if s.contains("docker") || s.contains("system") || s.contains("health") {
        return vec![
            PlanStep { tool: "process.list".into(), agent: "system-agent".into(),
                       args: serde_json::json!({}), risk: Risk::Low },
            PlanStep { tool: "docker.ps".into(), agent: "system-agent".into(),
                       args: serde_json::json!({}), risk: Risk::Low },
        ];
    }
    if s.contains("network") || s.contains("firewall") {
        return vec![PlanStep { tool: "net.status".into(), agent: "network-agent".into(),
                               args: serde_json::json!({}), risk: Risk::Low }];
    }
    vec![PlanStep { tool: "research.query".into(), agent: "research-agent".into(),
                    args: serde_json::json!({"query": intent}), risk: Risk::Low }]
}
```

```bash
# After editing, rebuild and test:
cd core/rednode-core
cargo build

# Start CNS and test with a natural intent:
cargo run &

curl -X POST http://localhost:8787/intent \
  -H "Content-Type: application/json" \
  -d '{"intent":"check if my SSH is secure, list running containers, and show network connections"}'

# The LLM should now return a 3-step plan:
# [sec.ssh_audit, docker.ps, net.status]
# Instead of falling through to research.query!
```

---

## Step 9: Fix the Bus (Day 4 — 15 minutes)

Quick safety fix — replace `unsafe static mut` with proper Rust:

```bash
# Edit: core/rednode-core/src/bus.rs
# Replace `static mut BUS` with:
#   static BUS: tokio::sync::OnceCell<Bus> = tokio::sync::OnceCell::const_new();
# Use BUS.get() instead of unsafe blocks
# This takes 15 minutes and eliminates undefined behavior
```

---

## Step 10: Wire Sentience to Real Data (Day 5 — 2 hours)

```bash
# Edit: core/rednode-core/src/sentience.rs

# Line 144: Replace hardcoded security_score = 0.9 with:
#   Query: SELECT COUNT(*) FROM security_events
#          WHERE ts > now() - interval '1 hour'
#          AND acknowledged = false
#   Score: 1.0 - (count * 0.1), clamped to 0.0..1.0

# Line 151: Replace knowledge = 0.75 with:
#   Query Qdrant collection count
#   Score: min(count / 100.0, 1.0)

# Line 214: UNCOMMENT the goal execution:
#   crate::coordinator::coordinate(&g.description, "sentience").await;
#   But ONLY for Low/Medium risk goals — High/Critical create approvals

# Line 280: Replace disk_used_gb: 42 with:
#   Read from statvfs or df command
```

---

## What You Should Have After 5 Days

```
Day 1: NixOS installed, GPU working, models pulled, infra running
Day 2: CNS + agents + web UI running, network configured
Day 3: VLANs live, Pi-hole serving DNS, cameras isolated
Day 4: LLM planner working (!!), bus fixed
Day 5: Sentience wired to real data, goals auto-executing

YOU CAN NOW:
  → Type any natural language intent and get an intelligent plan
  → See real CPU/RAM/disk in the Sentience panel
  → Watch RedNode generate and execute its own maintenance goals
  → View all activity in the hash-chained audit log
  → Access everything from your phone via WireGuard
  → All DNS goes through Pi-hole (ads blocked everywhere)
  → Cameras have zero internet access
  → Pi-hole, TrueNAS, cameras all isolated on proper VLANs
```

---

## After Day 5: What's Next (Priority Order)

```
Week 2:
  □ Flesh out System Agent handleTool (format output, detect unhealthy containers)
  □ Flesh out Network Agent (parse ss output, return structured JSON)
  □ Make Coding Agent actually run clippy/eslint
  □ Connect Research Agent to RAG pipeline (call /memory/query)
  □ WebSocket: forward NATS events to browser (replace the "hello" stub)

Week 3:
  □ Infrastructure Agent (NEW) → Pi-hole API integration
  □ Storage Agent (NEW) → TrueNAS API integration
  □ Wire both into Sentience Engine drives
  □ Nightly backup of RedNode brain to TrueNAS

Week 4-5:
  □ Frigate NVR deployment (Docker)
  □ Surveillance Agent (NEW) → Frigate API + MQTT
  □ Camera alerts → Security events → Mobile push

Week 6:
  □ Automation Agent workflows (goodnight, focus, morning brief)
  □ Voice: real Whisper STT + Piper TTS

Week 7+:
  □ Communications Agent (email/calendar)
  □ Productivity Agent (notes/tasks)
  □ Browser Agent (SearXNG)
```

---

## Quick Reference: Commands You'll Use Daily

```bash
# Start everything (order matters):
docker compose -f deployment/docker-compose.yml up -d   # infra
cd core/rednode-core && cargo run &                      # CNS
cd ../.. && pnpm agents &                                # agents
pnpm web &                                               # dashboard

# Test an intent:
curl -s -X POST http://localhost:8787/intent \
  -H "Content-Type: application/json" \
  -d '{"intent":"YOUR INTENT HERE"}' | jq .

# Check sentience:
curl -s http://localhost:8787/sentience | jq .model.drives

# Check audit log:
curl -s http://localhost:8787/audit?limit=5 | jq .entries

# Check agents:
curl -s http://localhost:8787/agents/status | jq .

# Search memory:
curl -s "http://localhost:8787/memory/query?q=YOUR+QUERY" | jq .

# Ingest a document:
curl -X POST http://localhost:8787/memory/ingest \
  -H "Content-Type: application/json" \
  -d '{"source":"manual","content":"Your knowledge here"}'
```

---

*Stop reading. Start doing. Pick the PC. Install NixOS. Day 1 is today.*
