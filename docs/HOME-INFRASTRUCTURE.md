# RedNode-OS — Home Infrastructure Integration Guide

> **Your existing stack**: Standalone NVR (CCTV) • Pi-hole (DNS) • TrueNAS (File Storage) • Full homelab network (VLANs, managed switches, dedicated firewall)

---

## Your Current Home Infrastructure Map

```
                           ┌──────────────┐
                           │   INTERNET    │
                           └──────┬───────┘
                                  │
                           ┌──────▼───────┐
                           │   FIREWALL    │  pfSense / OPNsense
                           │  (gateway)    │
                           └──────┬───────┘
                                  │
                    ┌─────────────┼─────────────┐
                    │             │              │
              ┌─────▼─────┐ ┌────▼────┐  ┌──────▼──────┐
              │  VLAN 10   │ │ VLAN 20 │  │   VLAN 30   │
              │  Trusted   │ │   IoT   │  │  Cameras    │
              │ (devices)  │ │(smart)  │  │  (NVR+cams) │
              └─────┬──────┘ └────┬────┘  └──────┬──────┘
                    │             │              │
        ┌───────┬──┘    ┌────────┘        ┌─────┘
        │       │       │                 │
   ┌────▼───┐ ┌▼────┐ ┌▼─────┐    ┌──────▼──────┐
   │RedNode │ │True- │ │Pi-   │    │ Standalone  │
   │  OS    │ │NAS   │ │hole  │    │    NVR      │
   │ server │ │      │ │      │    │ + cameras   │
   └────────┘ └──────┘ └──────┘    └─────────────┘
```

---

## The Integration Architecture

RedNode-OS sits at the **center** — it becomes the brain that observes, manages, and automates your entire home infrastructure through APIs. It doesn't replace anything — it **orchestrates everything**.

```
┌─────────────────────────────────────────────────────────────────┐
│                      RedNode-OS (CNS)                           │
│                    The Central Brain                             │
│─────────────────────────────────────────────────────────────────│
│                                                                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐            │
│  │Infrastructure│  │ Surveillance│  │  Storage    │            │
│  │   Agent     │  │   Agent     │  │  Agent      │            │
│  │             │  │             │  │             │            │
│  │ • Pi-hole   │  │ • NVR/RTSP  │  │ • TrueNAS   │            │
│  │ • Firewall  │  │ • Frigate   │  │ • Snapshots │            │
│  │ • VLANs     │  │ • Alerts    │  │ • Health    │            │
│  │ • UPS       │  │ • Zones     │  │ • Shares    │            │
│  └──────┬──────┘  └──────┬──────┘  └──────┬──────┘            │
│         │                │                │                    │
│         │     NATS JetStream Bus          │                    │
│─────────┼────────────────┼────────────────┼────────────────────│
│         │                │                │                    │
│  ┌──────▼──────┐  ┌──────▼──────┐  ┌──────▼──────┐            │
│  │ Existing    │  │ Existing    │  │ Existing    │            │
│  │ Network     │  │ Security    │  │ System      │            │
│  │ Agent       │  │ Agent       │  │ Agent       │            │
│  └─────────────┘  └─────────────┘  └─────────────┘            │
└─────────────────────────────────────────────────────────────────┘
         │                │                │
         ▼                ▼                ▼
    ┌─────────┐    ┌───────────┐    ┌───────────┐
    │ Pi-hole │    │    NVR    │    │  TrueNAS  │
    │ API v6  │    │  (RTSP +  │    │ REST API  │
    │         │    │  ONVIF)   │    │ v2.0      │
    └─────────┘    └───────────┘    └───────────┘
```

---

## 1. Pi-hole Integration — DNS Intelligence

### What RedNode Does With Pi-hole

Your Pi-hole already blocks ads and trackers at the DNS level. RedNode turns it from a passive blocker into an **active network intelligence layer**:

