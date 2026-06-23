import { RedNodeAgent } from "../../shared/src/agent.js";

const PIHOLE_URL = process.env.PIHOLE_URL || "http://10.0.50.2";
const PIHOLE_PASSWORD = process.env.PIHOLE_PASSWORD || "";

const TOOLS = [
  "pihole.stats",
  "pihole.top_blocked",
  "pihole.top_clients",
  "pihole.query_log",
  "pihole.disable",
  "pihole.enable",
  "pihole.add_block",
  "pihole.remove_block",
  "pihole.anomaly",
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
