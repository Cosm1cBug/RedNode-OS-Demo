# RedNode-OS — Complete Network & Infrastructure Placement Guide

> **Your Setup**: ISP router (bridge mode) → pfSense firewall → 4-5 VLANs → 15-30 devices  
> **Components**: pfSense, Pi-hole, TrueNAS (file storage), Standalone NVR + cameras, RedNode-OS server

---

## The Golden Rule of DNS Placement

**Pi-hole goes BEHIND pfSense, on the LAN side — on its own Management/Services VLAN.**

Why? Because pfSense is your gateway. Everything enters and exits through it. Pi-hole is a **service** — it serves DNS to your internal devices. It should be protected by your firewall, not exposed before it.

```
 ❌ WRONG: Pi-hole between ISP router and pfSense
    (exposed to WAN, unprotected, defeats the purpose)

 ❌ WRONG: Pi-hole on the same flat network as IoT devices
    (IoT devices could attack/bypass Pi-hole)

 ✅ CORRECT: Pi-hole behind pfSense, on a Services/Management VLAN
    (protected by firewall, serves all VLANs, isolated from untrusted devices)
```

---

## Complete Network Architecture

```
                        ┌─────────────┐
                        │  INTERNET   │
                        └──────┬──────┘
                               │
                        ┌──────▼──────┐
                        │ ISP ROUTER  │
                        │ (BRIDGE     │
                        │  MODE)      │
                        │             │
                        │ Just a modem│
                        │ passes      │
                        │ public IP   │
                        │ to pfSense  │
                        └──────┬──────┘
                               │ Public IP directly to pfSense
                               │
                ╔══════════════╧═══════════════╗
                ║        pfSENSE FIREWALL      ║
                ║     (Gateway / Router /       ║
                ║      DHCP Server / NAT)       ║
                ║                               ║
                ║  WAN: Public IP from ISP      ║
                ║  LAN: Trunk port to switch    ║
                ║                               ║
                ║  DHCP for each VLAN:          ║
                ║    VLAN 10: 10.0.10.0/24      ║
                ║    VLAN 20: 10.0.20.0/24      ║
                ║    VLAN 30: 10.0.30.0/24      ║
                ║    VLAN 40: 10.0.40.0/24      ║
                ║    VLAN 50: 10.0.50.0/24      ║
                ║                               ║
                ║  DNS FOR ALL VLANS:           ║
                ║  ──────────────────           ║
                ║  DHCP option: DNS =           ║
                ║    10.0.50.2 (Pi-hole)        ║
                ║                               ║
                ║  pfSense itself uses:         ║
                ║    Pi-hole as DNS →            ║
                ║    Pi-hole upstream: 1.1.1.1  ║
                ║    or Unbound on pfSense      ║
                ╚══════════════╤═══════════════╝
                               │
                               │ TRUNK (carries all VLANs tagged)
                               │
                ┌──────────────▼───────────────┐
                │     MANAGED SWITCH            │
                │  (VLAN-aware, 802.1Q)         │
                │                               │
                │  Trunk port ← pfSense         │
                │  Access ports per VLAN:       │
                │    Ports 1-6:   VLAN 10       │
                │    Ports 7-10:  VLAN 20       │
                │    Ports 11-14: VLAN 30       │
                │    Ports 15-18: VLAN 40       │
                │    Ports 19-22: VLAN 50       │
                │    Port 23-24:  Trunk/uplink  │
                └──────────────┬───────────────┘
                               │
          ┌────────────────────┼────────────────────┐
          │                    │                     │
          │                    │                     │
   ═══════╧═══════    ════════╧════════    ═════════╧═════════
```

---

## VLAN Layout — What Goes Where