| Capability | How |
|---|---|
| **DNS analytics in your dashboard** | Pull query stats, top blocked domains, top clients → show in RedNode Security Feed |
| **Threat detection** | If a device starts making unusual DNS queries (malware C2 callbacks), RedNode flags it as a security event |
| **Dynamic blocking** | "Block TikTok on all devices from 9pm to 7am" → RedNode creates/removes Pi-hole blocklists on schedule |
| **Per-device policies** | "Block social media on my work laptop during focus hours" → Pi-hole group management via API |
| **Disable on demand** | "Pause Pi-hole for 5 minutes" → when a site is over-blocked |
| **Network audit** | "Which device is making the most DNS queries?" → anomaly detection |

### Pi-hole v6 API Integration

Pi-hole v6 has a full REST API at `http://<pihole-ip>/api/`:

```
# Authentication (v6 uses session-based auth)
POST /api/auth                    → {"password": "your_password"}
                                  → returns {session: {sid: "..."}}

# Stats
GET  /api/stats/summary?sid=...   → total queries, blocked, percentage
GET  /api/stats/top_domains       → most queried domains
GET  /api/stats/top_blocked       → most blocked domains
GET  /api/stats/top_clients       → most active devices

# Control
POST /api/dns/blocking?sid=...    → {"blocking": false, "timer": 300}
                                  → disable for 5 minutes

# Lists
GET  /api/lists                   → all blocklists
POST /api/lists                   → add blocklist
DELETE /api/lists/:id             → remove blocklist

# Groups / Per-device
GET  /api/groups                  → device groups
POST /api/groups                  → create group with policies
```

### New Tools for Infrastructure Agent

```json
{"name": "pihole.stats",       "agent": "infra-agent", "risk": "low"},
{"name": "pihole.top_blocked", "agent": "infra-agent", "risk": "low"},
{"name": "pihole.top_clients", "agent": "infra-agent", "risk": "low"},
{"name": "pihole.query_log",   "agent": "infra-agent", "risk": "low"},
{"name": "pihole.disable",     "agent": "infra-agent", "risk": "medium"},
{"name": "pihole.enable",      "agent": "infra-agent", "risk": "low"},
{"name": "pihole.add_block",   "agent": "infra-agent", "risk": "medium"},
{"name": "pihole.remove_block","agent": "infra-agent", "risk": "medium"},
{"name": "pihole.anomaly",     "agent": "infra-agent", "risk": "low"}
```

### What You Can Say to RedNode

```
"Show me DNS stats for today"
"Which device is making the most queries?"
"Block TikTok on all devices"
"Pause Pi-hole for 5 minutes"
"Are any devices making suspicious DNS queries?"
"Show me what Pi-hole blocked this week"
"Set up a focus mode — block social media on my laptop from 9am to 5pm"
```

---

## 2. TrueNAS Integration — Storage Intelligence

### What RedNode Does With TrueNAS

Your TrueNAS handles file storage and shares. RedNode turns it into **managed intelligent storage**:

| Capability | How |
|---|---|
| **Health monitoring** | Pool health, disk SMART status, temperature → Sentience Engine drives |
| **Storage alerts** | "Pool 85% full" → RedNode security event + proactive alert |
| **Automated snapshots** | "Snapshot my documents every 6 hours" → TrueNAS API |
| **Snapshot before risky operations** | Before any High/Critical RedNode action → auto-snapshot TrueNAS datasets |
| **Backup verification** | "Are my backups current?" → check last snapshot timestamps |
| **Share management** | "Create a new share for the project folder" → SMB/NFS via API |
| **Storage reporting** | "How much space am I using?" → dataset usage breakdown |
| **Replication monitoring** | If you replicate to offsite → monitor replication health |
| **RedNode memory backup** | Auto-backup RedNode's Postgres/Qdrant data to TrueNAS nightly |

### TrueNAS REST API v2.0 Integration

TrueNAS has a comprehensive REST API:

