#!/usr/bin/env node
import { Command } from "commander";

const program = new Command();
const CNS = process.env.REDNODE_CNS || "http://localhost:8787";
const TOKEN = process.env.REDNODE_API_TOKEN || "";

const headers: Record<string, string> = {
  "Content-Type": "application/json",
};
if (TOKEN) headers["Authorization"] = `Bearer ${TOKEN}`;

async function get(path: string) {
  const r = await fetch(`${CNS}${path}`, { headers });
  if (!r.ok) throw new Error(`${r.status} ${r.statusText}`);
  return r.json() as any;
}

async function post(path: string, body: any) {
  const r = await fetch(`${CNS}${path}`, {
    method: "POST",
    headers,
    body: JSON.stringify(body),
  });
  if (!r.ok) throw new Error(`${r.status} ${r.statusText}`);
  return r.json() as any;
}

function printJSON(data: any) {
  console.log(JSON.stringify(data, null, 2));
}

function printTable(rows: any[], keys: string[]) {
  if (rows.length === 0) {
    console.log("  (no data)");
    return;
  }
  for (const row of rows) {
    const parts = keys.map((k) => {
      const val = row[k];
      return val !== undefined && val !== null ? String(val) : "-";
    });
    console.log("  " + parts.join(" | "));
  }
}

// ─── Commands ───

program.name("rednode").description("RedNode-OS CLI – Personal Autonomous Operating System").version("0.3.1");

program
  .command("intent <text...>")
  .description("Submit an intention to the CNS")
  .action(async (text) => {
    const intent = text.join(" ");
    console.log(`\n🧠 Intent: "${intent}"\n`);
    const res = await post("/intent", { intent });
    console.log("📋 Plan:");
    for (const step of res.plan || []) {
      console.log(`  → ${step.tool} (${step.agent}) [${step.risk}]`);
    }
    console.log("\n📊 Results:");
    for (const r of res.results || []) {
      const status = r.status === "executed" ? "✅" : r.status === "needs_approval" ? "⏳" : "❌";
      console.log(`  ${status} ${r.tool}: ${r.status}`);
      if (r.result?.output) {
        const output = String(r.result.output).substring(0, 300);
        console.log(`     ${output}`);
      }
    }
  });

program
  .command("status")
  .description("Full system status – health, drives, agents, storage")
  .action(async () => {
    console.log("\n🧠 RedNode-OS Status\n");

    // Health
    const health = await get("/health");
    console.log(`  Node: ${health.node} v${health.version}`);
    console.log(`  Uptime: ${Math.floor((health.uptime_secs || 0) / 60)} minutes\n`);

    // Sentience
    const sentience = await get("/sentience");
    if (sentience.sentience && sentience.model) {
      const d = sentience.model.drives;
      console.log("  Drives:");
      const bar = (v: number) => {
        const filled = Math.round(v * 10);
        return "█".repeat(filled) + "░".repeat(10 - filled) + ` ${(v * 100).toFixed(0)}%`;
      };
      console.log(`    Security:     ${bar(d.security)}`);
      console.log(`    Integrity:    ${bar(d.integrity)}`);
      console.log(`    Knowledge:    ${bar(d.knowledge)}`);
      console.log(`    Energy:       ${bar(d.energy)}`);
      console.log(`    Availability: ${bar(d.availability)}`);

      const r = sentience.model.resources;
      console.log(`\n  Resources:`);
      console.log(`    CPU: ${r.cpu_percent?.toFixed(1)}% | RAM: ${r.mem_used_mb}/${r.mem_total_mb} MB | Disk: ${r.disk_used_pct?.toFixed(0)}%`);

      console.log(`\n  Goals executed: ${sentience.model.goals_executed}`);
      const pending = (sentience.model.goals || []).filter((g: any) => g.status === "pending" || g.status === "executing");
      if (pending.length > 0) {
        console.log(`  Active goals: ${pending.length}`);
        for (const g of pending) console.log(`    → [${g.drive}] ${g.description}`);
      }
    }

    // Agents
    const agents = await get("/agents/status");
    console.log("\n  Agents:");
    for (const a of agents.agents || []) {
      const icon = a.alive ? "✅" : a.status === "stale" ? "⚠️" : "❌";
      console.log(`    ${icon} ${a.name}: ${a.status} (tasks: ${a.tasks_completed || 0})`);
    }
    console.log("");
  });

program
  .command("health")
  .description("Quick health check")
  .action(async () => {
    const r = await get("/health");
    console.log(r);
  });

program
  .command("agents")
  .description("List agent status")
  .action(async () => {
    const r = await get("/agents/status");
    for (const a of r.agents || []) {
      const icon = a.alive ? "✅" : "❌";
      console.log(`${icon} ${a.name} — ${a.status} — tasks: ${a.tasks_completed || 0}`);
    }
  });

program
  .command("sentience")
  .description("Show sentience engine state – drives, goals, resources")
  .action(async () => {
    const r = await get("/sentience");
    printJSON(r.model || r);
  });