```
┌──────────────────────────────────────────────────────────────────────┐
│                                                                      │
│  VLAN 10 — TRUSTED (Your Devices)         Subnet: 10.0.10.0/24     │
│  ─────────────────────────────────                                   │
│  │                                                                   │
│  ├── 🖥️  Your workstation / laptop        10.0.10.10               │
│  ├── 📱  Your phone (WiFi)                10.0.10.11               │
│  ├── 📱  Your tablet                      10.0.10.12               │
│  └── 💻  Any other personal devices       10.0.10.x                │
│                                                                      │
│  Access: Full internet, can reach VLAN 50 (management services)     │
│  DNS: 10.0.50.2 (Pi-hole)                                          │
│                                                                      │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  VLAN 20 — IoT (Smart Home Devices)       Subnet: 10.0.20.0/24     │
│  ──────────────────────────────────                                  │
│  │                                                                   │
│  ├── 💡  Smart lights (Hue, etc.)         10.0.20.x                │
│  ├── 🌡️  Smart thermostat                 10.0.20.x                │
│  ├── 🔊  Smart speakers                   10.0.20.x                │
│  ├── 🔌  Smart plugs                      10.0.20.x                │
│  └── 📺  Smart TV                         10.0.20.x                │
│                                                                      │
│  Access: Limited internet (cloud APIs only), NO access to other     │
│          VLANs, DNS via Pi-hole (heavy blocking on this VLAN)       │
│  DNS: 10.0.50.2 (Pi-hole — strict blocklist group)                  │
│                                                                      │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  VLAN 30 — CAMERAS (Surveillance)         Subnet: 10.0.30.0/24     │
│  ────────────────────────────────                                    │
│  │                                                                   │
│  ├── 📹  Camera 1 (front door)            10.0.30.10               │
│  ├── 📹  Camera 2 (driveway)              10.0.30.11               │
│  ├── 📹  Camera 3 (backyard)              10.0.30.12               │
│  ├── 📹  Camera 4 (garage)               10.0.30.13               │
│  └── 📹  Standalone NVR                   10.0.30.2                │
│                                                                      │
│  Access: ██ NO INTERNET ██ — cameras have ZERO reason to call      │
│          home to China/cloud. Only VLAN 50 (RedNode/Frigate)        │
│          can pull RTSP streams FROM this VLAN.                      │
│  DNS: NONE or Pi-hole (block everything — cameras don't need DNS)   │
│                                                                      │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  VLAN 40 — GUEST (Visitors WiFi)          Subnet: 10.0.40.0/24     │
│  ───────────────────────────────                                     │
│  │                                                                   │
│  ├── 📱  Guest phones                     10.0.40.x (DHCP)        │
│  └── 💻  Guest laptops                    10.0.40.x (DHCP)        │
│                                                                      │
│  Access: Internet only. ZERO access to any other VLAN.              │
│  DNS: 10.0.50.2 (Pi-hole — blocks ads for guests too)              │
│  Bandwidth: Rate-limited (optional — pfSense limiter)               │
│                                                                      │
├──────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  VLAN 50 — MANAGEMENT / SERVICES          Subnet: 10.0.50.0/24     │
│  ────────────────────────────────                                    │
│  │                                                                   │
│  │  ┌────────────────────────────────────────────────┐              │
│  │  │         🧠 RedNode-OS Server                   │              │
│  │  │         10.0.50.10                             │              │
│  │  │                                                │              │
│  │  │  Services running:                             │              │
│  │  │    CNS (Rust):        :8787                    │              │
│  │  │    Web Dashboard:     :3000                    │              │
│  │  │    Frigate NVR:       :5000  (Docker)          │              │
│  │  │    NATS JetStream:    :4222  (internal)        │              │
│  │  │    PostgreSQL:        :5432  (internal)        │              │
│  │  │    Qdrant:            :6333  (internal)        │              │
│  │  │    Ollama:            :11434 (internal)        │              │
│  │  │    Grafana:           :3001                    │              │
│  │  │    MQTT Broker:       :1883  (for Frigate)     │              │
│  │  └────────────────────────────────────────────────┘              │
│  │                                                                   │
│  ├── 🛡️  Pi-hole DNS                      10.0.50.2                │
│  │       (Raspberry Pi or Docker on RedNode)                        │
│  │       Port 53 (DNS), Port 80 (Admin UI)                         │
│  │       Upstream: Unbound on pfSense or Quad9/Cloudflare DoH      │
│  │                                                                   │
│  ├── 💾  TrueNAS                           10.0.50.3                │
│  │       Port 443 (Web UI + API)                                    │
│  │       Port 445 (SMB shares)                                      │
│  │       Port 2049 (NFS exports)                                    │
│  │                                                                   │
│  └── 🔧  Managed Switch management IP      10.0.50.1 (optional)    │
│                                                                      │
│  Access: CAN reach all VLANs (for management/monitoring)            │
│          Reachable FROM VLAN 10 only (your devices → dashboards)    │
│          NOT reachable from VLAN 20, 30, 40                         │
│  DNS: 10.0.50.2 (Pi-hole — itself uses upstream directly)           │
│                                                                      │
└──────────────────────────────────────────────────────────────────────┘
```