```bash
# Authentication — Bearer token (create API key in TrueNAS UI)
curl -k -H "Authorization: Bearer <API-KEY>" \
  https://truenas-ip/api/v2.0/system/info

# System Health
GET  /api/v2.0/system/info          → hostname, version, uptime
GET  /api/v2.0/pool                  → all pools, health status
GET  /api/v2.0/pool/dataset          → all datasets with usage
GET  /api/v2.0/disk                  → all disks, SMART status
GET  /api/v2.0/alert/list            → active alerts

# Snapshots
GET  /api/v2.0/zfs/snapshot          → list all snapshots
POST /api/v2.0/zfs/snapshot          → create snapshot
     {"dataset": "tank/documents", "name": "rednode-auto-20260613", "recursive": true}
DELETE /api/v2.0/zfs/snapshot/id/... → delete old snapshots

# Datasets
GET  /api/v2.0/pool/dataset/id/tank%2Fdocuments  → dataset info
PUT  /api/v2.0/pool/dataset/id/...               → update quota, compression
POST /api/v2.0/pool/dataset                      → create new dataset

# Shares
GET  /api/v2.0/sharing/smb           → list SMB shares
POST /api/v2.0/sharing/smb           → create SMB share
GET  /api/v2.0/sharing/nfs           → list NFS exports

# Replication
GET  /api/v2.0/replication           → replication jobs
POST /api/v2.0/replication/run       → trigger replication

# SMART
GET  /api/v2.0/smart/test/results    → disk SMART test results
POST /api/v2.0/smart/test            → run SMART test
```

### New Tools for Storage Agent

```json
{"name": "nas.health",          "agent": "storage-agent", "risk": "low"},
{"name": "nas.pools",           "agent": "storage-agent", "risk": "low"},
{"name": "nas.datasets",        "agent": "storage-agent", "risk": "low"},
{"name": "nas.usage",           "agent": "storage-agent", "risk": "low"},
{"name": "nas.disks",           "agent": "storage-agent", "risk": "low"},
{"name": "nas.smart",           "agent": "storage-agent", "risk": "low"},
{"name": "nas.alerts",          "agent": "storage-agent", "risk": "low"},
{"name": "nas.snapshot_create", "agent": "storage-agent", "risk": "medium"},
{"name": "nas.snapshot_list",   "agent": "storage-agent", "risk": "low"},
{"name": "nas.snapshot_delete", "agent": "storage-agent", "risk": "high"},
{"name": "nas.share_create",    "agent": "storage-agent", "risk": "medium"},
{"name": "nas.share_list",      "agent": "storage-agent", "risk": "low"},
{"name": "nas.replicate",       "agent": "storage-agent", "risk": "medium"},
{"name": "nas.backup_rednode",  "agent": "storage-agent", "risk": "medium"}
```

### What You Can Say to RedNode

```
"How healthy are my storage pools?"
"Show disk status and temperatures"
"How much storage am I using?"
"Snapshot my documents dataset"
"When was the last backup?"
"Are any disks showing SMART warnings?"
"Create a new share called 'project-alpha'"
"Back up RedNode's memory to TrueNAS"
"Clean up snapshots older than 30 days"
"Alert me if any pool goes above 80%"
```

### RedNode ↔ TrueNAS Symbiosis

This is where it gets powerful — RedNode uses TrueNAS as its **safety net**:

```
RedNode Security Agent                    TrueNAS
        │                                    │
        │  1. CVE found — HIGH severity      │
        │                                    │
        │  2. BEFORE patching:               │
        ├───────── nas.snapshot_create ──────▶│  "pre-CVE-2024-1234"
        │                                    │
        │  3. Apply patch (sandboxed)        │
        │                                    │
        │  4a. Patch SUCCESS:                │
        │     Keep snapshot for 7 days       │
        │                                    │
        │  4b. Patch FAILED:                 │
        ├───────── rollback to snapshot ────▶│  Automatic restore
        │                                    │
```

---

## 3. CCTV / NVR Integration — Surveillance Intelligence

### The Challenge With Standalone NVRs

Standalone NVRs (Reolink, Hikvision, Dahua) are **closed boxes**. They record video, but they don't think. Most of them:
- Have basic motion detection (lots of false alarms — cats, shadows, wind)
- No AI object detection (person vs car vs animal)
- No integration APIs (or very limited ones)
- Can't alert RedNode about threats
- Can't be queried ("show me when a person was at the front door at 3pm")

