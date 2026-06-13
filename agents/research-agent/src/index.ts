import { RedNodeAgent } from "../../shared/src/agent.js";
const TOOLS = ["research.search","research.query","kb.query","kb.ingest"];
class ResearchAgent extends RedNodeAgent {
  constructor(){ super("research", TOOLS); }
  async handleTool(tool: string, args: any) {
    // Enrich with Qdrant / Kuzu context – Phase 2
    return null;
  }
}
const agent = new ResearchAgent();
await agent.connect();
await agent.serve();