program
  .command("audit")
  .option("-n, --limit <n>", "number of entries", "20")
  .description("Show audit log")
  .action(async (opts) => {
    const r = await get(`/audit?limit=${opts.limit}`);
    for (const e of r.entries || []) {
      const hash = e.hash ? e.hash.substring(0, 8) + "…" : "-";
      console.log(`  ${e.ts?.substring(11, 19) || "?"} | ${e.actor} | ${e.action} | ${e.tool || "-"} | ${e.risk || "-"} | ${hash}`);
    }
  });

program
  .command("security")
  .description("Show security events")
  .action(async () => {
    const r = await get("/security/events");
    const events = r.events || [];
    if (events.length === 0) {
      console.log("No security events ✅");
      return;
    }
    for (const e of events.slice(0, 20)) {
      const icon = e.severity === "CRITICAL" ? "🔴" : e.severity === "HIGH" ? "🟠" : e.severity === "MEDIUM" ? "🟡" : "🟢";
      const ack = e.acknowledged ? "✓" : "○";
      console.log(`  ${icon} [${ack}] ${e.severity} | ${e.source} | ${e.summary?.substring(0, 80)}`);
    }
  });

program
  .command("approvals")
  .description("Show pending approvals")
  .action(async () => {
    const r = await get("/approvals");
    const approvals = r.approvals || [];
    if (approvals.length === 0) {
      console.log("No pending approvals ✅");
      return;
    }
    for (const a of approvals) {
      console.log(`  ⏳ ${a.id} | ${a.tool} | risk: ${a.risk} | ${a.intent || "-"}`);
    }
  });

program
  .command("approve <id>")
  .description("Approve a pending action")
  .action(async (id) => {
    const r = await post(`/approvals/${id}/approve`, { approved: true });
    console.log(r.ok ? `✅ Approved: ${id}` : `❌ Failed: ${r.error}`);
  });

program
  .command("deny <id>")
  .description("Deny a pending action")
  .action(async (id) => {
    const r = await post(`/approvals/${id}/approve`, { approved: false });
    console.log(r.ok ? `🚫 Denied: ${id}` : `❌ Failed: ${r.error}`);
  });

program
  .command("memory <query...>")
  .description("Search RedNode memory (RAG)")
  .action(async (query) => {
    const q = query.join(" ");
    const r = await get(`/memory/query?q=${encodeURIComponent(q)}&limit=5`);
    for (const hit of r.results || []) {
      console.log(`  [${(hit.score * 100).toFixed(0)}%] ${hit.source}: ${hit.content.substring(0, 150)}`);
    }
  });

program
  .command("ingest <source> <content...>")
  .description("Ingest knowledge into memory")
  .action(async (source, content) => {
    const r = await post("/memory/ingest", { source, content: content.join(" ") });
    console.log(r.ok ? `✅ Ingested: ${r.id}` : `❌ Failed: ${r.error}`);
  });

// ─── Workflow shortcuts ───

program
  .command("goodnight")
  .description("Run goodnight workflow — night mode for your home")
  .action(async () => {
    console.log("🌙 Running goodnight workflow...\n");
    const r = await post("/intent", { intent: "run workflow goodnight", session_id: "cli" });
    for (const res of r.results || []) console.log(`  ${res.status === "executed" ? "✅" : "❌"} ${res.tool}`);
  });

program
  .command("morning")
  .description("Run morning brief — overnight summary")
  .action(async () => {
    console.log("☀️ Running morning brief...\n");
    const r = await post("/intent", { intent: "run workflow morning", session_id: "cli" });
    for (const res of r.results || []) {
      console.log(`  ${res.status === "executed" ? "✅" : "❌"} ${res.tool}`);
      if (res.result?.output) console.log(`     ${String(res.result.output).substring(0, 200)}`);
    }
  });

program
  .command("focus")
  .description("Run focus mode — block distractions")
  .action(async () => {
    console.log("🎯 Activating focus mode...\n");
    const r = await post("/intent", { intent: "run workflow focus", session_id: "cli" });
    for (const res of r.results || []) console.log(`  ${res.status === "executed" ? "✅" : "❌"} ${res.tool}`);
  });

// ─── Infrastructure shortcuts ───

program
  .command("cameras")
  .description("Show camera status and recent events")
  .action(async () => {
    const r = await post("/intent", { intent: "show camera status and recent person detections" });
    for (const res of r.results || []) {
      if (res.result?.output) console.log(res.result.output);
    }
  });

program
  .command("nas")
  .description("Show TrueNAS status")
  .action(async () => {
    const r = await post("/intent", { intent: "check TrueNAS pool health and disk status" });
    for (const res of r.results || []) {
      if (res.result?.output) console.log(res.result.output);
    }
  });

program
  .command("pihole")
  .description("Show Pi-hole DNS stats")
  .action(async () => {
    const r = await post("/intent", { intent: "show pihole DNS stats" });
    for (const res of r.results || []) {
      if (res.result?.output) console.log(res.result.output);
    }
  });

program
  .command("emails")
  .description("Show recent emails summary")
  .action(async () => {
    const r = await post("/intent", { intent: "summarize my recent emails" });
    for (const res of r.results || []) {
      if (res.result?.output) console.log(res.result.output);
    }
  });

program.parse();