### The Solution: Add Frigate NVR as an AI Layer

You keep your standalone NVR (it keeps recording as backup). You add **Frigate NVR** alongside it — Frigate connects to the same camera RTSP streams and adds AI intelligence on top.

```
                    Cameras (PoE)
                   ┌──┬──┬──┬──┐
                   │  │  │  │  │
          ┌────────┴──┴──┴──┴──┴────────┐
          │                              │
    ┌─────▼──────┐              ┌────────▼────────┐
    │ Standalone  │              │    Frigate NVR   │
    │    NVR      │              │  (Docker on      │
    │ (keeps      │              │   RedNode or     │
    │  recording  │              │   TrueNAS)       │
    │  as backup) │              │                  │
    └─────────────┘              │  • AI detection  │
                                 │  • Person/car/   │
                                 │    animal/pkg    │
                                 │  • Face recog    │
                                 │  • License plate │
                                 │  • Audio events  │
                                 │  • Zone alerts   │
                                 │  • MQTT events   │
                                 └────────┬─────────┘
                                          │ MQTT + REST API
                                          │
                                 ┌────────▼─────────┐
                                 │    RedNode-OS     │
                                 │  Surveillance     │
                                 │    Agent          │
                                 │                   │
                                 │  • Security events│
                                 │  • Smart alerts   │
                                 │  • Zone queries   │
                                 │  • Clip retrieval │
                                 │  • Anomaly detect │
                                 └───────────────────┘
```

### How Frigate Connects to Your Standalone NVR

Most standalone NVRs output RTSP streams. Frigate taps into those:

```yaml
# Frigate config.yml — connecting to NVR RTSP output
# Each camera channel on the NVR has its own RTSP stream

go2rtc:
  streams:
    front_door:
      - "rtsp://admin:password@NVR_IP:554/h264Preview_01_main"
    front_door_sub:
      - "rtsp://admin:password@NVR_IP:554/h264Preview_01_sub"
    driveway:
      - "rtsp://admin:password@NVR_IP:554/h264Preview_02_main"
    driveway_sub:
      - "rtsp://admin:password@NVR_IP:554/h264Preview_02_sub"
    backyard:
      - "rtsp://admin:password@NVR_IP:554/h264Preview_03_main"
    backyard_sub:
      - "rtsp://admin:password@NVR_IP:554/h264Preview_03_sub"
    # Channel numbers: 01, 02, 03... match NVR channel layout

mqtt:
  enabled: true
  host: rednode-server-ip  # or a dedicated MQTT broker
  user: frigate
  password: secure_password

detectors:
  # Option A: Google Coral USB ($60) — fastest
  coral:
    type: edgetpu
    device: usb
  # Option B: CPU-only (works, slower)
  # cpu:
  #   type: cpu
  # Option C: If RedNode has NVIDIA GPU
  # nvidia:
  #   type: tensorrt

cameras:
  front_door:
    enabled: true
    ffmpeg:
      inputs:
        - path: rtsp://127.0.0.1:8554/front_door
          input_args: preset-rtsp-restream
          roles: [record]
        - path: rtsp://127.0.0.1:8554/front_door_sub
          input_args: preset-rtsp-restream
          roles: [detect]
    detect:
      enabled: true
      width: 1280
      height: 720
      fps: 5
    objects:
      track: [person, car, dog, cat, package]
      filters:
        person:
          min_score: 0.65
          min_area: 1000
    zones:
      porch:
        coordinates: 100,400,300,400,300,600,100,600
        objects: [person, package]
    record:
      enabled: true
      retain:
        days: 7
        mode: motion
      events:
        retain:
          default: 30
          mode: active_objects

  driveway:
    enabled: true
    ffmpeg:
      inputs:
        - path: rtsp://127.0.0.1:8554/driveway
          input_args: preset-rtsp-restream
          roles: [record]
        - path: rtsp://127.0.0.1:8554/driveway_sub
          input_args: preset-rtsp-restream
          roles: [detect]
    detect:
      enabled: true
      width: 1280
      height: 720
      fps: 5
    objects:
      track: [person, car, motorcycle, bicycle]

  backyard:
    enabled: true
    ffmpeg:
      inputs:
        - path: rtsp://127.0.0.1:8554/backyard
          input_args: preset-rtsp-restream
          roles: [record]
        - path: rtsp://127.0.0.1:8554/backyard_sub
          input_args: preset-rtsp-restream
          roles: [detect]
    detect:
      enabled: true
      width: 1280
      height: 720
      fps: 5
    objects:
      track: [person, dog, cat]
```

