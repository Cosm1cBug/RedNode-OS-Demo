# RedNode-OS — Network Security Setup

> Suricata IDS on pfSense + arpwatch for ARP spoofing detection

---

## 1. Suricata IDS on pfSense

Suricata inspects actual packet content — catches exploits, malware downloads, and C2 traffic that DNS blocking alone misses.

### Install

```
pfSense Web UI → System → Package Manager → Available Packages
→ Search "suricata" → Install
```

### Configure

```
Services → Suricata → Interfaces → Add (WAN)

Settings:
  ✅ Enable Suricata on this interface
  Interface: WAN
  
  # Rules
  → Categories: select ET Open rules (free)
  → Enable: Emerging Threats rules
  → Update interval: 12 hours
  
  # Performance (for your hardware)
  Max Pending Packets: 1024
  Detect-Engine Profile: Medium
  Pattern Matcher: Aho-Corasick (default)
  
  # Important: Block or Alert
  ✅ Block Offenders — enables IPS mode (blocks, not just alerts)
  Block Duration: 3600 (1 hour)
  
  # To avoid false positives initially:
  ❌ Block Offenders (start with ALERT mode first)
  → Review alerts for a week
  → Then enable blocking after tuning
```

### Suricata Alerts → RedNode

To forward Suricata alerts to RedNode, create a cron job on pfSense:

```bash
# On pfSense shell (Diagnostics → Command Prompt):
# This sends new Suricata alerts to RedNode every 60 seconds

cat > /root/suricata-to-rednode.sh << 'EOF'
#!/bin/sh
REDNODE="http://10.0.50.10:8787"
LOG="/var/log/suricata/suricata_WAN/alerts.json"
LAST="/tmp/suricata-last-pos"

if [ ! -f "$LAST" ]; then echo 0 > "$LAST"; fi
POS=$(cat "$LAST")
CURRENT=$(wc -c < "$LOG" 2>/dev/null || echo 0)

if [ "$CURRENT" -gt "$POS" ]; then
  tail -c +$((POS+1)) "$LOG" | head -20 | while read line; do
    ALERT=$(echo "$line" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('alert',{}).get('signature','unknown'))" 2>/dev/null || echo "unknown")
    SEVERITY=$(echo "$line" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('alert',{}).get('severity',3))" 2>/dev/null || echo "3")
    
    SEV="LOW"
    [ "$SEVERITY" -le 1 ] && SEV="CRITICAL"
    [ "$SEVERITY" -eq 2 ] && SEV="HIGH"
    
    curl -sf -X POST "$REDNODE/security/events" \
      -H "Content-Type: application/json" \
      -d "{\"severity\":\"$SEV\",\"source\":\"suricata/pfSense\",\"summary\":\"$ALERT\",\"raw\":$line}" \
      >/dev/null 2>&1
  done
  echo "$CURRENT" > "$LAST"
fi
EOF
chmod +x /root/suricata-to-rednode.sh

# Add to cron (System → Cron → Add):
# Minute: */1  Hour: *  Command: /root/suricata-to-rednode.sh
```

---

## 2. ARP Watch — ARP Spoofing Detection

ARP spoofing lets an attacker intercept traffic on your LAN by pretending to be the gateway. arpwatch detects MAC address changes.

### Option A: arpwatch on RedNode Server (Recommended)

```bash
# Install arpwatch on RedNode (NixOS)
# Add to configuration.nix:
environment.systemPackages = [ pkgs.arpwatch ];

# Or install directly:
nix-env -i arpwatch

# Start monitoring your management VLAN interface:
sudo arpwatch -i enp0s31f6 -f /var/lib/rednode/arpwatch.dat

# arpwatch logs to syslog. Monitor for alerts:
# "changed ethernet address" = possible ARP spoofing
# "new station" = new device appeared
# "flip flop" = MAC address rapidly changing (definite attack)
```

### Create a RedNode integration script:

```bash
# /var/lib/rednode/scripts/arpwatch-monitor.sh
#!/bin/bash
REDNODE="http://localhost:8787"

# Watch syslog for arpwatch events
journalctl -f -u arpwatch --no-pager 2>/dev/null | \
tail -f /var/log/syslog 2>/dev/null | \
grep --line-buffered "arpwatch" | while read line; do
  SEVERITY="MEDIUM"
  if echo "$line" | grep -qi "changed ethernet\|flip flop"; then
    SEVERITY="CRITICAL"
  fi
  if echo "$line" | grep -qi "new station"; then
    SEVERITY="LOW"
  fi

  SUMMARY=$(echo "$line" | sed 's/.*arpwatch: //')

  curl -sf -X POST "$REDNODE/security/events" \
    -H "Content-Type: application/json" \
    -d "{\"severity\":\"$SEVERITY\",\"source\":\"arpwatch\",\"summary\":\"$SUMMARY\",\"raw\":{\"log\":\"$line\"}}" \
    >/dev/null 2>&1

  echo "[arpwatch→rednode] $SEVERITY: $SUMMARY"
done
```

### Option B: arpwatch on pfSense

```
# pfSense doesn't have arpwatch as a package, but you can:
# 1. Use pfSense's built-in ARP table monitoring:
#    Diagnostics → ARP Table → review periodically
# 2. Or install arpwatch via FreeBSD pkg:
#    pkg install arpwatch
#    (not officially supported on pfSense — use Option A instead)
```

### What arpwatch Detects:

| Event | Meaning | Severity |
|---|---|---|
| `new station` | New device joined the network | LOW |
| `changed ethernet address` | A known IP changed its MAC — possible ARP spoof | CRITICAL |
| `flip flop` | MAC rapidly alternating — active ARP spoofing attack | CRITICAL |
| `new activity` | Known device became active after being quiet | LOW |
| `reused old ethernet address` | Device returned to a previously seen MAC | LOW |

---

## 3. Port Mirroring on TL-SG2218

To let RedNode see ALL network traffic (not just its own):

```
Switch Web UI → Monitoring → Port Mirroring

  Source Ports: All VLAN ports (or specific VLAN trunk)
  Direction: Both (ingress + egress)
  Destination Port: The port connected to RedNode server

  This copies ALL traffic to RedNode's NIC.
  RedNode can then run tcpdump, Suricata, or arpwatch on it.
```

**Note**: Port mirroring doubles the traffic on RedNode's port. Ensure your NIC can handle it (1Gbps is usually fine for a home network).
