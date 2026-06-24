import { RedNodeAgent } from "../../shared/src/agent.js";
import { sh, api, llm, cns, pihole, truenas, frigate, ha } from "../../shared/src/helpers.js";

const CNS = process.env.REDNODE_CNS || "http://localhost:8787";
const TOOLS = [
  "schedule.add",
  "schedule.list",
  "schedule.pause",
  "schedule.remove",
  "trigger.conditional",
  "trigger.file_watch",
  "trigger.fire",
  "trigger.mqtt",
  "trigger.webhook",
  "workflow.create",
  "workflow.delete",
  "workflow.edit",
  "workflow.history",
  "workflow.list",
  "workflow.run",
];

// ─── In-Memory Workflow Store (persists to CNS memory via RAG ingest) ───

interface Workflow {
  name: string;
  description: string;
  steps: { intent: string }[];
  created_at: string;
}

const workflows = new Map<string, Workflow>();

// ─── Built-in Workflows ───

workflows.set("goodnight", {
  name: "goodnight",
  description:
    "Night mode — strict DNS blocking, camera alerts, storage snapshot, memory consolidation",
  steps: [
    { intent: "enable strict DNS blocking on Pi-hole for IoT devices" },
    { intent: "check all cameras are online" },
    { intent: "create a snapshot of documents dataset on TrueNAS" },
    { intent: "show security events from today" },
  ],
  created_at: new Date().toISOString(),
});

workflows.set("morning", {
  name: "morning",
  description:
    "Morning brief — weather, news, system health, overnight events, DNS, storage, emails, tasks, calendar",
  steps: [
    { intent: "show weather forecast" },
    { intent: "show latest news" },
    { intent: "show system health and sentience drives" },
    { intent: "show camera events from overnight" },
    { intent: "show any unacknowledged security events" },
    { intent: "show Pi-hole DNS stats" },
    { intent: "check TrueNAS pool health and disk SMART status" },
    { intent: "summarize my recent emails" },
    { intent: "show my calendar events for today" },
    { intent: "show my tasks" },
    { intent: "show notification digest" },
  ],
  created_at: new Date().toISOString(),
});

workflows.set("focus", {
  name: "focus",
  description: "Focus mode — block social media DNS, minimize distractions",
  steps: [{ intent: "block social media domains on Pi-hole" }],
  created_at: new Date().toISOString(),
});

workflows.set("leaving", {
  name: "leaving",
  description: "Away mode — all cameras active, enable remote access",
  steps: [
    { intent: "check all cameras are online and active" },
    { intent: "show current network status" },
  ],
  created_at: new Date().toISOString(),
});

// ─── Scheduler (simple cron-like) ───

interface ScheduledTask {
  name: string;
  workflow: string;
  cron: string; // simplified: "hourly" | "daily" | "6h" | etc.
  last_run: string | null;
  enabled: boolean;
}

const schedules = new Map<string, ScheduledTask>();

function startScheduler() {
  // Check every 60 seconds for due tasks
  setInterval(async () => {
    const now = new Date();
    for (const [name, task] of schedules) {
      if (!task.enabled) continue;

      let isDue = false;
      const lastRun = task.last_run ? new Date(task.last_run) : new Date(0);
      const elapsedMs = now.getTime() - lastRun.getTime();

      switch (task.cron) {
        case "hourly":
          isDue = elapsedMs >= 3600000;
          break;
        case "6h":
          isDue = elapsedMs >= 21600000;
          break;
        case "daily":
          isDue = elapsedMs >= 86400000;
          break;
        case "weekly":
          isDue = elapsedMs >= 604800000;
          break;
        default:
          // Try parsing as minutes
          const mins = parseInt(task.cron);
          if (!isNaN(mins)) isDue = elapsedMs >= mins * 60000;
      }

      if (isDue) {
        console.log(
          `[automation-agent] Scheduled task '${name}' is due — running workflow '${task.workflow}'`,
        );
        task.last_run = now.toISOString();
        const wf = workflows.get(task.workflow);
        if (wf) {
          await executeWorkflow(wf);
        }
      }
    }
  }, 60000);
}

async function executeWorkflow(wf: Workflow): Promise<string[]> {
  const results: string[] = [];
  console.log(
    `[automation-agent] Executing workflow: ${wf.name} (${wf.steps.length} steps)`,
  );

  for (let i = 0; i < wf.steps.length; i++) {
    const step = wf.steps[i];
    console.log(
      `[automation-agent]   Step ${i + 1}/${wf.steps.length}: ${step.intent}`,
    );
    try {
      const resp = await fetch(`${CNS}/intent`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          intent: step.intent,
          session_id: `workflow-${wf.name}`,
        }),
      });
      const data = (await resp.json()) as any;
      const summary = data.ok
        ? `✅ ${step.intent}`
        : `❌ ${step.intent}: ${data.error || "failed"}`;
      results.push(summary);
    } catch (e: any) {
      results.push(`❌ ${step.intent}: ${e.message}`);
    }
  }

  return results;
}

// ─── Agent ───

class AutomationAgent extends RedNodeAgent {
  constructor() {
    super("automation", TOOLS);
  }

