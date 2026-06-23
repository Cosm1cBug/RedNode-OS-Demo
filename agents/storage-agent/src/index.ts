import { RedNodeAgent } from "../../shared/src/agent.js";

const TRUENAS_URL = process.env.TRUENAS_URL || "https://10.0.50.3";
const TRUENAS_API_KEY = process.env.TRUENAS_API_KEY || "";

const TOOLS = [
  "nas.health",
  "nas.pools",
  "nas.datasets",
  "nas.usage",
  "nas.disks",
  "nas.smart",
  "nas.alerts",
  "nas.snapshot_create",
  "nas.snapshot_list",
  "nas.snapshot_delete",
  "nas.share_create",
  "nas.share_list",
  "nas.replicate",
  "nas.backup_rednode",
];

// ─── TrueNAS REST API v2.0 ───

async function nasGet(path: string): Promise<any> {
  const resp = await fetch(`${TRUENAS_URL}/api/v2.0${path}`, {
    headers: {
      Authorization: `Bearer ${TRUENAS_API_KEY}`,
      "Content-Type": "application/json",
    },
    // TrueNAS often uses self-signed certs
    // @ts-ignore — Node 18+ supports this
  });
  if (!resp.ok)
    throw new Error(`TrueNAS API error: ${resp.status} ${resp.statusText}`);
  return resp.json();
}

async function nasPost(path: string, body: any): Promise<any> {
  const resp = await fetch(`${TRUENAS_URL}/api/v2.0${path}`, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${TRUENAS_API_KEY}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify(body),
  });
  if (!resp.ok)
    throw new Error(`TrueNAS API error: ${resp.status} ${resp.statusText}`);
  return resp.json();
}

async function nasDelete(path: string): Promise<any> {
  const resp = await fetch(`${TRUENAS_URL}/api/v2.0${path}`, {
    method: "DELETE",
    headers: { Authorization: `Bearer ${TRUENAS_API_KEY}` },
  });
  if (!resp.ok)
    throw new Error(`TrueNAS API error: ${resp.status} ${resp.statusText}`);
  return resp.json();
}

// ─── Helpers ───

