import { connect, NatsConnection, StringCodec, JSONCodec } from "nats";
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
    }, 15000);
  }

  async callTool(tool: string, args: any = {}, actor?: string): Promise<any> {
    if (!this.capabilities.has(tool)) {
      console.warn(
        `[${this.name}] tool ${tool} not in capability list – forwarding anyway (policy enforced in Rust)`,
      );
    }
    const req = {
      tool,
      args,
      actor: actor || `${this.name}-agent`,
      agent: this.name,
      session_id: "default",
    };
    try {
      const msg = await this.nc.request("rednode.tool.exec", jc.encode(req), {
        timeout: 8000,
      });
      const resp = jc.decode(msg.data) as any;
      // Rust Executor response: {ok, tool, exit_code, stdout, stderr, risk, audit_id, sandbox}
      if (!resp.ok) {
        throw new Error(resp.stderr || "tool_exec failed");
      }
      // Normalize for backward compat – dashboard expects `output`
      return {
        ok: true,
        tool: resp.tool,
        output: resp.stdout,
        stdout: resp.stdout,
        stderr: resp.stderr,
        exit_code: resp.exit_code,
        risk: resp.risk,
        audit_id: resp.audit_id,
        sandbox: resp.sandbox,
      };
    } catch (e: any) {
      throw new Error(`tool_exec ${tool} failed: ${e.message}`);
    }
  }

  async serve() {
    const subject = `rednode.agent.${this.name}.task`;
    const sub = this.nc.subscribe(subject);
    console.log(`[${this.name}-agent] listening on ${subject}`);
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
