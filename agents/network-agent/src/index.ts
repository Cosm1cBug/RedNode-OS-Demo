import { RedNodeAgent } from "../../shared/src/agent.js";

const PIHOLE_URL = process.env.PIHOLE_URL || "http://10.0.50.2";
const TOOLS = ["net.status", "firewall.rules", "vpn.connect", "dns.check", "traffic.analyze", "net.scan", "net.devices"];

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

        const lines = (result.stdout || result.output || "").split("\n").filter((l: string) => l.trim());
        const listening = lines.filter((l: string) => l.includes("LISTEN"));
        const established = lines.filter((l: string) => l.includes("ESTAB"));

        let enriched = `Network connections:\n`;
        enriched += `  Listening ports: ${listening.length}\n`;
        enriched += `  Established connections: ${established.length}\n\n`;
        enriched += `Listening:\n`;
        listening.forEach((l: string) => { enriched += `  ${l}\n`; });

        if (established.length > 0) {
          enriched += `\nEstablished (first 10):\n`;
          established.slice(0, 10).forEach((l: string) => { enriched += `  ${l}\n`; });
        }

        return { ok: true, output: enriched, tool, listening: listening.length, established: established.length };
      }

      case "dns.check": {
        // Check if Pi-hole is responding
        let piholeOk = false;
        let piholeStats: any = null;
        try {
          const resp = await fetch(`${PIHOLE_URL}/api/stats/summary`, { signal: AbortSignal.timeout(3000) });
          if (resp.ok) {
            piholeOk = true;
            piholeStats = await resp.json();
          }
        } catch {}

        // Also check external DNS resolution via the Rust executor
        const digResult = await this.callTool("shell.run_safe", { cmd: "date" }).catch(() => null);

        let output = `DNS Status:\n`;
        output += `  Pi-hole (${PIHOLE_URL}): ${piholeOk ? "✅ Online" : "❌ OFFLINE"}\n`;
        if (piholeStats) {
          output += `  Queries today: ${piholeStats.queries?.total || "?"}\n`;
          output += `  Blocked: ${piholeStats.queries?.blocked || "?"} (${piholeStats.queries?.percent_blocked?.toFixed(1) || "?"}%)\n`;
        }
        output += `  System time: ${digResult?.output?.trim() || "unknown"}\n`;

        return { ok: true, output, pihole_online: piholeOk, stats: piholeStats };
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
        if (!/^(10\.|192\.168\.|172\.(1[6-9]|2[0-9]|3[01])\.)/.test(target) && target !== "localhost") {
          return { ok: false, error: "net.scan only allowed on private networks (10.x, 192.168.x, 172.16-31.x)" };
        }

        const nmapArgs: Record<string, string> = {
          quick: `-sn ${target}`,             // Ping scan — fast, just find live hosts
          ports: `-sT --top-ports 100 ${target}`, // Top 100 TCP ports
          full: `-sV -O --top-ports 1000 ${target}`, // Service version + OS detection
          vuln: `--script vuln --top-ports 100 ${target}`, // Vulnerability scan
        };

        const cmd = nmapArgs[scanType] || nmapArgs.quick;

        try {
          const { exec } = await import("child_process");
          const { promisify } = await import("util");
          const execAsync = promisify(exec);

          const { stdout, stderr } = await execAsync(`nmap ${cmd} 2>&1`, { timeout: 120000 });
          const output = stdout + stderr;

          // Parse results for structured output
          const hosts: { ip: string; hostname: string; ports: string[]; os: string }[] = [];
          let currentHost: any = null;

          for (const line of output.split("\n")) {
            const hostMatch = line.match(/Nmap scan report for (\S+)\s*\(?(\d+\.\d+\.\d+\.\d+)?\)?/);
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
              currentHost.ports.push(`${portMatch[1]}/${portMatch[2]} ${portMatch[3]} ${portMatch[4]}`);
            }
            if (line.includes("OS details:") && currentHost) {
              currentHost.os = line.replace("OS details:", "").trim();
            }
          }
          if (currentHost) hosts.push(currentHost);

          // Ingest scan results into memory
          const CNS = process.env.REDNODE_CNS || "http://localhost:8787";
          const summary = hosts.map(h =>
            `${h.ip}${h.hostname ? ` (${h.hostname})` : ""}: ${h.ports.length} ports${h.os ? `, OS: ${h.os}` : ""}`
          ).join("\n");

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
            return { ok: false, error: "nmap not installed. Install: apt install nmap / nix-env -i nmap" };
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

          const { stdout } = await execAsync("ip neigh show 2>/dev/null || arp -a 2>/dev/null", { timeout: 5000 });
          const devices: { ip: string; mac: string; state: string }[] = [];

          for (const line of stdout.trim().split("\n")) {
            // ip neigh format: 10.0.50.1 dev enp0s31f6 lladdr aa:bb:cc:dd:ee:ff REACHABLE
            const match = line.match(/^(\d+\.\d+\.\d+\.\d+)\s.*lladdr\s+([0-9a-f:]+)\s+(\w+)/i);
            if (match) {
              devices.push({ ip: match[1], mac: match[2], state: match[3] });
            }
            // arp -a format: ? (10.0.50.1) at aa:bb:cc:dd:ee:ff [ether] on enp0s31f6
            const arpMatch = line.match(/\((\d+\.\d+\.\d+\.\d+)\)\s+at\s+([0-9a-f:]+)/i);
            if (arpMatch) {
              devices.push({ ip: arpMatch[1], mac: arpMatch[2], state: "reachable" });
            }
          }

          const lines = devices.map(d => `  ${d.ip.padEnd(16)} ${d.mac.padEnd(18)} ${d.state}`);
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

      default:
        return null;
    }
  }
}

const agent = new NetworkAgent();
await agent.connect();
await agent.serve();