### Where to Run Frigate

| Option | Pros | Cons |
|---|---|---|
| **On RedNode server** (Docker) | Closest to the brain, lowest latency for alerts | Uses RedNode's CPU/GPU resources |
| **On TrueNAS** (Docker/App) | Dedicated storage for recordings, TrueNAS has the disk | Less CPU headroom |
| **Dedicated mini-PC** | Isolated, can add Coral TPU | Another device to manage |

**Recommendation**: Run Frigate **on the RedNode server** in Docker — it keeps everything centralized. Add a **Coral USB Accelerator** (~$60) for fast AI inference without loading the CPU.

### Frigate API → RedNode Integration

Frigate has both an **MQTT event stream** and a **REST API**:

```
# MQTT Events (real-time — the primary integration path)
Topic: frigate/events
Payload: {
  "type": "new",
  "after": {
    "id": "1718000000.000000-abcdef",
    "camera": "front_door",
    "label": "person",
    "score": 0.89,
    "zone": "porch",
    "start_time": 1718000000.0,
    "has_clip": true,
    "has_snapshot": true
  }
}

# REST API
GET  /api/events          → list detection events (filterable)
GET  /api/events/:id/thumbnail.jpg  → event thumbnail
GET  /api/events/:id/clip.mp4       → event video clip
GET  /api/events/:id/snapshot.jpg   → full-res snapshot
GET  /api/stats            → camera stats, detection counts
GET  /api/:camera/latest.jpg        → latest frame from camera
POST /api/events/:id/retain         → keep event permanently
GET  /api/reviews          → AI-generated review summaries (v0.17+)
```

### New Tools for Surveillance Agent

```json
{"name": "cam.status",         "agent": "surveillance-agent", "risk": "low"},
{"name": "cam.live",           "agent": "surveillance-agent", "risk": "low"},
{"name": "cam.events",         "agent": "surveillance-agent", "risk": "low"},
{"name": "cam.snapshot",       "agent": "surveillance-agent", "risk": "low"},
{"name": "cam.clip",           "agent": "surveillance-agent", "risk": "low"},
{"name": "cam.search",         "agent": "surveillance-agent", "risk": "low"},
{"name": "cam.zones",          "agent": "surveillance-agent", "risk": "low"},
{"name": "cam.alert_config",   "agent": "surveillance-agent", "risk": "medium"},
{"name": "cam.person_detect",  "agent": "surveillance-agent", "risk": "low"},
{"name": "cam.anomaly",        "agent": "surveillance-agent", "risk": "low"},
{"name": "cam.retain_event",   "agent": "surveillance-agent", "risk": "low"},
{"name": "cam.review",         "agent": "surveillance-agent", "risk": "low"}
```

### What You Can Say to RedNode

```
"Show me who's at the front door"
"Was anyone in the driveway today?"
"Show me all person detections from last night"
"Pull up the clip from 3pm front door"
"How many cars came up the driveway today?"
"Alert me immediately if a person is in the backyard after 11pm"
"Show me a snapshot from all cameras right now"
"Any unusual activity today?"
"Who was at the porch at 2:30pm? Show me the clip"
"Set up a zone alert for packages on the porch"
```

### The Smart Security Loop

This is where everything connects — Frigate + Pi-hole + RedNode Security Agent:

