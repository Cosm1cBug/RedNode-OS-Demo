import { RedNodeAgent } from "../../shared/src/agent.js";
import { sh, api, llm, cns, pihole, truenas, frigate, ha } from "../../shared/src/helpers.js";

const PIHOLE_URL = process.env.PIHOLE_URL || "http://10.0.50.2";
const PIHOLE_PASSWORD = process.env.PIHOLE_PASSWORD || "";

const TOOLS = [
  "docker.images",
  "docker.logs",
  "docker.prune",
  "docker.restart",
  "pihole.add_block",
  "pihole.anomaly",
  "pihole.client_report",
  "pihole.cname_add",
  "pihole.disable",
  "pihole.dns_history",
  "pihole.enable",
  "pihole.gravity_update",
  "pihole.group_manage",
  "pihole.query_log",
  "pihole.regex_add",
  "pihole.regex_remove",
  "pihole.remove_block",
  "pihole.stats",
  "pihole.top_blocked",
  "pihole.top_clients",
  "pihole.whitelist_add",
  "pihole.whitelist_remove",
];

// ─── Pi-hole v6 API Session Management ───

let piholeSession: string | null = null;

async function piholeAuth(): Promise<string> {
  if (piholeSession) return piholeSession;
  try {
    const resp = await fetch(`${PIHOLE_URL}/api/auth`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ password: PIHOLE_PASSWORD }),
    });
    const data = (await resp.json()) as any;
    piholeSession = data?.session?.sid || null;
    if (!piholeSession) throw new Error("No session ID returned");
    // Auto-expire session after 5 minutes
    setTimeout(
      () => {
        piholeSession = null;
      },
      5 * 60 * 1000,
    );
    return piholeSession;
  } catch (e: any) {
    console.warn("[infra-agent] Pi-hole auth failed:", e.message);
    throw e;
  }
}

async function piholeGet(path: string): Promise<any> {
  const sid = await piholeAuth();
  const resp = await fetch(`${PIHOLE_URL}/api${path}?sid=${sid}`);
  if (!resp.ok)
    throw new Error(`Pi-hole API error: ${resp.status} ${resp.statusText}`);
  return resp.json();
}

async function piholePost(path: string, body: any): Promise<any> {
  const sid = await piholeAuth();
  const resp = await fetch(`${PIHOLE_URL}/api${path}?sid=${sid}`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!resp.ok)
    throw new Error(`Pi-hole API error: ${resp.status} ${resp.statusText}`);
  return resp.json();
}

// ─── Anomaly Detection ───

let lastQueryCount = 0;
let lastBlockedCount = 0;

function detectAnomalies(stats: any): string[] {
  const anomalies: string[] = [];
  const totalQueries = stats.queries?.total || 0;
  const blockedQueries = stats.queries?.blocked || 0;

  // Spike detection: >2x normal rate
  if (lastQueryCount > 0 && totalQueries > lastQueryCount * 2) {
    anomalies.push(
      `DNS query spike: ${totalQueries} (was ${lastQueryCount}) — possible malware C2 callback`,
    );
  }
  if (lastBlockedCount > 0 && blockedQueries > lastBlockedCount * 3) {
    anomalies.push(
      `Blocked query spike: ${blockedQueries} (was ${lastBlockedCount}) — device may be compromised`,
    );
  }

  lastQueryCount = totalQueries;
  lastBlockedCount = blockedQueries;

  // High block ratio
  const blockPct = totalQueries > 0 ? (blockedQueries / totalQueries) * 100 : 0;
  if (blockPct > 50) {
    anomalies.push(
      `Block ratio very high: ${blockPct.toFixed(1)}% — review blocklists or check for misconfiguration`,
    );
  }

  return anomalies;
}

// ─── Agent ───

class InfraAgent extends RedNodeAgent {
  constructor() {
    super("infra", TOOLS);
  }