---

## The Complete Data Flow — How Everything Talks

```
                              INTERNET
                                 │
                          ┌──────▼──────┐
                          │ ISP ROUTER  │
                          │ (bridge)    │
                          └──────┬──────┘
                                 │ raw public IP
                          ╔══════▼══════╗
                          ║  pfSENSE    ║
                          ║             ║
                          ║  NAT / FW   ║──── Upstream DNS: Quad9 / Cloudflare
                          ║  DHCP       ║     (or Unbound resolver on pfSense
                          ║  Router     ║      for full DNS privacy)
                          ║             ║
                          ║  DNS option ║──── Points ALL VLANs to
                          ║  for DHCP:  ║     10.0.50.2 (Pi-hole)
                          ╚══════╤══════╝
                                 │
                         TRUNK (all VLANs tagged)
                                 │
                    ┌────────────┼────────────────────┐
                    │            │                     │
              VLAN 10      VLAN 20,30,40         VLAN 50
              TRUSTED      IoT/Cams/Guest       MANAGEMENT
                    │            │                     │
          ┌─────┬──┘            │           ┌─────────┼──────────┐
          │     │               │           │         │          │
       ┌──▼─┐ ┌▼───┐     ┌────┘      ┌────▼────┐ ┌──▼───┐  ┌──▼───┐
       │Work│ │Pho-│     │           │ RedNode │ │Pi-   │  │True- │
       │sta-│ │ne  │  ┌──▼───┐      │ Server  │ │hole  │  │NAS   │
       │tion│ │    │  │NVR + │      │         │ │      │  │      │
       └──┬─┘ └─┬──┘  │Cams  │      │ 10.0.   │ │10.0. │  │10.0. │
          │     │     └──────┘      │ 50.10   │ │50.2  │  │50.3  │
          │     │                   └────┬────┘ └──┬───┘  └──┬───┘
          │     │                        │         │         │
          └─────┴────────────────────────┴─────────┴─────────┘
```

### DNS Flow (Every Device)

```
Any device on any VLAN
        │
        │  DNS query: "google.com"
        │
        ▼
   ┌─────────┐
   │ Pi-hole  │  10.0.50.2:53
   │          │
   │ 1. Check blocklist     → if blocked → return 0.0.0.0 (ad gone)
   │ 2. Check cache          → if cached  → return cached IP
   │ 3. Forward upstream     → Quad9 / Unbound on pfSense
   │ 4. Log query + client   → RedNode pulls this via API
   └─────────┘
        │
        │  Response: 142.250.80.46
        │
        ▼
   Device gets the answer
```

### Camera Stream Flow

```
   Camera (VLAN 30: 10.0.30.10)
        │
        │  RTSP stream (video)
        │  rtsp://admin:pass@10.0.30.10:554/h264Preview_01_main
        │
        │  ⚠️ Camera has NO internet access
        │     pfSense blocks VLAN 30 → WAN entirely
        │     Camera cannot phone home to manufacturer cloud
        │
        ▼
   Firewall rule: ALLOW VLAN 50 → VLAN 30 :554 (RTSP only)
        │
        ▼
   ┌──────────────┐
   │  Frigate NVR  │  (Docker on RedNode server, VLAN 50)
   │               │
   │  Pulls RTSP   │ ◄── from camera via cross-VLAN firewall rule
   │  AI detection │     (person, car, animal, package)
   │  Records clips│     stored on RedNode SSD or TrueNAS NFS
   │               │
   │  Event fired: │
   │  "person at   │
   │   front door" │
   └───────┬───────┘
           │ MQTT publish: frigate/events
           ▼
   ┌───────────────┐
   │ RedNode CNS   │
   │ Surveillance  │
   │ Agent         │
   │               │
   │ → Security    │──► Push notification to your phone
   │   event       │    with snapshot attached
   │ → Audit log   │    (via FCM over WireGuard)
   │ → Sentience   │
   │   Engine      │
   └───────────────┘
```

### RedNode ↔ TrueNAS Flow