```
Frigate detects person ──────► MQTT ──────► RedNode Surveillance Agent
at back door at 2am                                    │
                                                       ▼
                                              RedNode Security Agent
                                              "CRITICAL: Person at back
                                               door, unusual hour"
                                                       │
                                        ┌──────────────┼──────────────┐
                                        ▼              ▼              ▼
                                  Push to your    Log security    Auto-snapshot
                                  phone (FCM)     event + audit   TrueNAS
                                  with snapshot   hash-chained    (preserve evidence)
                                  + biometric                   
                                  unlock to view                

Meanwhile, Pi-hole notices ────► RedNode Infrastructure Agent
a device making DNS queries                    │
to known C2 domains                            ▼
                                      "CRITICAL: Device 192.168.1.45
                                       querying C2 domain botnet.xyz"
                                                │
                                        ┌───────┼───────┐
                                        ▼       ▼       ▼
                                   Block via  Alert   Isolate device
                                   Pi-hole    owner   (firewall rule)
```

---

## 4. Firewall Integration — Network Intelligence

Since you have a full homelab with a dedicated firewall (pfSense/OPNsense), RedNode can manage it too:

### New Tools for Network Agent (Extended)

```json
{"name": "fw.status",          "agent": "network-agent", "risk": "low"},
{"name": "fw.rules_list",      "agent": "network-agent", "risk": "low"},
{"name": "fw.rule_add",        "agent": "network-agent", "risk": "critical"},
{"name": "fw.rule_delete",     "agent": "network-agent", "risk": "critical"},
{"name": "fw.block_ip",        "agent": "network-agent", "risk": "high"},
{"name": "fw.unblock_ip",      "agent": "network-agent", "risk": "high"},
{"name": "fw.vlan_status",     "agent": "network-agent", "risk": "low"},
{"name": "fw.traffic_top",     "agent": "network-agent", "risk": "low"},
{"name": "fw.vpn_status",      "agent": "network-agent", "risk": "low"},
{"name": "fw.dhcp_leases",     "agent": "network-agent", "risk": "low"},
{"name": "fw.isolate_device",  "agent": "network-agent", "risk": "high"}
```

---

## 5. Bringing It All Together — The Unified Home Brain

### Updated Agent Society (14 agents, ~120 tools)

```
Original (6):                    New Home Infrastructure (3):
├── 🔧 System Agent              ├── 🏗️ Infrastructure Agent (Pi-hole, firewall, UPS)
├── 🛡️ Security Agent            ├── 📹 Surveillance Agent (Frigate/NVR, cameras)
├── 💻 Coding Agent              └── 💾 Storage Agent (TrueNAS, backups, snapshots)
├── 🔬 Research Agent
├── ⚙️ Automation Agent          New Personal (8 — from previous discussion):
└── 🌐 Network Agent             ├── 📧 Communications Agent
                                  ├── 📱 Social Media Agent
                                  ├── 📝 Productivity Agent
                                  ├── 🌐 Browser Agent
                                  ├── 💰 Finance Agent
                                  ├── 🎵 Media Agent
                                  ├── 🏠 Smart Home Agent
                                  └── 🧬 Life Management Agent
```

### The Sentience Engine Integration

Your home infrastructure feeds directly into RedNode's self-awareness:

