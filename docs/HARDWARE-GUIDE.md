# RedNode-OS — Hardware Architecture Decision

> **Your situation**: Budget under $500 • Have an old PC + GPU • Maximum uptime required (family depends on it) • 15-30 devices • Already have TrueNAS + NVR

---

## The Verdict: Hybrid Approach

**Neither "everything on Proxmox" nor "all Raspberry Pis" — use a split architecture.**

```
╔═══════════════════════════════════════════════════════════════════╗
║                                                                   ║
║   $30 ─── Raspberry Pi Zero 2W ──── pfSense? NO. Pi-hole? YES.  ║
║                                                                   ║
║   $50 ─── Mini-PC (used) ────────── pfSense (bare metal)        ║
║                                                                   ║
║   $0  ─── Your old PC + GPU ─────── RedNode-OS (bare metal      ║
║                                      NixOS, NOT Proxmox)         ║
║                                                                   ║
║   $0  ─── Existing TrueNAS ──────── Storage (keep as-is)        ║
║                                                                   ║
║   $0  ─── Existing NVR ──────────── Cameras (keep as-is)        ║
║                                                                   ║
║   Total new spend: ~$80                                          ║
║                                                                   ║
╚═══════════════════════════════════════════════════════════════════╝
```

---

## Why NOT "Everything on Proxmox"

I know it's tempting — one box, everything virtualized, clean and tidy. But given your **"mission critical / family depends on this"** requirement, here's why it's the wrong call:

### The Single Point of Failure Problem

```
                    ┌──────────────────────┐
                    │    PROXMOX HOST       │
                    │                       │
                    │  ┌─────────────────┐  │
                    │  │ pfSense VM      │  │ ← Your firewall
                    │  └─────────────────┘  │
                    │  ┌─────────────────┐  │
                    │  │ Pi-hole CT      │  │ ← Your DNS
                    │  └─────────────────┘  │
                    │  ┌─────────────────┐  │
                    │  │ RedNode VM      │  │ ← Your brain
                    │  │ (GPU passthru)  │  │
                    │  └─────────────────┘  │
                    │                       │
                    └──────────┬────────────┘
                               │
                    Proxmox needs a kernel update?
                    Power supply fails?
                    RAM goes bad?
                    Bad NVIDIA driver update?
                               │
                               ▼
                    ██████████████████████
                    ██  EVERYTHING DIES ██
                    ██                  ██
                    ██  No internet     ██
                    ██  No DNS          ██
                    ██  No firewall     ██
                    ██  No cameras      ██
                    ██  No RedNode      ██
                    ██  Family upset    ██
                    ██████████████████████
```

### Specific Risks of Proxmox + pfSense VM

| Risk | Impact | Likelihood |
|---|---|---|
| Proxmox kernel update breaks NVIDIA driver | GPU passthrough fails → RedNode/Frigate down. But worse: if Proxmox crashes during update, pfSense VM dies → **no internet** | Medium — happens regularly |
| GPU passthrough IOMMU issues | NVIDIA driver conflicts on host can freeze the entire hypervisor, not just the VM | Medium — well-documented problem |
| Proxmox storage corruption | ALL VMs gone simultaneously | Low but catastrophic |
| Need to debug Proxmox over SSH | But your firewall IS a VM on Proxmox — chicken-and-egg: you need internet to fix the box that provides internet | This WILL happen eventually |
| Power supply / hardware failure | Single box = single failure domain for everything | Your old PC is... old |
| RAM failure | One bad DIMM can take down the host + all VMs | Higher on older hardware |

### The Damning Quote from the Community

> *"I strongly encourage you to run pfSense on dedicated hardware, not as a VM. If your machine hosting the VM goes down (bugs or maintenance) you lose internet, which might be crucial in getting the hosting machine back up."* — top-voted advice on r/pfSense

And the counterpoint from someone who's done it for years:

> *"I have been running it for some time without issue."* — but these people accept periodic outages as part of the deal.

**You said your family depends on this. Periodic outages are not acceptable.**

---

## Why NOT Two Raspberry Pis for pfSense + Pi-hole

### pfSense on a Raspberry Pi: Doesn't Work

pfSense requires **x86_64 hardware**. It literally cannot run on a Raspberry Pi (ARM). Full stop.

You *could* run OPNsense on ARM, but it's experimental and not production-ready for a family-critical network.

### Pi-hole on a Raspberry Pi: ✅ Perfect

Pi-hole is *designed* for Raspberry Pi. A $15 Pi Zero 2W handles DNS for 30+ devices effortlessly. It's the ideal deployment.

---

## The Correct Architecture: Split by Failure Domain

### Principle: Things that must NEVER go down → dedicated hardware. Things that CAN briefly go down → can share hardware.