```
   RedNode Server (10.0.50.10)
        │
        │  REST API call
        │  GET https://10.0.50.3/api/v2.0/pool
        │  Authorization: Bearer <API-KEY>
        │
        ▼
   ┌──────────┐
   │ TrueNAS  │  10.0.50.3
   │          │
   │ Returns: │
   │  Pool health, disk SMART, usage
   │  RedNode stores in Sentience Engine
   │  (Integrity drive)
   └──────────┘

   Also:
   RedNode backs up its own Postgres + Qdrant
   to TrueNAS via SMB/NFS mount nightly:

   RedNode ──SMB──► TrueNAS:/mnt/tank/backups/rednode/
```

---

## pfSense Firewall Rules — Complete

This is the critical part. The rules define what can talk to what.

```
╔══════════════════════════════════════════════════════════════════════╗
║                    pfSENSE FIREWALL RULES                           ║
╠══════════════════════════════════════════════════════════════════════╣
║                                                                      ║
║  ── VLAN 10 (TRUSTED — Your Devices) ──────────────────────────     ║
║                                                                      ║
║  ALLOW  VLAN10 any  →  10.0.50.2    :53       # DNS to Pi-hole     ║
║  ALLOW  VLAN10 any  →  10.0.50.2    :80       # Pi-hole admin UI   ║
║  ALLOW  VLAN10 any  →  10.0.50.10   :3000     # RedNode Web UI     ║
║  ALLOW  VLAN10 any  →  10.0.50.10   :8787     # RedNode API        ║
║  ALLOW  VLAN10 any  →  10.0.50.10   :5000     # Frigate UI         ║
║  ALLOW  VLAN10 any  →  10.0.50.10   :3001     # Grafana            ║
║  ALLOW  VLAN10 any  →  10.0.50.3    :443      # TrueNAS UI         ║
║  ALLOW  VLAN10 any  →  10.0.50.3    :445      # TrueNAS SMB        ║
║  ALLOW  VLAN10 any  →  WAN          any       # Full internet      ║
║  DENY   VLAN10 any  →  VLAN20/30/40 any       # No direct IoT/cam  ║
║                                                                      ║
║  ── VLAN 20 (IoT — Smart Devices) ─────────────────────────────     ║
║                                                                      ║
║  ALLOW  VLAN20 any  →  10.0.50.2    :53       # DNS to Pi-hole     ║
║  ALLOW  VLAN20 any  →  WAN          :443      # HTTPS only (cloud) ║
║  ALLOW  VLAN20 any  →  WAN          :8883     # MQTT cloud (some)  ║
║  DENY   VLAN20 any  →  10.0.50.0/24 any       # No mgmt access    ║
║  DENY   VLAN20 any  →  VLAN10/30/40 any       # No cross-VLAN     ║
║  DENY   VLAN20 any  →  WAN          :80       # Block HTTP (force  ║
║                                                  HTTPS or nothing)  ║
║                                                                      ║
║  ── VLAN 30 (CAMERAS — Surveillance) ──────────────────────────     ║
║                                                                      ║
║  ██ DENY  VLAN30 any  →  WAN        any  ██   # ZERO INTERNET      ║
║  ██ DENY  VLAN30 any  →  VLAN10     any  ██   # for cameras.       ║
║  ██ DENY  VLAN30 any  →  VLAN20     any  ██   # They are           ║
║  ██ DENY  VLAN30 any  →  VLAN40     any  ██   # completely         ║
║  ██ DENY  VLAN30 any  →  VLAN50     any  ██   # isolated.          ║
║                                                                      ║
║  # But VLAN 50 CAN reach INTO VLAN 30 (one-way):                   ║
║  # (This is set on VLAN 50 rules, not here)                        ║
║                                                                      ║
║  ── VLAN 40 (GUEST — Visitor WiFi) ────────────────────────────     ║
║                                                                      ║
║  ALLOW  VLAN40 any  →  10.0.50.2    :53       # DNS to Pi-hole     ║
║  ALLOW  VLAN40 any  →  WAN          :443      # HTTPS internet     ║
║  ALLOW  VLAN40 any  →  WAN          :80       # HTTP internet      ║
║  DENY   VLAN40 any  →  10.0.50.0/24 any       # No mgmt access    ║
║  DENY   VLAN40 any  →  VLAN10/20/30 any       # Total isolation    ║
║  # Optional: bandwidth limiter (50 Mbps per guest)                  ║
║                                                                      ║
║  ── VLAN 50 (MANAGEMENT — Services) ───────────────────────────     ║
║                                                                      ║
║  # RedNode → Camera VLAN (pull RTSP streams)                        ║
║  ALLOW  10.0.50.10  →  10.0.30.0/24 :554      # RTSP only         ║
║  ALLOW  10.0.50.10  →  10.0.30.0/24 :9000     # ONVIF (optional)  ║
║                                                                      ║
║  # RedNode → Pi-hole (API management)                               ║
║  ALLOW  10.0.50.10  →  10.0.50.2    :80       # Pi-hole API       ║
║  ALLOW  10.0.50.10  →  10.0.50.2    :53       # DNS queries       ║
║                                                                      ║
║  # RedNode → TrueNAS (API + storage)                                ║
║  ALLOW  10.0.50.10  →  10.0.50.3    :443      # TrueNAS API       ║
║  ALLOW  10.0.50.10  →  10.0.50.3    :445      # SMB backups       ║
║  ALLOW  10.0.50.10  →  10.0.50.3    :2049     # NFS (optional)    ║
║                                                                      ║
║  # RedNode → pfSense (firewall API management)                     ║
║  ALLOW  10.0.50.10  →  pfSense_IP   :443      # pfSense API       ║
║                                                                      ║
║  # RedNode → Internet (for Ollama model pulls, NVD CVE sync)       ║
║  ALLOW  10.0.50.10  →  WAN          :443      # HTTPS only        ║
║  # ⚠️ This can be tightened to specific IPs/domains only           ║
║                                                                      ║
║  # Pi-hole → upstream DNS                                           ║
║  ALLOW  10.0.50.2   →  WAN          :443      # DoH upstream      ║
║  ALLOW  10.0.50.2   →  WAN          :853      # DoT upstream      ║
║  # OR if using Unbound on pfSense:                                  ║
║  ALLOW  10.0.50.2   →  pfSense_IP   :53       # Unbound           ║
║                                                                      ║
║  # TrueNAS — no internet needed                                     ║
║  DENY   10.0.50.3   →  WAN          any       # TrueNAS is local  ║
║                                                                      ║
║  ── WireGuard / Tailscale (Remote Access) ─────────────────────     ║
║                                                                      ║
║  # VPN interface (wg0 / tailscale0) — treat like VLAN 10           ║
║  ALLOW  wg0 any     →  10.0.50.10   :3000     # RedNode Web       ║
║  ALLOW  wg0 any     →  10.0.50.10   :8787     # RedNode API       ║
║  ALLOW  wg0 any     →  10.0.50.10   :5000     # Frigate           ║
║  ALLOW  wg0 any     →  10.0.50.3    :443      # TrueNAS           ║
║  DENY   wg0 any     →  any          any       # Nothing else      ║
║                                                                      ║
╚══════════════════════════════════════════════════════════════════════╝
```