```
┌─────────── Sentience Engine Drives ───────────┐
│                                                │
│  🛡️ Security Drive (0.0 → 1.0)                │
│     ├── Falco eBPF events                      │
│     ├── CVE scan results                       │
│     ├── Pi-hole anomaly detection ◄── NEW      │
│     ├── Frigate unusual detections ◄── NEW     │
│     └── Firewall blocked attacks  ◄── NEW      │
│                                                │
│  🏗️ Integrity Drive (0.0 → 1.0)               │
│     ├── All agents alive?                      │
│     ├── TrueNAS pool health    ◄── NEW         │
│     ├── Disk SMART status      ◄── NEW         │
│     ├── NVR cameras online?    ◄── NEW         │
│     └── Pi-hole responding?    ◄── NEW         │
│                                                │
│  📚 Knowledge Drive (0.0 → 1.0)               │
│     ├── RAG corpus freshness                   │
│     └── Backup currency check  ◄── NEW         │
│                                                │
│  ⚡ Energy Drive (0.0 → 1.0)                   │
│     ├── UPS battery status     ◄── NEW         │
│     └── Server power draw                      │
│                                                │
│  🟢 Availability Drive (0.0 → 1.0)            │
│     ├── Can serve intentions?                  │
│     ├── Network connectivity                   │
│     └── Storage space available ◄── NEW        │
│                                                │
│  When any drive drops, RedNode autonomously    │
│  generates goals and dispatches agents.        │
│                                                │
│  Example: TrueNAS pool at 90% →                │
│    Integrity drops to 0.7 →                    │
│    Goal: "Clean old snapshots, alert owner" →  │
│    Storage Agent executes cleanup              │
└────────────────────────────────────────────────┘
```

### Example Automated Workflows

**1. "Goodnight" Workflow**
```
You say: "Goodnight"

RedNode does:
  1. Infrastructure Agent → Pi-hole: enable strict blocking (social media, gaming)
  2. Surveillance Agent → Set cameras to night mode, enable backyard motion alerts
  3. Network Agent → Firewall: block IoT outbound traffic (smart devices don't need internet at night)
  4. Storage Agent → TrueNAS: trigger nightly snapshot of documents dataset
  5. Sentience Engine → Start memory consolidation ("dream cycle")
  6. Productivity Agent → Generate tomorrow's task summary
```

**2. "I'm leaving the house" Workflow**
```
You say: "I'm leaving"

RedNode does:
  1. Surveillance Agent → All cameras to active monitoring, person detection HIGH priority
  2. Infrastructure Agent → Pi-hole stays on, full blocking
  3. Network Agent → Enable WireGuard tunnel for remote access
  4. All alerts → Route to mobile push (FCM) with snapshots
  5. If person detected → Immediate push with clip + biometric to view
```

**3. Autonomous: Disk Failing**
```
TrueNAS SMART shows disk warnings
  │
  ▼
Storage Agent detects via nas.smart
  │
  ▼
Sentience Engine: Integrity drive drops to 0.6
  │
  ▼
Autonomous goal: "Disk health critical — back up data, alert owner"
  │
  ├── Storage Agent: Trigger immediate replication to secondary
  ├── Security event logged: "DISK WARNING: /dev/sda — reallocated sectors"
  ├── Push notification to your phone
  └── Audit log: timestamped, hash-chained
```

---

## 6. Network Topology — Where RedNode Sits

Given your full homelab setup, here's the recommended placement:

```
                            INTERNET
                               │
                        ┌──────▼──────┐
                        │  FIREWALL   │  pfSense / OPNsense
                        │  (gateway)  │  RedNode manages via API
                        └──────┬──────┘
                               │
              ┌────────────────┼────────────────┐
              │                │                │
        VLAN 10          VLAN 20          VLAN 30
        TRUSTED          IoT/CAMERAS      MANAGEMENT
              │                │                │
     ┌────────┴────┐    ┌─────┴─────┐    ┌─────┴──────┐
     │             │    │           │    │            │
  ┌──▼───┐   ┌────▼┐  ┌▼────┐  ┌──▼─┐  │   ┌───────▼──────┐
  │Work- │   │Phone│  │NVR  │  │IoT │  │   │   RedNode    │
  │station│  │     │  │+cams│  │devs│  │   │   Server     │
  └──────┘   └─────┘  └─────┘  └────┘  │   │              │
                                        │   │  CNS:8787    │
                                        │   │  Web:3000    │
                                   ┌────▼┐  │  Frigate:5000│
                                   │True │  │  NATS:4222   │
                                   │NAS  │  │  Postgres    │
                                   └─────┘  │  Qdrant      │
                                        │   │  Ollama      │
                                   ┌────▼┐  │  Grafana:3001│
                                   │Pi-  │  └──────────────┘
                                   │hole │
                                   └─────┘

  RedNode on MANAGEMENT VLAN:
  - Can reach ALL VLANs (firewall rules allow)
  - Nothing can reach RedNode except:
    - Your devices (VLAN 10) on ports 3000, 8787
    - Tailscale/WireGuard for remote access
  - Pi-hole is the DNS server for ALL VLANs
  - RedNode queries Pi-hole API
  - RedNode queries TrueNAS API
  - RedNode queries NVR RTSP streams (via Frigate)
  - RedNode queries firewall API
```