```
┌─────────────────────────────────────────────────────────────┐
│                                                             │
│  FAILURE DOMAIN 1: NETWORK (must NEVER go down)            │
│  ─────────────────────────────────────────────              │
│                                                             │
│  ┌──────────────────────┐   ┌────────────────────┐         │
│  │ DEVICE 1             │   │ DEVICE 2           │         │
│  │ Mini-PC (used)       │   │ Raspberry Pi       │         │
│  │                      │   │ Zero 2W            │         │
│  │ pfSense              │   │                    │         │
│  │ (bare metal)         │   │ Pi-hole            │         │
│  │                      │   │ (bare metal)       │         │
│  │ Cost: ~$50 used      │   │ Cost: ~$30 new     │         │
│  │ Power: ~10W          │   │ Power: ~2W         │         │
│  │ Boots in: 30s        │   │ Boots in: 20s      │         │
│  │                      │   │                    │         │
│  │ If this dies:        │   │ If this dies:      │         │
│  │ → No internet        │   │ → Devices fallback │         │
│  │ → Keep ISP router    │   │   to pfSense DNS   │         │
│  │   as emergency       │   │   (no ad-blocking  │         │
│  │   backup             │   │   but internet     │         │
│  │                      │   │   still works)     │         │
│  └──────────────────────┘   └────────────────────┘         │
│                                                             │
│  These two devices have ONE job each.                      │
│  They don't get updated often.                             │
│  They don't run experiments.                               │
│  They just WORK, 24/7/365.                                │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  FAILURE DOMAIN 2: INTELLIGENCE (can briefly restart)      │
│  ────────────────────────────────────────────────           │
│                                                             │
│  ┌──────────────────────────────────────────────┐          │
│  │ DEVICE 3                                     │          │
│  │ Your Old PC + GPU                            │          │
│  │                                               │          │
│  │ RedNode-OS (NixOS bare metal — NOT Proxmox)  │          │
│  │                                               │          │
│  │ Running:                                      │          │
│  │   • RedNode CNS (Rust)         :8787         │          │
│  │   • Frigate NVR (Docker)       :5000         │          │
│  │   • Ollama (GPU-accelerated)   :11434        │          │
│  │   • PostgreSQL                 :5432         │          │
│  │   • Qdrant                     :6333         │          │
│  │   • NATS JetStream             :4222         │          │
│  │   • Next.js Dashboard          :3000         │          │
│  │   • Grafana                    :3001         │          │
│  │   • MQTT Broker                :1883         │          │
│  │   • Agent Society (6 agents)                 │          │
│  │                                               │          │
│  │ GPU: NVIDIA → Ollama (LLM) + Frigate (AI)   │          │
│  │ Cost: $0 (your existing hardware)            │          │
│  │                                               │          │
│  │ If this reboots/dies:                         │          │
│  │ → Internet still works (pfSense is separate) │          │
│  │ → DNS still works (Pi-hole is separate)      │          │
│  │ → Cameras still record (NVR is separate)     │          │
│  │ → Files still accessible (TrueNAS separate)  │          │
│  │ → Only RedNode intelligence is offline       │          │
│  │ → You temporarily lose: dashboard, AI alerts,│          │
│  │   smart camera detection, automations        │          │
│  │ → Everything comes back on reboot             │          │
│  └──────────────────────────────────────────────┘          │
│                                                             │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  FAILURE DOMAIN 3: STORAGE (you already have this)         │
│  ─────────────────────────────────────────────              │
│                                                             │
│  ┌──────────────────────┐   ┌────────────────────┐         │
│  │ DEVICE 4             │   │ DEVICE 5           │         │
│  │ TrueNAS              │   │ NVR + Cameras      │         │
│  │ (existing)           │   │ (existing)         │         │
│  │                      │   │                    │         │
│  │ If this dies:        │   │ If this dies:      │         │
│  │ → No file shares     │   │ → No camera        │         │
│  │ → Frigate recordings │   │   recording        │         │
│  │   buffer locally on  │   │ → Frigate still    │         │
│  │   RedNode SSD        │   │   works (pulls     │         │
│  │                      │   │   RTSP directly    │         │
│  │                      │   │   from cameras)    │         │
│  └──────────────────────┘   └────────────────────┘         │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

---

## Why NixOS Bare Metal Instead of Proxmox for RedNode

RedNode-OS was **designed** to be a bare-metal OS. Putting it in a Proxmox VM adds:

| Overhead | Impact |
|---|---|
| Hypervisor layer | 5-10% CPU/RAM overhead for no benefit |
| GPU passthrough complexity | IOMMU, vfio-pci, driver matching — constant maintenance |
| Two things to update | Proxmox host + VM OS — double the attack surface |
| Can't be PID1 | RedNode's Phase 5 vision is to BE the init system — impossible in a VM |
| Snapshot management | You get Proxmox snapshots but lose NixOS atomic rollbacks (which are better) |
| Network complexity | Virtual bridges, NAT layers — unnecessary when it owns the hardware |

NixOS already gives you everything Proxmox offers:
- **Atomic rollbacks** → `nixos-rebuild switch` — better than VM snapshots
- **Reproducible builds** → entire OS from `.nix` files
- **Docker support** → Frigate, Postgres, Qdrant all run in Docker on NixOS
- **GPU direct access** → no passthrough needed, NVIDIA drivers installed natively

---

## The Complete Physical Setup

```
YOUR RACK / SHELF
═══════════════════════════════════════════════════════

 ┌─────────────────────────────────────────────────┐
 │ ISP Router (bridge mode)                        │
 │ Just a modem — passes public IP through         │
 │ Power: wall adapter                             │
 └───────────────────┬─────────────────────────────┘
                     │ Ethernet
 ┌───────────────────▼─────────────────────────────┐
 │ pfSense — Dedicated Mini-PC              ~$50   │
 │                                                  │
 │ Hardware options (used):                        │
 │   • Dell Wyse 5070 (~$30-40 used)              │
 │   • HP ProDesk 400 G4 Mini (~$40-50)           │
 │   • Lenovo ThinkCentre M720q (~$50)            │
 │   • Any mini-PC with 2+ ethernet ports         │
 │   • Or: single NIC + managed switch (VLAN trunk)│
 │                                                  │
 │ Specs needed: Any x86_64, 2GB RAM, 16GB SSD    │
 │ Power: ~10W                                     │
 │                                                  │
 │ Connections:                                    │
 │   WAN port ← ISP router                        │
 │   LAN port → managed switch (trunk)            │
 └───────────────────┬─────────────────────────────┘
                     │ Trunk (all VLANs)
 ┌───────────────────▼─────────────────────────────┐
 │ Managed Switch (VLAN-aware)          (existing?) │
 │                                                  │
 │ If you don't have one: TP-Link TL-SG108E ~$30  │
 │ Or: Netgear GS308E ~$35                        │
 │                                                  │
 │ Port assignments:                               │
 │   Port 1:  Trunk ← pfSense                     │
 │   Port 2:  VLAN 50 — RedNode server             │
 │   Port 3:  VLAN 50 — TrueNAS                   │
 │   Port 4:  VLAN 50 — Pi-hole (Raspberry Pi)    │
 │   Port 5:  VLAN 30 — NVR                       │
 │   Port 6:  VLAN 10 — Your workstation          │
 │   Port 7:  VLAN 10 — WiFi AP (trusted SSID)    │
 │   Port 8:  VLAN 20 — WiFi AP (IoT SSID)        │
 └─────────────────────────────────────────────────┘
       │       │       │       │       │
 ┌─────▼──┐ ┌─▼────┐ ┌▼─────┐ ┌▼────┐ ┌▼─────┐
 │RedNode │ │True- │ │Pi-   │ │NVR  │ │WiFi  │
 │Server  │ │NAS   │ │hole  │ │+cams│ │AP(s) │
 │        │ │      │ │(RPi) │ │     │ │      │
 │VLAN 50 │ │VL 50 │ │VL 50 │ │VL30 │ │VL10/ │
 │        │ │      │ │      │ │     │ │20/40 │
 └────────┘ └──────┘ └──────┘ └─────┘ └──────┘

 ┌─────────────────────────────────────────────────┐
 │ UPS (Uninterruptible Power Supply)              │
 │                                                  │
 │ CyberPower CP1500AVRLCD (~$160) or similar     │
 │ Powers: pfSense, switch, RedNode, TrueNAS,     │
 │         NVR, Pi-hole                            │
 │ USB to RedNode for monitoring                   │
 │ Runtime: ~15-30 min at your load                │
 │                                                  │
 │ ⚠️  If you don't have a UPS, THIS is where     │
 │    your $500 budget should go first.            │
 │    Nothing else matters if a power flicker      │
 │    corrupts your TrueNAS pool.                  │
 └─────────────────────────────────────────────────┘

