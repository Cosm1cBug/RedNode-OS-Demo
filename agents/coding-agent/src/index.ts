import { RedNodeAgent } from "../../shared/src/agent.js";
const TOOLS = ["code.generate","code.test","code.analyze","code.refactor","git.status"];
class CodingAgent extends RedNodeAgent {
  constructor(){ super("coding", TOOLS); }
}
const agent = new CodingAgent();
await agent.connect();
await agent.serve();