---

## Physical Rack Layout

```
┌──────────────────────────────────────────┐
│              SERVER RACK                  │
│                                          │
│  ┌──────────────────────────────────┐    │
│  │ 1U — ISP Router (bridge mode)   │    │  ← WAN in
│  │      Just passes internet through│    │
│  └──────────────────────────────────┘    │
│                 │ ethernet                │
│  ┌──────────────▼───────────────────┐    │
│  │ 1U — pfSense Firewall           │    │  ← Gateway, NAT, DHCP, 
│  │      (Netgate / mini-PC / VM)   │    │    VLAN routing, VPN
│  │      WAN port ← ISP router      │    │
│  │      LAN port → managed switch  │    │    DNS server setting for
│  │      (trunk, all VLANs)         │    │    all DHCP scopes:
│  └──────────────┬───────────────────┘    │    → 10.0.50.2 (Pi-hole)
│                 │ trunk (802.1Q)          │
│  ┌──────────────▼───────────────────┐    │
│  │ 1U — Managed Switch             │    │  ← VLAN-aware, 24-port
│  │      (UniFi / MikroTik / etc.)  │    │    Trunk from pfSense
│  │                                  │    │    Access ports per VLAN
│  │      Ports 1-6:   VLAN 10       │    │
│  │      Ports 7-10:  VLAN 20       │    │
│  │      Ports 11-14: VLAN 30       │    │
│  │      Ports 15-18: VLAN 40       │    │
│  │      Ports 19-22: VLAN 50       │    │
│  └──────────────────────────────────┘    │
│                                          │
│  ┌──────────────────────────────────┐    │
│  │ 1-2U — RedNode-OS Server        │    │  ← The Brain
│  │        10.0.50.10 (VLAN 50)     │    │    CNS + Frigate + all
│  │                                  │    │    services + Coral USB
│  │   Connected to switch VLAN 50   │    │
│  │   port (access mode)            │    │
│  └──────────────────────────────────┘    │
│                                          │
│  ┌──────────────────────────────────┐    │
│  │ 2-4U — TrueNAS                  │    │  ← Storage
│  │        10.0.50.3 (VLAN 50)      │    │    SMB shares, backups,
│  │                                  │    │    Frigate recordings
│  │   Connected to switch VLAN 50   │    │
│  │   port (access mode)            │    │
│  └──────────────────────────────────┘    │
│                                          │
│  ┌──────────────────────────────────┐    │
│  │ Pi-hole (Raspberry Pi or Docker) │    │  ← DNS
│  │        10.0.50.2 (VLAN 50)      │    │
│  │                                  │    │    Option A: Raspberry Pi
│  │   Connected to switch VLAN 50   │    │    on VLAN 50 port
│  │   port (access mode)            │    │    
│  │   OR: Docker container on       │    │    Option B: Docker on
│  │   RedNode server (simpler)      │    │    RedNode server (fewer
│  └──────────────────────────────────┘    │    devices to manage)
│                                          │
│  ┌──────────────────────────────────┐    │
│  │ NVR (Standalone)                 │    │  ← Cameras
│  │        10.0.30.2 (VLAN 30)      │    │    Connected to switch
│  │                                  │    │    on VLAN 30 port
│  │   Cameras connect to NVR or     │    │    (access mode)
│  │   directly to VLAN 30 ports     │    │
│  └──────────────────────────────────┘    │
│                                          │
│  ┌──────────────────────────────────┐    │
│  │ UPS (Uninterruptible Power)     │    │  ← Power protection
│  │   Powers: pfSense, switch,      │    │    for all critical
│  │   RedNode, TrueNAS, NVR         │    │    equipment
│  │   USB to RedNode for monitoring │    │
│  └──────────────────────────────────┘    │
│                                          │
└──────────────────────────────────────────┘
```