  async handleTool(tool: string, args: any): Promise<any> {
    try {
      switch (tool) {
        case "pihole.stats": {
          const data = await piholeGet("/stats/summary");
          return {
            ok: true,
            output: JSON.stringify(data, null, 2),
            stats: data,
          };
        }

        case "pihole.top_blocked": {
          const data = await piholeGet("/stats/top_blocked");
          const domains = Object.entries(data?.top_blocked || {})
            .map(([domain, count]) => `${domain}: ${count}`)
            .join("\n");
          return { ok: true, output: domains || "No blocked domains", data };
        }

        case "pihole.top_clients": {
          const data = await piholeGet("/stats/top_clients");
          const clients = Object.entries(data?.top_clients || {})
            .map(([client, count]) => `${client}: ${count}`)
            .join("\n");
          return { ok: true, output: clients || "No client data", data };
        }

        case "pihole.query_log": {
          const limit = args.limit || 20;
          const data = await piholeGet(`/queries?limit=${limit}`);
          return {
            ok: true,
            output: JSON.stringify(data?.queries?.slice(0, limit), null, 2),
            data,
          };
        }

        case "pihole.disable": {
          const timer = args.timer || 300; // 5 minutes default
          const data = await piholePost("/dns/blocking", {
            blocking: false,
            timer,
          });
          return {
            ok: true,
            output: `Pi-hole blocking disabled for ${timer}s`,
            data,
          };
        }

        case "pihole.enable": {
          const data = await piholePost("/dns/blocking", { blocking: true });
          return { ok: true, output: "Pi-hole blocking re-enabled", data };
        }

        case "pihole.add_block": {
          const domain = args.domain;
          if (!domain) return { ok: false, error: "Missing domain argument" };
          // Add to local blacklist
          const data = await piholePost("/lists", {
            address: domain,
            type: "deny",
            comment: `Blocked by RedNode at ${new Date().toISOString()}`,
          });
          return { ok: true, output: `Blocked domain: ${domain}`, data };
        }

        case "pihole.remove_block": {
          const domain = args.domain;
          if (!domain) return { ok: false, error: "Missing domain argument" };
          // This requires finding the list ID first — simplified
          return {
            ok: true,
            output: `Unblock ${domain}: use Pi-hole admin UI for list management`,
          };
        }

        case "pihole.anomaly": {
          const stats = await piholeGet("/stats/summary");
          const anomalies = detectAnomalies(stats);
          if (anomalies.length > 0) {
            // Report anomalies as security events
            for (const a of anomalies) {
              await this.reportSecurityEvent("MEDIUM", a);
            }
            return {
              ok: true,
              output: `${anomalies.length} anomalies detected:\n${anomalies.join("\n")}`,
              anomalies,
            };
          }
          return {
            ok: true,
            output: "No DNS anomalies detected",
            anomalies: [],
          };
        }
      case "pihole.gravity_update": {
        const r = await pihole("updateGravity"); return { ok: r.ok, output: "Gravity update triggered", tool };
      }

      case "pihole.regex_add": {
        const regex = args.regex || args.pattern || ""; if (!regex) return { ok: false, error: "Missing regex pattern" }; const r = await pihole(`list=regex_black&add=${encodeURIComponent(regex)}`); return { ok: r.ok, output: `Regex added: ${regex}`, tool };
      }

      case "pihole.regex_remove": {
        const regex = args.regex || args.pattern || ""; if (!regex) return { ok: false, error: "Missing regex" }; const r = await pihole(`list=regex_black&sub=${encodeURIComponent(regex)}`); return { ok: r.ok, output: `Regex removed: ${regex}`, tool };
      }

      case "pihole.whitelist_add": {
        const domain = args.domain || ""; if (!domain) return { ok: false, error: "Missing domain" }; const r = await pihole(`list=white&add=${domain}`); return { ok: r.ok, output: `Whitelisted: ${domain}`, tool };
      }

      case "pihole.whitelist_remove": {
        const domain = args.domain || ""; if (!domain) return { ok: false, error: "Missing domain" }; const r = await pihole(`list=white&sub=${domain}`); return { ok: r.ok, output: `Removed from whitelist: ${domain}`, tool };
      }

      case "pihole.client_report": {
        const client = args.client || args.ip || "";
                if (!client) return { ok: false, error: "Missing 'client' IP" };
                const url = process.env.PIHOLE_URL || "http://10.0.50.2";
                try {
                  const res = await fetch(\`\${url}/admin/api.php?getQuerySources&client=\${client}\`);
                  const data = await res.json();
                  return { ok: true, output: JSON.stringify(data, null, 2), tool };
                } catch (e: any) { return { ok: false, error: \`Pi-hole API error: \${e.message}\` }; }
      }

      case "pihole.dns_history": {
        const url = process.env.PIHOLE_URL || "http://10.0.50.2";
                try {
                  const res = await fetch(\`\${url}/admin/api.php?overTimeData10mins\`);
                  const data = await res.json();
                  return { ok: true, output: JSON.stringify(data, null, 2), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "pihole.cname_add": {
        const domain = args.domain || ""; const target = args.target || ""; if (!domain || !target) return { ok: false, error: "Missing domain and target" }; const r = await pihole(`customcname&action=add&domain=${domain}&target=${target}`); return { ok: r.ok, output: `CNAME: ${domain} → ${target}`, tool };
      }

      case "pihole.group_manage": {
        const r = await pihole("groups"); return { ok: r.ok, output: r.output, tool };
      }

      case "docker.images": {
        try {
                  const { execSync } = await import("child_process");
                  const out = execSync("docker images --format 'table {{.Repository}}\t{{.Tag}}\t{{.Size}}\t{{.CreatedSince}}'", { encoding: "utf-8", timeout: 10000 });
                  return { ok: true, output: out.trim(), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "docker.logs": {
        const container = args.container || args.name || "";
                if (!container) return { ok: false, error: "Missing 'container' name" };
                try {
                  const { execSync } = await import("child_process");
                  const n = args.lines || 50;
                  const out = execSync(\`docker logs --tail \${n} \${container} 2>&1\`, { encoding: "utf-8", timeout: 10000 });
                  return { ok: true, output: out.trim(), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "docker.restart": {
        const container = args.container || args.name || ""; if (!container) return { ok: false, error: "Missing container name" }; const r = await sh(`docker restart ${container} 2>&1`); return { ok: r.ok, output: r.output, tool };
      }

      case "docker.prune": {
        const r = await sh("docker system prune -f 2>&1"); return { ok: r.ok, output: r.output, tool };
      }



        default:
          return null; // fall through to Rust executor
      }
    } catch (e: any) {
      console.error(`[infra-agent] ${tool} failed:`, e.message);
      return { ok: false, error: e.message };
    }
  }

  private async reportSecurityEvent(severity: string, summary: string) {
    const CNS = process.env.REDNODE_CNS || "http://localhost:8787";
    try {
      await fetch(`${CNS}/security/events`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          severity,
          source: "infra-agent/pihole",
          summary,
          raw: {},
        }),
      });
    } catch {}
  }
}

const agent = new InfraAgent();
await agent.connect();
await agent.serve();