═══════════════════════════════════════════════════════
```

---

## Budget Breakdown

| Item | Cost | Notes |
|---|---|---|
| pfSense mini-PC (used) | ~$40-50 | Dell Wyse 5070 / HP Mini / Lenovo M720q from eBay |
| Raspberry Pi Zero 2W + case + SD card | ~$25-30 | For Pi-hole |
| Managed switch (if you don't have one) | ~$30-35 | TP-Link TL-SG108E (8-port, VLAN-aware) |
| Coral USB Accelerator (optional) | ~$60 | For Frigate AI detection — not required if you have GPU |
| RedNode server | **$0** | Your old PC + existing GPU |
| TrueNAS | **$0** | Already have it |
| NVR + cameras | **$0** | Already have it |
| UPS (if you don't have one) | ~$150-170 | **Highest priority purchase** |
| **TOTAL** | **$95-$150** | (or ~$250-320 if you need switch + UPS) |

**Well under $500.** And you could even skip the Coral USB if your GPU handles both Ollama and Frigate (it can — see below).

---

## Your Old PC as the RedNode Server — What It Needs

| Component | Minimum | Ideal |
|---|---|---|
| CPU | Any 4-core x86_64 (Intel i5 4th gen+, AMD Ryzen) | 6+ cores |
| RAM | 16 GB | 32 GB (if available) |
| Boot drive | 120 GB SSD | 500 GB NVMe |
| GPU | Any NVIDIA with 6GB+ VRAM | 8GB+ VRAM (RTX 3060 12GB is perfect) |
| Network | 1 Gbps NIC | 2.5 Gbps |

### How the GPU Gets Shared

On NixOS bare metal (not Proxmox!), the GPU is available to ALL services simultaneously — no passthrough tricks needed:

```
NVIDIA GPU (your existing card)
    │
    ├── Ollama (LLM inference)
    │     Uses: ~4-8 GB VRAM depending on model
    │     Qwen2.5-7B q4: ~5 GB VRAM
    │     Qwen2.5-14B q4: ~9 GB VRAM
    │
    ├── Frigate (AI object detection via TensorRT)
    │     Uses: ~500 MB - 1 GB VRAM
    │     Detects: person, car, animal, package
    │
    └── Both run simultaneously — NVIDIA handles VRAM sharing
        No IOMMU, no passthrough, no hypervisor needed
        Just install nvidia-docker and go
