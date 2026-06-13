import { RedNodeAgent } from "../../shared/src/agent.js";

const TOOLS = [
  "fs.read",
  "process.list",
  "docker.ps",
  "service.status",
  "shell.run_safe"
];

class SystemAgent extends RedNodeAgent {
  constructor() { super("system", TOOLS); }
  async handleTool(tool: string, args: any) {
    // System-specific pre-validation
    if (tool === "fs.read") {
      const p = args.path || "";
      if (p.includes("..")) throw new Error("path traversal denied by agent policy");
    }
    return null; // fall through to central executor
  }
}

const agent = new SystemAgent();
await agent.connect();
await agent.serve();