---

## Pi-hole Placement: The Two Options

### Option A: Pi-hole on a Dedicated Raspberry Pi (Recommended)

```
Pros:
  ✅ If RedNode server goes down, DNS still works (internet doesn't break)
  ✅ Dedicated hardware — no resource contention
  ✅ Simple, low power (~5W), always on
  ✅ Pi-hole updates don't affect RedNode
  ✅ Hardware cost: ~$50 (Pi 4) or ~$15 (Pi Zero 2W)

Cons:
  ❌ One more device to manage
  ❌ Separate IP to maintain

Setup:
  - Raspberry Pi on VLAN 50 port (access mode)
  - Static IP: 10.0.50.2
  - pfSense DHCP: DNS = 10.0.50.2 for ALL VLANs
  - Pi-hole upstream: Quad9 (9.9.9.9) or Unbound on pfSense
  - RedNode queries Pi-hole API at http://10.0.50.2/api/
```

### Option B: Pi-hole as Docker Container on RedNode Server

```
Pros:
  ✅ One less device
  ✅ Easier to manage (everything in one place)
  ✅ RedNode has direct localhost access to Pi-hole API

Cons:
  ❌ If RedNode server reboots/crashes, ALL DNS stops → internet down for
     every device until RedNode comes back
  ❌ Docker/RedNode update could break DNS
  ❌ Port 53 conflicts possible

Mitigation if choosing this option:
  - Set pfSense as SECONDARY DNS (10.0.50.1) in DHCP
  - Devices will fall back to pfSense DNS if Pi-hole is down
  - But fallback DNS won't have ad-blocking

Setup:
  - Docker container on RedNode server
  - Binds to 10.0.50.10:53
  - pfSense DHCP: DNS = 10.0.50.10 (primary), 10.0.50.1 (secondary/fallback)
  - RedNode queries Pi-hole API at http://localhost/api/
```

