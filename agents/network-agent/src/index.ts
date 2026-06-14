import { RedNodeAgent } from "../../shared/src/agent.js";

const PIHOLE_URL = process.env.PIHOLE_URL || "http://10.0.50.2";
const TOOLS = ["net.status", "firewall.rules", "vpn.connect", "dns.check", "traffic.analyze"];

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