function formatBytes(bytes: number): string {
  if (bytes === 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB", "PB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`;
}

function poolHealthSummary(pools: any[]): string {
  return pools
    .map((p: any) => {
      const used = formatBytes(p.topology?.data?.[0]?.stats?.allocated || 0);
      const total = formatBytes(p.topology?.data?.[0]?.stats?.size || 0);
      return `${p.name}: ${p.status} | ${p.healthy ? "✅ Healthy" : "❌ DEGRADED"} | ${used} / ${total}`;
    })
    .join("\n");
}

// ─── Agent ───

class StorageAgent extends RedNodeAgent {
  constructor() {
    super("storage", TOOLS);
  }

  async handleTool(tool: string, args: any): Promise<any> {
    try {
      switch (tool) {
        case "nas.health":
        case "nas.pools": {
          const pools = await nasGet("/pool");
          const summary = poolHealthSummary(pools);
          // Report unhealthy pools as security events
          for (const p of pools) {
            if (!p.healthy) {
              await this.reportSecurityEvent(
                "HIGH",
                `TrueNAS pool '${p.name}' is DEGRADED — status: ${p.status}`,
              );
            }
          }
          return { ok: true, output: summary, pools };
        }

        case "nas.datasets":
        case "nas.usage": {
          const datasets = await nasGet("/pool/dataset");
          const lines = datasets.map((d: any) => {
            const used = formatBytes(d.used?.parsed || 0);
            const avail = formatBytes(d.available?.parsed || 0);
            const pct =
              d.used?.parsed && d.available?.parsed
                ? (
                    (d.used.parsed / (d.used.parsed + d.available.parsed)) *
                    100
                  ).toFixed(0)
                : "?";
            return `${d.name}: ${used} used / ${avail} free (${pct}%)`;
          });
          return { ok: true, output: lines.join("\n"), datasets };
        }

        case "nas.disks": {
          const disks = await nasGet("/disk");
          const lines = disks.map(
            (d: any) =>
              `${d.name} | ${d.model || "unknown"} | ${formatBytes(d.size || 0)} | Serial: ${d.serial || "?"} | Temp: ${d.temperature || "?"}°C`,
          );
          return { ok: true, output: lines.join("\n"), disks };
        }

        case "nas.smart": {
          const results = await nasGet("/smart/test/results");
          const lines = (results || []).map(
            (r: any) =>
              `${r.disk}: ${r.status} — ${r.description || "no details"}`,
          );
          return {
            ok: true,
            output:
              lines.length > 0
                ? lines.join("\n")
                : "No SMART test results available. Run: nas.smart_test",
            results,
          };
        }

        case "nas.alerts": {
          const alerts = await nasGet("/alert/list");
          if (!alerts || alerts.length === 0) {
            return {
              ok: true,
              output: "No active TrueNAS alerts ✅",
              alerts: [],
            };
          }
          const lines = alerts.map(
            (a: any) => `[${a.level}] ${a.formatted || a.text || a.title}`,
          );
          // Report critical alerts
          for (const a of alerts) {
            if (a.level === "CRITICAL" || a.level === "ERROR") {
              await this.reportSecurityEvent(
                "HIGH",
                `TrueNAS alert: ${a.formatted || a.text}`,
              );
            }
          }
          return { ok: true, output: lines.join("\n"), alerts };
        }

        case "nas.snapshot_create": {
          const dataset = args.dataset;
          if (!dataset)
            return {
              ok: false,
              error: "Missing 'dataset' argument (e.g. 'tank/documents')",
            };
          const name =
            args.name ||
            `rednode-auto-${new Date().toISOString().replace(/[:.]/g, "-")}`;
          const result = await nasPost("/zfs/snapshot", {
            dataset,
            name,
            recursive: args.recursive ?? true,
          });
          return {
            ok: true,
            output: `Snapshot created: ${dataset}@${name}`,
            result,
          };
        }

        case "nas.snapshot_list": {
          const snapshots = await nasGet("/zfs/snapshot");
          const dataset = args.dataset;
          const filtered = dataset
            ? snapshots.filter((s: any) => s.name.startsWith(dataset))
            : snapshots;
          const lines = filtered
            .slice(-20) // last 20
            .map(
              (s: any) =>
                `${s.name} | ${new Date(s.properties?.creation?.parsed * 1000 || 0).toLocaleString()} | ${formatBytes(s.properties?.used?.parsed || 0)}`,
            );
          return {
            ok: true,
            output: lines.join("\n") || "No snapshots found",
            count: filtered.length,
          };
        }

        case "nas.snapshot_delete": {
          const id = args.id || args.name;
          if (!id)
            return { ok: false, error: "Missing 'id' or 'name' argument" };
          const encodedId = encodeURIComponent(id);
          const result = await nasDelete(`/zfs/snapshot/id/${encodedId}`);
          return { ok: true, output: `Snapshot deleted: ${id}`, result };
        }

        case "nas.share_create": {
          const path = args.path;
          const name = args.name;
          if (!path || !name)
            return {
              ok: false,
              error: "Missing 'path' and/or 'name' arguments",
            };
          const result = await nasPost("/sharing/smb", {
            path,
            name,
            comment: `Created by RedNode at ${new Date().toISOString()}`,
            browsable: true,
            ro: false,
          });
          return {
            ok: true,
            output: `SMB share created: \\\\truenas\\${name} → ${path}`,
            result,
          };
        }

        case "nas.share_list": {
          const smb = await nasGet("/sharing/smb");
          const nfs = await nasGet("/sharing/nfs");
          const lines = [
            "=== SMB Shares ===",
            ...(smb || []).map(
              (s: any) =>
                `  ${s.name}: ${s.path} ${s.enabled ? "✅" : "❌ disabled"}`,
            ),
            "=== NFS Exports ===",
            ...(nfs || []).map(
              (s: any) =>
                `  ${s.paths?.join(", ")} ${s.enabled ? "✅" : "❌ disabled"}`,
            ),
          ];
          return { ok: true, output: lines.join("\n"), smb, nfs };
        }

        case "nas.replicate": {
          const jobs = await nasGet("/replication");
          if (!jobs || jobs.length === 0) {
            return { ok: true, output: "No replication jobs configured" };
          }
          // Trigger first enabled job
          const job = jobs.find((j: any) => j.enabled);
          if (!job) return { ok: true, output: "No enabled replication jobs" };
          const result = await nasPost(`/replication/id/${job.id}/run`, {});
          return {
            ok: true,
            output: `Replication job '${job.name}' triggered`,
            result,
          };
        }

        case "nas.backup_rednode": {
          // Create a snapshot of RedNode data on TrueNAS
          const dataset = args.dataset || "tank/backups";
          const name = `rednode-brain-${new Date().toISOString().slice(0, 10)}`;
          const result = await nasPost("/zfs/snapshot", {
            dataset,
            name,
            recursive: true,
          });
          return {
            ok: true,
            output: `RedNode brain backup snapshot: ${dataset}@${name}\nNote: Postgres pg_dump + Qdrant snapshot should run before this via automation workflow`,
            result,
          };
        }

        default:
          return null; // fall through to Rust executor
      }
    } catch (e: any) {
      console.error(`[storage-agent] ${tool} failed:`, e.message);
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
          source: "storage-agent/truenas",
          summary,
          raw: {},
        }),
      });
    } catch {}
  }
}

const agent = new StorageAgent();
await agent.connect();
await agent.serve();