```

If your GPU has 6 GB VRAM → run Qwen2.5-7B + Frigate (fits)
If your GPU has 8 GB VRAM → comfortable with both
If your GPU has 12 GB VRAM → run Qwen2.5-14B + Frigate with room to spare

---

## What Gets Installed Where — Final Summary

| Device | OS | Services | VLAN | IP |
|---|---|---|---|---|
| **pfSense mini-PC** | pfSense (bare metal) | Firewall, NAT, DHCP, VLAN routing, VPN server | WAN + all VLANs | 10.0.x.1 per VLAN |
| **Raspberry Pi** | Raspberry Pi OS Lite | Pi-hole DNS | VLAN 50 | 10.0.50.2 |
| **Your old PC** | NixOS (RedNode-OS, bare metal) | CNS, Frigate, Ollama, Postgres, Qdrant, NATS, Web UI, Grafana, MQTT, all agents | VLAN 50 | 10.0.50.10 |
| **TrueNAS** | TrueNAS (existing) | File storage, SMB/NFS, backups | VLAN 50 | 10.0.50.3 |
| **NVR** | NVR firmware (existing) | Camera recording (backup to Frigate) | VLAN 30 | 10.0.30.2 |
| **Cameras** | Camera firmware | Video streams | VLAN 30 | 10.0.30.10+ |

---

## The Resilience Matrix — What Breaks When Something Dies

```
                    pfSense  Pi-hole  RedNode  TrueNAS  NVR
                    dies     dies     dies     dies     dies
                    ─────    ─────    ─────    ─────    ─────
Internet            ❌ DOWN   ✅ OK    ✅ OK    ✅ OK    ✅ OK
DNS (ad-blocking)   ✅ OK*   ❌ DOWN† ✅ OK    ✅ OK    ✅ OK
Firewall/VLANs      ❌ DOWN   ✅ OK    ✅ OK    ✅ OK    ✅ OK
Camera recording    ✅ OK     ✅ OK    ✅ OK    ✅ OK    ❌ DOWN
AI camera alerts    ✅ OK     ✅ OK    ❌ DOWN   ✅ OK    ✅ OK‡
File shares (SMB)   ✅ OK     ✅ OK    ✅ OK    ❌ DOWN   ✅ OK
RedNode dashboard   ✅ OK     ✅ OK    ❌ DOWN   ✅ OK    ✅ OK
LLM / AI features   ✅ OK     ✅ OK    ❌ DOWN   ✅ OK    ✅ OK
Smart automations   ✅ OK     ✅ OK    ❌ DOWN   ✅ OK    ✅ OK

 * pfSense has basic DNS resolver — works without Pi-hole
 † Devices fallback to pfSense DNS (no ad-blocking, but internet works)
 ‡ Cameras still stream to Frigate on RedNode (NVR is backup only)

KEY INSIGHT: No single device failure takes down more than
one or two capabilities. This is impossible with "everything
on Proxmox" — where ONE failure = EVERYTHING down.
```

---

*Three devices handle your critical infrastructure (pfSense mini-PC, Pi Zero, your old PC). Each can fail independently without killing the others. Total new spend: ~$80-150. Your family's internet, cameras, and files survive any single hardware failure.*
