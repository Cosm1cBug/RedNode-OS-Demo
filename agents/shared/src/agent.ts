import { connect, NatsConnection, StringCodec, JSONCodec } from "nats";
import { loadConfig, watchConfigChanges } from "./config-loader.js";

const sc = StringCodec();
const jc = JSONCodec();

export interface ToolCall {
  tool: string;
  args?: any;
}
export interface AgentTask {
  tool: string;
  args?: any;
  intent?: string;
  session_id?: string;
  risk?: string;
}

export class RedNodeAgent {
  nc!: NatsConnection;
  name: string;
  private capabilities: Set<string>;

  constructor(name: string, capabilities: string[]) {
    this.name = name;
    this.capabilities = new Set(capabilities);
  }

  async connect(url = process.env.NATS_URL || "nats://127.0.0.1:4222") {
    // Load config from CNS BEFORE connecting
    // This populates process.env with values from the web dashboard config
    // If CNS is not reachable, process.env from .env file is used as fallback
    await loadConfig();

    this.nc = await connect({ servers: url, name: `${this.name}-agent` });
    console.log(`[${this.name}-agent] connected to ${url}`);

    // heartbeat
    setInterval(() => {
      this.nc.publish(
        `rednode.agent.${this.name}.heartbeat`,
        sc.encode(
          JSON.stringify({
            agent: this.name,
            ts: Date.now(),
            capabilities: [...this.capabilities],
          }),
        ),
      );
    }, 10_000);

    // Watch for config changes from the dashboard
    // When someone changes a setting in the web UI, this agent re-fetches
    watchConfigChanges(this.nc, () => {
      console.info(`[${this.name}-agent] Config reloaded from dashboard`);
    });
  }

  /** Execute a tool via the Rust executor (sandboxed) */
  async callTool(tool: string, args: any): Promise<any> {
    const CNS = process.env.REDNODE_CNS || "http://localhost:8787";
    try {
      const resp = await fetch(`${CNS}/exec`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ tool, args, agent: this.name }),
      });
      const result = await resp.json();
      return result;
    } catch (e: any) {
      throw new Error(`tool_exec ${tool} failed: ${e.message}`);
    }
  }

  async serve() {
    const subject = `rednode.agent.${this.name}.task`;
    const sub = this.nc.subscribe(subject);
    console.log(`[${this.name}-agent] listening on ${subject}`);

    // Listen for hot-reload signals from the evolution engine
    const reloadSub = this.nc.subscribe(`rednode.reload.${this.name}-agent`);
    (async () => {
      for await (const msg of reloadSub) {
        try {
          const data = jc.decode(msg.data) as any;
          console.info(
            `[${this.name}-agent] 🔄 Reload signal received: new tool '${data.tool || "unknown"}' evolved. ` +
            `Agent will use the updated handler on next call. ` +
            `(Full restart needed for handler code changes to take effect.)`
          );
        } catch {}
      }
    })();

    for await (const m of sub) {
      let task: AgentTask;
      try {
        task = jc.decode(m.data) as AgentTask;
      } catch {
        task = JSON.parse(sc.decode(m.data));
      }
      const start = Date.now();
      console.log(`[${this.name}] task: ${task.tool}`, task.args || {});
      try {
        // Agent-specific pre-processing hook
        const handled = await this.handleTool(task.tool, task.args || {});
        const result =
          handled ?? (await this.callTool(task.tool, task.args || {}));

        const response = {
          ok: true,
          agent: this.name,
          tool: task.tool,
          result,
          duration_ms: Date.now() - start,
        };
        if (m.reply) m.respond(jc.encode(response));
      } catch (err: any) {
        console.error(`[${this.name}] task failed:`, err.message);
        const response = {
          ok: false,
          agent: this.name,
          tool: task.tool,
          error: err.message,
        };
        if (m.reply) m.respond(jc.encode(response));
      }
    }
  }

  // Override in subclasses for agent-specific logic
  async handleTool(tool: string, args: any): Promise<any | null> {
    // return null to fall through to default callTool()
    return null;
  }
}