### My Recommendation

**Go with Option A (Raspberry Pi)** — DNS is too critical to tie to a single server. If RedNode reboots for an update, you don't want your entire home to lose internet. A $15 Pi Zero 2W running Pi-hole is the most reliable DNS setup.

---

## pfSense DHCP Configuration

Set this in pfSense for each VLAN's DHCP scope:

```
pfSense → Services → DHCP Server

VLAN 10 (Trusted):
  DNS Server 1: 10.0.50.2  (Pi-hole)
  DNS Server 2: (empty — or pfSense IP as fallback)
  Gateway: 10.0.10.1 (pfSense VLAN 10 interface)

VLAN 20 (IoT):
  DNS Server 1: 10.0.50.2  (Pi-hole — strict blocking group)
  Gateway: 10.0.20.1

VLAN 30 (Cameras):
  DNS Server 1: (none — cameras don't need DNS)
  Gateway: 10.0.30.1
  ⚠️ Or set DNS to Pi-hole and block ALL domains for this VLAN

VLAN 40 (Guest):
  DNS Server 1: 10.0.50.2  (Pi-hole — blocks ads for guests)
  Gateway: 10.0.40.1

VLAN 50 (Management):
  DNS Server 1: 10.0.50.2  (Pi-hole)
  Gateway: 10.0.50.1
  ⚠️ Pi-hole itself doesn't use DHCP — static IP
  ⚠️ RedNode, TrueNAS — all static IPs
```

### Pi-hole Group Management (Per-VLAN Blocking)

Pi-hole v6 supports **groups** — you assign clients to groups and apply different blocklists per group:

```
Pi-hole Groups:
  ├── "trusted"    → VLAN 10 clients → moderate blocking (ads + trackers)
  ├── "iot"        → VLAN 20 clients → strict blocking (telemetry + cloud)
  ├── "cameras"    → VLAN 30 clients → block everything (if DNS enabled)
  ├── "guests"     → VLAN 40 clients → moderate blocking (ads)
  └── "management" → VLAN 50 clients → minimal blocking (don't block infra)

RedNode can manage these groups via Pi-hole API:
  "Block social media on IoT VLAN" → add blocklist to "iot" group
  "Enable focus mode" → add social media blocklist to "trusted" group temporarily
```

---

## Complete Integration Summary

```
┌──────────────────────────────────────────────────────────────┐
│                                                              │
│                   YOUR COMPLETE SETUP                         │
│                                                              │
│  Internet → ISP (bridge) → pfSense → Managed Switch         │
│                                          │                   │
│                    ┌─────────────────────┼───────────┐       │
│                    │                     │           │       │
│               VLAN 10              VLAN 30      VLAN 50      │
│               Your devices         Cameras      Services     │
│                    │                     │           │       │
│               ┌────┘               ┌────┘     ┌─────┼────┐  │
│               │                    │          │     │    │  │
│           Workstation           NVR+Cams   RedNode Pi-  True │
│           Phone                              hole  NAS │
│           Tablet                               │        │  │
│                                                │        │  │
│               You open browser:                │        │  │
│               http://10.0.50.10:3000           │        │  │
│                        │                       │        │  │
│                        ▼                       │        │  │
│                ┌───────────────┐               │        │  │
│                │ RedNode       │───── queries ─┘        │  │
│                │ Dashboard     │───── queries ──────────┘  │
│                │               │───── pulls RTSP from ─────┘
│                │ "How's my     │     cameras (VLAN 30)      │
│                │  home doing?" │                            │
│                │               │                            │
│                │ "All systems  │                            │
│                │  green.       │                            │
│                │  2 persons    │                            │
│                │  detected     │                            │
│                │  today.       │                            │
│                │  Pi-hole:     │                            │
│                │  23% blocked. │                            │
│                │  TrueNAS:     │                            │
│                │  42% used.    │                            │
│                │  All disks    │                            │
│                │  healthy."    │                            │
│                └───────────────┘                            │
│                                                              │
└──────────────────────────────────────────────────────────────┘
```

---

*Everything behind pfSense. Pi-hole on VLAN 50 (management). Cameras completely isolated on VLAN 30 with zero internet. RedNode orchestrates everything from VLAN 50. Your devices on VLAN 10 can only reach the dashboards. Guests on VLAN 40 get internet and nothing else.*
