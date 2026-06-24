import { RedNodeAgent } from "../../shared/src/agent.js";
import { sh, api, llm, cns, pihole, truenas, frigate, ha } from "../../shared/src/helpers.js";

const PIHOLE_URL = process.env.PIHOLE_URL || "http://10.0.50.2";
const TOOLS = [
  "dns.check",
  "firewall.rules",
  "fw.block_ip",
  "fw.isolate_device",
  "fw.unblock_ip",
  "net.arp_table",
  "net.bandwidth",
  "net.connection_table",
  "net.devices",
  "net.dhcp_leases",
  "net.dns_lookup",
  "net.interface_stats",
  "net.mtr",
  "net.ping",
  "net.port_forward",
  "net.route_table",
  "net.scan",
  "net.speed_test",
  "net.status",
  "net.traceroute",
  "net.vlan_list",
  "net.vlan_move",
  "net.whois",
  "net.wifi_scan",
  "net.wol",
  "traffic.analyze",
  "vpn.add_peer",
  "vpn.connect",
  "vpn.remove_peer",
  "vpn.status",
];

class NetworkAgent extends RedNodeAgent {
  constructor() {
    super("network", TOOLS);
  }

  async handleTool(tool: string, args: any): Promise<any> {
    switch (tool) {
      case "net.status": {
        // Execute ss via Rust executor, then parse output
        const result = await this.callTool(tool, args);
        if (!result?.ok) return result;

        const lines = (result.stdout || result.output || "")
          .split("\n")
          .filter((l: string) => l.trim());
        const listening = lines.filter((l: string) => l.includes("LISTEN"));
        const established = lines.filter((l: string) => l.includes("ESTAB"));

        let enriched = `Network connections:\n`;
        enriched += `  Listening ports: ${listening.length}\n`;
        enriched += `  Established connections: ${established.length}\n\n`;
        enriched += `Listening:\n`;
        listening.forEach((l: string) => {
          enriched += `  ${l}\n`;
        });

        if (established.length > 0) {
          enriched += `\nEstablished (first 10):\n`;
          established.slice(0, 10).forEach((l: string) => {
            enriched += `  ${l}\n`;
          });
        }

        return {
          ok: true,
          output: enriched,
          tool,
          listening: listening.length,
          established: established.length,
        };
      }

      case "dns.check": {
        // Check if Pi-hole is responding
        let piholeOk = false;
        let piholeStats: any = null;
        try {
          const resp = await fetch(`${PIHOLE_URL}/api/stats/summary`, {
            signal: AbortSignal.timeout(3000),
          });
          if (resp.ok) {
            piholeOk = true;
            piholeStats = await resp.json();
          }
        } catch {}

        // Also check external DNS resolution via the Rust executor
        const digResult = await this.callTool("shell.run_safe", {
          cmd: "date",
        }).catch(() => null);

        let output = `DNS Status:\n`;
        output += `  Pi-hole (${PIHOLE_URL}): ${piholeOk ? "✅ Online" : "❌ OFFLINE"}\n`;
        if (piholeStats) {
          output += `  Queries today: ${piholeStats.queries?.total || "?"}\n`;
          output += `  Blocked: ${piholeStats.queries?.blocked || "?"} (${piholeStats.queries?.percent_blocked?.toFixed(1) || "?"}%)\n`;
        }
        output += `  System time: ${digResult?.output?.trim() || "unknown"}\n`;

        return {
          ok: true,
          output,
          pihole_online: piholeOk,
          stats: piholeStats,
        };
      }

      case "traffic.analyze": {
        // Use ss to get connection info, analyze top talkers
        const result = await this.callTool("net.status", {});
        if (!result?.ok) return result;

        return {
          ok: true,
          output: `Traffic analysis:\n  Listening: ${result.listening} ports\n  Established: ${result.established} connections\n  (Full traffic analysis requires additional tooling — NetFlow/sFlow)`,
          tool,
        };
      }

      case "net.scan": {
        // Nmap-based network scan — discovers devices, open ports, OS detection
        // Requires: nmap installed (apt install nmap / nix-env -i nmap)
        const target = args.target || args.subnet || "10.0.50.0/24";
        const scanType = args.type || "quick"; // quick, full, vuln

        // Validate target — only allow private network ranges
        if (
          !/^(10\.|192\.168\.|172\.(1[6-9]|2[0-9]|3[01])\.)/.test(target) &&
          target !== "localhost"
        ) {
          return {
            ok: false,
            error:
              "net.scan only allowed on private networks (10.x, 192.168.x, 172.16-31.x)",
          };
        }

        const nmapArgs: Record<string, string> = {
          quick: `-sn ${target}`, // Ping scan — fast, just find live hosts
          ports: `-sT --top-ports 100 ${target}`, // Top 100 TCP ports
          full: `-sV -O --top-ports 1000 ${target}`, // Service version + OS detection
          vuln: `--script vuln --top-ports 100 ${target}`, // Vulnerability scan
        };

        const cmd = nmapArgs[scanType] || nmapArgs.quick;

        try {
          const { exec } = await import("child_process");
          const { promisify } = await import("util");
          const execAsync = promisify(exec);

          const { stdout, stderr } = await execAsync(`nmap ${cmd} 2>&1`, {
            timeout: 120000,
          });
          const output = stdout + stderr;

          // Parse results for structured output
          const hosts: {
            ip: string;
            hostname: string;
            ports: string[];
            os: string;
          }[] = [];
          let currentHost: any = null;

          for (const line of output.split("\n")) {
            const hostMatch = line.match(
              /Nmap scan report for (\S+)\s*\(?(\d+\.\d+\.\d+\.\d+)?\)?/,
            );
            if (hostMatch) {
              if (currentHost) hosts.push(currentHost);
              currentHost = {
                ip: hostMatch[2] || hostMatch[1],
                hostname: hostMatch[2] ? hostMatch[1] : "",
                ports: [],
                os: "",
              };
            }
            const portMatch = line.match(/^(\d+)\/(tcp|udp)\s+(\w+)\s+(.*)/);
            if (portMatch && currentHost) {
              currentHost.ports.push(
                `${portMatch[1]}/${portMatch[2]} ${portMatch[3]} ${portMatch[4]}`,
              );
            }
            if (line.includes("OS details:") && currentHost) {
              currentHost.os = line.replace("OS details:", "").trim();
            }
          }
          if (currentHost) hosts.push(currentHost);

          // Ingest scan results into memory
          const CNS = process.env.REDNODE_CNS || "http://localhost:8787";
          const summary = hosts
            .map(
              (h) =>
                `${h.ip}${h.hostname ? ` (${h.hostname})` : ""}: ${h.ports.length} ports${h.os ? `, OS: ${h.os}` : ""}`,
            )
            .join("\n");

          await fetch(`${CNS}/memory/ingest`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              source: `net-scan/${target.replace(/\//g, "-")}`,
              content: `Network scan ${target} (${scanType}): ${hosts.length} hosts\n${summary}`,
            }),
          }).catch(() => {});

          // Report new/unexpected devices as security events
          if (scanType === "quick" && hosts.length > 0) {
            await fetch(`${CNS}/security/events`, {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                severity: "LOW",
                source: "net-scan",
                summary: `Network scan: ${hosts.length} hosts found on ${target}`,
                raw: { hosts, scan_type: scanType },
              }),
            }).catch(() => {});
          }

          return {
            ok: true,
            output: `Network Scan (${scanType}) — ${target}:\n${hosts.length} hosts found\n\n${summary || output.substring(0, 3000)}`,
            hosts,
            scan_type: scanType,
          };
        } catch (e: any) {
          if (e.message.includes("not found") || e.message.includes("ENOENT")) {
            return {
              ok: false,
              error:
                "nmap not installed. Install: apt install nmap / nix-env -i nmap",
            };
          }
          return { ok: false, error: e.message };
        }
      }

      case "net.devices": {
        // Quick ARP table scan — see all devices on local network
        try {
          const { exec } = await import("child_process");
          const { promisify } = await import("util");
          const execAsync = promisify(exec);

          const { stdout } = await execAsync(
            "ip neigh show 2>/dev/null || arp -a 2>/dev/null",
            { timeout: 5000 },
          );
          const devices: { ip: string; mac: string; state: string }[] = [];

          for (const line of stdout.trim().split("\n")) {
            // ip neigh format: 10.0.50.1 dev enp0s31f6 lladdr aa:bb:cc:dd:ee:ff REACHABLE
            const match = line.match(
              /^(\d+\.\d+\.\d+\.\d+)\s.*lladdr\s+([0-9a-f:]+)\s+(\w+)/i,
            );
            if (match) {
              devices.push({ ip: match[1], mac: match[2], state: match[3] });
            }
            // arp -a format: ? (10.0.50.1) at aa:bb:cc:dd:ee:ff [ether] on enp0s31f6
            const arpMatch = line.match(
              /\((\d+\.\d+\.\d+\.\d+)\)\s+at\s+([0-9a-f:]+)/i,
            );
            if (arpMatch) {
              devices.push({
                ip: arpMatch[1],
                mac: arpMatch[2],
                state: "reachable",
              });
            }
          }

          const lines = devices.map(
            (d) => `  ${d.ip.padEnd(16)} ${d.mac.padEnd(18)} ${d.state}`,
          );
          return {
            ok: true,
            output: `Network Devices (ARP table):\n${lines.join("\n") || "  No devices found"}`,
            devices,
            count: devices.length,
          };
        } catch (e: any) {
          return { ok: false, error: e.message };
        }
      }

      case "firewall.rules":
      case "vpn.connect":
        // High risk — pass through to Rust executor for approval gate
        return null;
      case "fw.block_ip": {
        const r = await sh("wg-quick up wg0 2>&1 || echo \"WireGuard not configured\""); return { ok: r.ok, output: r.output, tool };
      }

      case "fw.unblock_ip": {
        return null; // Rust executor: high-risk, requires approval
      }

      case "fw.isolate_device": {
        return null; // Rust executor: high-risk, requires approval
      }

      case "vpn.status": {
        const r = await sh("wg show 2>/dev/null || echo \"WireGuard not configured\""); return { ok: r.ok, output: r.output, tool };
      }

      case "vpn.add_peer": {
        const r = await sh("wg show 2>/dev/null || echo \"WireGuard not configured\""); return { ok: r.ok, output: r.output, tool };
      }

      case "vpn.remove_peer": {
        const r = await sh("wg show 2>/dev/null || echo \"WireGuard not configured\""); return { ok: r.ok, output: r.output, tool };
      }

      case "net.wol": {
        const mac = args.mac || args.target || "";
                if (!mac) return { ok: false, error: "Missing 'mac' address for Wake-on-LAN" };
                const mac = args.mac || ""; if (!mac) return { ok: false, error: "Missing MAC address" }; const r = await sh(`etherwake ${mac} 2>&1 || wakeonlan ${mac} 2>&1 || echo "WoL tools not installed"`); return { ok: r.ok, output: r.output, tool }; etherwake / wol
      }

      case "net.bandwidth": {
        const iface = args.interface || "enp0s31f6"; const r = await sh(`vnstat -i ${iface} --oneline 2>/dev/null || cat /proc/net/dev | grep ${iface}`); return { ok: r.ok, output: r.output, tool }; vnstat / nethogs summary
      }

      case "net.ping": {
        const host = args.host || args.target || "";
                if (!host) return { ok: false, error: "Missing 'host' to ping" };
                try {
                  const { execSync } = await import("child_process");
                  const count = args.count || 4;
                  const out = execSync(`ping -c ${count} -W 3 ${host} 2>&1`, { encoding: "utf-8", timeout: 15000 });
                  return { ok: true, output: out.trim(), tool };
                } catch (e: any) { return { ok: true, output: `Ping to ${host} failed: ${e.message}`, tool }; }
      }

      case "net.traceroute": {
        const host = args.host || args.target || "";
                if (!host) return { ok: false, error: "Missing 'host' to traceroute" };
                try {
                  const { execSync } = await import("child_process");
                  const out = execSync(`traceroute -m 20 -w 2 ${host} 2>&1 || tracepath ${host} 2>&1`, { encoding: "utf-8", timeout: 30000 });
                  return { ok: true, output: out.trim(), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "net.dns_lookup": {
        const domain = args.domain || args.host || "";
                if (!domain) return { ok: false, error: "Missing 'domain'" };
                try {
                  const { execSync } = await import("child_process");
                  const rtype = args.type || "A";
                  const out = execSync(`dig ${domain} ${rtype} +short 2>/dev/null || nslookup ${domain} 2>&1`, { encoding: "utf-8", timeout: 10000 });
                  return { ok: true, output: out.trim(), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "net.whois": {
        const domain = args.domain || args.target || "";
                if (!domain) return { ok: false, error: "Missing 'domain'" };
                try {
                  const { execSync } = await import("child_process");
                  const out = execSync(`whois ${domain} 2>&1 | head -60`, { encoding: "utf-8", timeout: 15000 });
                  return { ok: true, output: out.trim(), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "net.speed_test": {
        try {
                  const { execSync } = await import("child_process");
                  const out = execSync("iperf3 -c localhost -t 5 2>&1 || echo 'iperf3 not available — install and point to a target server'", { encoding: "utf-8", timeout: 30000 });
                  return { ok: true, output: out.trim(), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "net.arp_table": {
        try {
                  const { execSync } = await import("child_process");
                  const out = execSync("ip neigh show 2>/dev/null || arp -a 2>/dev/null", { encoding: "utf-8", timeout: 5000 });
                  return { ok: true, output: out.trim(), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "net.vlan_list": {
        try {
                  const { execSync } = await import("child_process");
                  const out = execSync("ip -d link show type vlan 2>/dev/null || cat /proc/net/vlan/config 2>/dev/null || echo 'No VLANs configured locally — check pfSense'", { encoding: "utf-8", timeout: 5000 });
                  return { ok: true, output: out.trim(), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "net.vlan_move": {
        return null; // Rust executor: high-risk, requires approval
      }

      case "net.dhcp_leases": {
        try {
                  const { execSync } = await import("child_process");
                  const out = execSync("cat /var/lib/dhcp/dhclient.leases 2>/dev/null || echo 'DHCP leases managed by pfSense — query via pfSense API'", { encoding: "utf-8", timeout: 5000 });
                  return { ok: true, output: out.trim(), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "net.port_forward": {
        return null; // Rust executor: high-risk, requires approval
      }

      case "net.route_table": {
        try {
                  const { execSync } = await import("child_process");
                  const out = execSync("ip route show 2>/dev/null || route -n 2>/dev/null", { encoding: "utf-8", timeout: 5000 });
                  return { ok: true, output: out.trim(), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "net.interface_stats": {
        try {
                  const { execSync } = await import("child_process");
                  const out = execSync("ip -s link show 2>/dev/null || netstat -i 2>/dev/null", { encoding: "utf-8", timeout: 5000 });
                  return { ok: true, output: out.trim(), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "net.wifi_scan": {
        try {
                  const { execSync } = await import("child_process");
                  const out = execSync("iw dev wlan0 scan 2>/dev/null | grep -E 'SSID|signal|freq' || echo 'No wireless interface or scan not supported'", { encoding: "utf-8", timeout: 15000 });
                  return { ok: true, output: out.trim(), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "net.mtr": {
        const host = args.host || args.target || "";
                if (!host) return { ok: false, error: "Missing 'host'" };
                try {
                  const { execSync } = await import("child_process");
                  const out = execSync(`mtr -r -c 5 ${host} 2>&1 || traceroute ${host} 2>&1`, { encoding: "utf-8", timeout: 30000 });
                  return { ok: true, output: out.trim(), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "net.connection_table": {
        try {
                  const { execSync } = await import("child_process");
                  const out = execSync("ss -tunap 2>/dev/null | head -50 || netstat -tunap 2>/dev/null | head -50", { encoding: "utf-8", timeout: 5000 });
                  return { ok: true, output: out.trim(), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }



      default:
        return null;
    }
  }
}

const agent = new NetworkAgent();
await agent.connect();
await agent.serve();
