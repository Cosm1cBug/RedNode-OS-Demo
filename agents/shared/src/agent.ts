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

// ─── Multi-Agent Society: Reputation + Confidence + Work Queue ───

export interface AgentMetrics {
  /** Reputation score (0-100) based on task success rate */
  reputation: number;
  /** Self-assessed confidence in current capabilities (0-100) */
  confidence: number;
  /** Total tasks completed successfully */
  tasks_completed: number;
  /** Total tasks failed */
  tasks_failed: number;
  /** Average task duration in ms */
  avg_duration_ms: number;
  /** Currently queued tasks awaiting execution */
  work_queue_size: number;
}

export class RedNodeAgent {
  nc!: NatsConnection;
  name: string;
  private capabilities: Set<string>;

  // ─── Agent Society Metrics ───
  metrics: AgentMetrics = {
    reputation: 80,
    confidence: 80,
    tasks_completed: 0,
    tasks_failed: 0,
    avg_duration_ms: 0,
    work_queue_size: 0,
  };
  private durationHistory: number[] = [];
  private workQueue: AgentTask[] = [];

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

    // heartbeat — now includes reputation and confidence
    setInterval(() => {
      this.nc.publish(
        `rednode.agent.${this.name}.heartbeat`,
        sc.encode(
          JSON.stringify({
            agent: this.name,
            ts: Date.now(),
            capabilities: [...this.capabilities],
            reputation: this.metrics.reputation,
            confidence: this.metrics.confidence,
            tasks_completed: this.metrics.tasks_completed,
            tasks_failed: this.metrics.tasks_failed,
            avg_duration_ms: this.metrics.avg_duration_ms,
            work_queue_size: this.workQueue.length,
          }),
        ),
      );
    }, 10_000);

    // Watch for config changes from the dashboard
    // When someone changes a setting in the web UI, this agent re-fetches
    watchConfigChanges(this.nc, () => {
      console.info(`[${this.name}-agent] Config reloaded from dashboard`);
    });

    // Listen for peer-review requests from other agents
    const peerSub = this.nc.subscribe(`rednode.agent.${this.name}.peer_review`);
    (async () => {
      for await (const msg of peerSub) {
        try {
          const request = jc.decode(msg.data) as any;
          const review = await this.peerReview(request);
          if (msg.reply) msg.respond(jc.encode(review));
        } catch (e: any) {
          if (msg.reply) {
            msg.respond(jc.encode({ ok: false, error: e.message }));
          }
        }
      }
    })();

    // Listen for metric queries
    const metricSub = this.nc.subscribe(`rednode.agent.${this.name}.metrics`);
    (async () => {
      for await (const msg of metricSub) {
        if (msg.reply) {
          msg.respond(jc.encode({
            ok: true,
            agent: this.name,
            metrics: this.metrics,
          }));
        }
      }
    })();
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

  /** Request a peer review from another agent */
  async requestPeerReview(targetAgent: string, data: any): Promise<any> {
    try {
      const subject = `rednode.agent.${targetAgent}.peer_review`;
      const resp = await this.nc.request(subject, jc.encode(data), { timeout: 5000 });
      return jc.decode(resp.data);
    } catch (e: any) {
      console.warn(`[${this.name}] Peer review from ${targetAgent} failed: ${e.message}`);
      return null;
    }
  }

  /** Handle a peer review request — override in subclasses for domain-specific review */
  async peerReview(request: any): Promise<any> {
    return {
      ok: true,
      reviewer: this.name,
      verdict: "no_opinion",
      message: "Agent does not implement domain-specific peer review",
    };
  }

  /** Update metrics after a task completes */
  private recordTaskResult(success: boolean, durationMs: number) {
    if (success) {
      this.metrics.tasks_completed++;
    } else {
      this.metrics.tasks_failed++;
    }

    // Update average duration
    this.durationHistory.push(durationMs);
    if (this.durationHistory.length > 100) {
      this.durationHistory.shift();
    }
    this.metrics.avg_duration_ms = Math.round(
      this.durationHistory.reduce((a, b) => a + b, 0) / this.durationHistory.length
    );

    // Recalculate reputation (weighted success rate)
    const total = this.metrics.tasks_completed + this.metrics.tasks_failed;
    if (total > 0) {
      const successRate = this.metrics.tasks_completed / total;
      // Smooth towards actual success rate (EMA)
      this.metrics.reputation = Math.round(
        this.metrics.reputation * 0.9 + successRate * 100 * 0.1
      );
      this.metrics.reputation = Math.max(0, Math.min(100, this.metrics.reputation));
    }

    // Confidence adjusts based on recent performance
    if (success) {
      this.metrics.confidence = Math.min(100, this.metrics.confidence + 1);
    } else {
      this.metrics.confidence = Math.max(10, this.metrics.confidence - 3);
    }

    this.metrics.work_queue_size = this.workQueue.length;
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
            `[${this.name}-agent] Reload signal received: new tool '${data.tool || "unknown"}' evolved. ` +
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

      // Add to work queue
      this.workQueue.push(task);
      this.metrics.work_queue_size = this.workQueue.length;

      const start = Date.now();
      console.log(`[${this.name}] task: ${task.tool}`, task.args || {});
      try {
        // Agent-specific pre-processing hook
        const handled = await this.handleTool(task.tool, task.args || {});
        const result =
          handled ?? (await this.callTool(task.tool, task.args || {}));

        const durationMs = Date.now() - start;
        this.recordTaskResult(true, durationMs);

        // Remove from work queue
        this.workQueue = this.workQueue.filter(t => t !== task);

        const response = {
          ok: true,
          agent: this.name,
          tool: task.tool,
          result,
          duration_ms: durationMs,
          reputation: this.metrics.reputation,
        };
        if (m.reply) m.respond(jc.encode(response));
      } catch (err: any) {
        const durationMs = Date.now() - start;
        this.recordTaskResult(false, durationMs);

        // Remove from work queue
        this.workQueue = this.workQueue.filter(t => t !== task);

        console.error(`[${this.name}] task failed:`, err.message);
        const response = {
          ok: false,
          agent: this.name,
          tool: task.tool,
          error: err.message,
          reputation: this.metrics.reputation,
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
