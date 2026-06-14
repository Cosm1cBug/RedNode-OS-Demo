import { RedNodeAgent } from "../../shared/src/agent.js";

const OLLAMA_URL = process.env.OLLAMA_URL || "http://127.0.0.1:11434";
const MODEL = process.env.REDNODE_CODE_MODEL || process.env.REDNODE_MODEL || "qwen2.5:14b-instruct-q4_K_M";
const TOOLS = ["code.generate", "code.test", "code.analyze", "code.refactor", "git.status"];

class CodingAgent extends RedNodeAgent {
  constructor() {
    super("coding", TOOLS);
  }

  async handleTool(tool: string, args: any): Promise<any> {
    switch (tool) {
      case "code.analyze": {
        // Run clippy for Rust, or eslint for TS — via Rust executor shell.run_safe
        const path = args.path || ".";
        const lang = args.language || "rust";

        if (lang === "rust") {
          try {
            const result = await this.callTool("shell.run_safe", { cmd: "cargo clippy" });
            return {
              ok: true,
              output: `Rust analysis (clippy):\n${result?.output || result?.stdout || "No issues found ✅"}`,
              tool,
            };
          } catch {
            return { ok: true, output: "Code analysis: clippy not available in sandbox — run manually: cargo clippy", tool };
          }
        }

        return { ok: true, output: `Code analysis for ${lang} at ${path}: use the appropriate linter (eslint, clippy, pylint)`, tool };
      }

      case "code.test": {
        const framework = args.framework || "cargo";
        try {
          if (framework === "cargo") {
            const result = await this.callTool("shell.run_safe", { cmd: "cargo test" });
            return { ok: true, output: `Test results:\n${result?.output || result?.stdout || "No output"}`, tool };
          }
          return { ok: true, output: `Run tests with: ${framework} test`, tool };
        } catch (e: any) {
          return { ok: false, error: `Tests failed: ${e.message}`, tool };
        }
      }

      case "code.generate": {
        const description = args.description || args.prompt || "";
        const language = args.language || "typescript";
        if (!description) return { ok: false, error: "Missing 'description' argument" };

        try {
          const resp = await fetch(`${OLLAMA_URL}/api/chat`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              model: MODEL,
              messages: [
                {
                  role: "system",
                  content: `You are an expert ${language} developer. Write clean, production-ready code. Include comments. No explanations outside the code block. Respond with ONLY the code.`,
                },
                { role: "user", content: description },
              ],
              stream: false,
              options: { temperature: 0.3, num_predict: 2048 },
            }),
          });
          const data = await resp.json() as any;
          const code = data.message?.content || "No code generated";
          return { ok: true, output: code, tool, language };
        } catch (e: any) {
          return { ok: false, error: `Code generation failed: ${e.message}` };
        }
      }

      case "code.refactor": {
        const code = args.code || "";
        const instruction = args.instruction || "improve code quality";
        if (!code) return { ok: false, error: "Missing 'code' argument" };

        try {
          const resp = await fetch(`${OLLAMA_URL}/api/chat`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              model: MODEL,
              messages: [
                {
                  role: "system",
                  content: "You are an expert code reviewer. Refactor the given code as instructed. Return ONLY the refactored code with comments explaining changes.",
                },
                { role: "user", content: `Instruction: ${instruction}\n\nCode:\n${code}` },
              ],
              stream: false,
              options: { temperature: 0.2, num_predict: 2048 },
            }),
          });
          const data = await resp.json() as any;
          return { ok: true, output: data.message?.content || "No output", tool };
        } catch (e: any) {
          return { ok: false, error: `Refactor failed: ${e.message}` };
        }
      }

      case "git.status": {
        try {
          const result = await this.callTool("shell.run_safe", { cmd: "git status" });
          return { ok: true, output: result?.output || result?.stdout || "Not a git repo", tool };
        } catch (e: any) {
          return { ok: false, error: e.message };
        }
      }

      default:
        return null;
    }
  }
}

const agent = new CodingAgent();
await agent.connect();
await agent.serve();