### Firewall Rules for RedNode (Management VLAN)

```
# RedNode → Pi-hole (DNS queries + API)
ALLOW  rednode:*  →  pihole:53 (DNS)
ALLOW  rednode:*  →  pihole:80 (API)

# RedNode → TrueNAS (API + storage)
ALLOW  rednode:*  →  truenas:443 (API)
ALLOW  rednode:*  →  truenas:445 (SMB for backups)

# RedNode → NVR (RTSP streams for Frigate)
ALLOW  rednode:*  →  nvr:554 (RTSP)
ALLOW  rednode:*  →  nvr:9000 (ONVIF, if available)

# RedNode → Firewall (API management)
ALLOW  rednode:*  →  firewall:443 (API)

# Your devices → RedNode (dashboard + API)
ALLOW  vlan10:*   →  rednode:3000 (Web UI)
ALLOW  vlan10:*   →  rednode:8787 (API)
ALLOW  vlan10:*   →  rednode:5000 (Frigate UI)

# WireGuard / Tailscale → RedNode (remote access)
ALLOW  wg0:*      →  rednode:3000,8787,5000

# DENY everything else inbound to RedNode
DENY   *          →  rednode:*
```

---

## 7. Implementation Priority

| Phase | What | Effort | Impact |
|---|---|---|---|
| **Week 1–2** | Pi-hole Integration | Low — simple REST API, 9 tools | High — instant DNS intelligence |
| **Week 2–4** | TrueNAS Integration | Medium — REST API, 14 tools | High — storage health + backups |
| **Week 4–8** | Frigate Setup + Surveillance Agent | High — Docker + config + 12 tools | Very High — AI cameras + smart alerts |
| **Week 8–10** | Firewall Integration | Medium — API varies by fw | High — network security layer |
| **Week 10–12** | Cross-system Workflows | Medium — automation chains | Very High — "goodnight", "leaving", etc. |
| **Ongoing** | Sentience Engine feeds | Low — update drive calculations | The brain gets smarter over time |

---

## 8. Hardware Recommendation for RedNode Server

Given your full homelab, the RedNode server should be a dedicated box:

| Component | Recommendation | Why |
|---|---|---|
| **CPU** | Intel i5-13500 or AMD Ryzen 5 5600 | Frigate hardware decoding + Ollama |
| **RAM** | 32 GB DDR4/DDR5 | Ollama models + Postgres + Qdrant + Frigate |
| **GPU** | NVIDIA RTX 3060 (12GB) or RTX 4060 | Ollama LLM acceleration + Frigate TensorRT |
| **Boot disk** | 500GB NVMe SSD | OS + databases + Frigate events |
| **AI accelerator** | Google Coral USB ($60) | Dedicated Frigate object detection (optional if GPU) |
| **Network** | 2.5Gbe NIC (management VLAN) | Camera streams + TrueNAS access |
| **Form factor** | Mini-ITX or 1U rack mount | Fits in your existing rack |

**Budget**: ~$600–$900 for a dedicated RedNode server with GPU

**Alternative**: Run RedNode on an existing machine if you have spare capacity. Even without GPU, Qwen2.5-7B runs on CPU (slower but works), and a Coral USB handles Frigate detection.

---

*Your home already has the bones — Pi-hole for DNS, TrueNAS for storage, NVR for cameras, firewall for network. RedNode becomes the brain that ties them all together into a single, self-aware, autonomous home intelligence system. You stop managing 4 separate dashboards and start having one conversation with one system that manages everything.*
