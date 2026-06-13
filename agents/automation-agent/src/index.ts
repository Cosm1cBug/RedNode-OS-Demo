import { RedNodeAgent } from "../../shared/src/agent.js";
const TOOLS = ["workflow.create","workflow.run","schedule.add","trigger.fire"];
class AutomationAgent extends RedNodeAgent {
  constructor(){ super("automation", TOOLS); }
}
const agent = new AutomationAgent();
await agent.connect();
await agent.serve();
