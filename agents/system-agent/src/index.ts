import { RedNodeAgent } from "../../shared/src/agent.js";

const TOOLS = [
  "fs.read",
  "process.list",
  "docker.ps",
  "service.status",
  "shell.run_safe",
];

class SystemAgent extends RedNodeAgent {
  constructor() {
    super("system", TOOLS);
  }

  async handleTool(tool: string, args: any): Promise<any> {
    switch (tool) {
      case "fs.read": {
        const p = args.path || "";
        // Double-check path security at agent level (Rust executor also checks)
        if (p.includes("..")) throw new Error("path traversal denied by agent policy");
        if (p.includes("/etc/shadow") || p.includes(".ssh/") || p.includes(".env"))
          throw new Error("sensitive file access denied by agent policy");
        // Fall through to Rust executor for sandboxed read
        return null;
      }

      case "process.list": {
        // Execute via Rust, then enrich the output
        const result = await this.callTool(tool, args);
        if (!result?.ok) return result;

        // Parse ps aux output and highlight high-CPU processes
        const lines = (result.stdout || result.output || "").split("\n");
        const header = lines[0] || "";
        const processes = lines.slice(1).filter((l: string) => l.trim());

        const highCpu = processes.filter((l: string) => {
          const parts = l.trim().split(/\s+/);
          const cpu = parseFloat(parts[2] || "0");
          return cpu > 50;
        });

        let enriched = `Total processes: ${processes.length}\n`;
        if (highCpu.length > 0) {
          enriched += `⚠️  High CPU processes (>50%):\n`;
          highCpu.forEach((l: string) => { enriched += `  ${l}\n`; });
        } else {
          enriched += `✅ No high-CPU processes\n`;
        }
        enriched += `\nTop 10 by CPU:\n${header}\n${processes.slice(0, 10).join("\n")}`;

        return { ok: true, output: enriched, tool, raw: result.stdout };
      }

      case "docker.ps": {
        const result = await this.callTool(tool, args);
        if (!result?.ok) return result;

        const output = result.stdout || result.output || "";
        const lines = output.split("\n").filter((l: string) => l.trim());

        // Detect unhealthy containers
        const unhealthy = lines.filter((l: string) =>
          l.toLowerCase().includes("unhealthy") ||
          l.toLowerCase().includes("exited") ||
          l.toLowerCase().includes("dead")
        );

        let enriched = `Docker containers: ${Math.max(0, lines.length - 1)}\n`;
        if (unhealthy.length > 0) {
          enriched += `⚠️  Unhealthy/stopped containers:\n`;
          unhealthy.forEach((l: string) => { enriched += `  ${l}\n`; });

          // Report to security events
          const CNS = process.env.REDNODE_CNS || "http://localhost:8787";
          for (const u of unhealthy) {
            try {
              await fetch(`${CNS}/security/events`, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({
                  severity: "MEDIUM",
                  source: "system-agent/docker",
                  summary: `Unhealthy container detected: ${u.trim().substring(0, 100)}`,
                  raw: { line: u },
                }),
              });
            } catch {}
          }
        } else {
          enriched += `✅ All containers healthy\n`;
        }
        enriched += `\n${output}`;

        return { ok: true, output: enriched, tool, unhealthy_count: unhealthy.length };
      }

      case "service.status": {
        // Pass through to Rust executor — it handles systemctl
        return null;
      }

      case "shell.run_safe": {
        // Pass through to Rust executor — it enforces the allowlist
        return null;
      }

      default:
        return null;
    }
  }
}

const agent = new SystemAgent();
await agent.connect();
await agent.serve();