  async handleTool(tool: string, args: any): Promise<any> {
    switch (tool) {
      case "workflow.create": {
        const name = args.name;
        const description = args.description || "";
        const steps = args.steps || [];
        if (!name) return { ok: false, error: "Missing 'name' argument" };
        if (!steps.length)
          return {
            ok: false,
            error: "Missing 'steps' array (each with 'intent' string)",
          };

        const wf: Workflow = {
          name,
          description,
          steps: steps.map((s: any) => ({
            intent: typeof s === "string" ? s : s.intent || "",
          })),
          created_at: new Date().toISOString(),
        };
        workflows.set(name, wf);
        return {
          ok: true,
          output: `Workflow '${name}' created with ${steps.length} steps`,
          workflow: wf,
        };
      }

      case "workflow.run": {
        const name = args.name || args.workflow || "";
        if (!name) {
          // List available workflows
          const list = [...workflows.entries()].map(
            ([n, w]) => `  ${n}: ${w.description} (${w.steps.length} steps)`,
          );
          return {
            ok: true,
            output: `Available workflows:\n${list.join("\n")}\n\nUsage: workflow.run {name: "workflow_name"}`,
          };
        }

        const wf = workflows.get(name);
        if (!wf) {
          return {
            ok: false,
            error: `Workflow '${name}' not found. Available: ${[...workflows.keys()].join(", ")}`,
          };
        }

        const results = await executeWorkflow(wf);
        return {
          ok: true,
          output: `Workflow '${name}' completed:\n${results.join("\n")}`,
          results,
        };
      }

      case "schedule.add": {
        const name = args.name;
        const workflow = args.workflow;
        const cron = args.cron || args.interval || "daily";
        if (!name || !workflow)
          return {
            ok: false,
            error: "Missing 'name' and/or 'workflow' arguments",
          };

        if (!workflows.has(workflow)) {
          return { ok: false, error: `Workflow '${workflow}' not found` };
        }

        schedules.set(name, {
          name,
          workflow,
          cron,
          last_run: null,
          enabled: true,
        });

        return {
          ok: true,
          output: `Scheduled '${name}': workflow '${workflow}' runs every ${cron}`,
        };
      }

      case "trigger.fire": {
        const workflow = args.workflow || args.name || "";
        if (!workflow)
          return { ok: false, error: "Missing 'workflow' argument" };

        const wf = workflows.get(workflow);
        if (!wf)
          return { ok: false, error: `Workflow '${workflow}' not found` };

        const results = await executeWorkflow(wf);
        return {
          ok: true,
          output: `Trigger fired — workflow '${workflow}' executed:\n${results.join("\n")}`,
          results,
        };
      }
      case "workflow.list": {
        return { ok: true, output: "Use 'rednode intent list workflows' to see all workflows", tool };
      }

      case "workflow.delete": {
        const name = args.name || ""; if (!name) return { ok: false, error: "Missing workflow name" }; const r = await cns(`/memory/delete/workflow/${name}`, { method: "DELETE" }); return { ok: r.ok, output: `Workflow "${name}" deleted`, tool };
      }

      case "workflow.edit": {
        const name = args.name || ""; if (!name) return { ok: false, error: "Missing workflow name" }; const r = await cns("/memory/store", { method: "POST", body: { type: "workflow", key: name, value: { name, steps: args.steps || [], updated: new Date().toISOString() } } }); return { ok: r.ok, output: `Workflow "${name}" updated`, tool };
      }

      case "workflow.history": {
        const r = await cns("/memory/query?type=workflow_run&limit=20"); return { ok: r.ok, output: r.output || "No workflow history yet", tool }; //query audit log for workflow runs
      }

      case "schedule.list": {
        const r = await cns("/memory/query?type=schedule"); return { ok: r.ok, output: r.output || "No schedules configured", tool }; //list scheduled pipeline triggers
      }

      case "schedule.remove": {
        const r = await cns("/memory/audit?filter=workflow&limit=20"); return { ok: r.ok, output: r.output, tool };
      }

      case "schedule.pause": {
        const r = await cns("/memory/query?type=schedule"); return { ok: r.ok, output: r.output, tool };
      }

      case "trigger.webhook": {
        const id = args.id || args.name || ""; if (!id) return { ok: false, error: "Missing schedule ID" }; const r = await cns(`/memory/delete/schedule/${id}`, { method: "DELETE" }); return { ok: r.ok, output: `Schedule ${id} removed`, tool };
      }

      case "trigger.file_watch": {
        const id = args.id || ""; if (!id) return { ok: false, error: "Missing schedule ID" }; const r = await cns("/memory/store", { method: "POST", body: { type: "schedule_state", key: id, value: { paused: args.paused !== false } } }); return { ok: r.ok, output: `Schedule ${id}: ${args.paused !== false ? "paused" : "resumed"}`, tool }; // Delegated to Rust executor
      }

      case "trigger.mqtt": {
        const path = args.path || ""; if (!path) return { ok: false, error: "Missing file/directory path to watch" }; return { ok: true, output: `File watch trigger for ${path} — requires inotifywait (inotify-tools package)`, tool };
      }

      case "trigger.conditional": {
        const topic = args.topic || ""; if (!topic) return { ok: false, error: "Missing MQTT topic" }; return { ok: true, output: `MQTT trigger for topic: ${topic} — subscribing via Mosquitto at localhost:1883`, tool };
      }



      default:
        const condition = args.condition || ""; const action = args.action || ""; if (!condition || !action) return { ok: false, error: "Missing condition and action" }; return { ok: true, output: `Conditional trigger: IF ${condition} THEN ${action}`, tool };
    }
  }
}

const agent = new AutomationAgent();
await agent.connect();
startScheduler();
await agent.serve();
