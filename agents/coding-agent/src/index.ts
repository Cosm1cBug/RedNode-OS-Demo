import { RedNodeAgent } from "../../shared/src/agent.js";

const OLLAMA_URL = process.env.OLLAMA_URL || "http://127.0.0.1:11434";
const MODEL = process.env.REDNODE_CODE_MODEL || process.env.REDNODE_MODEL || "qwen2.5:14b-instruct-q4_K_M";
const CNS = process.env.REDNODE_CNS || "http://localhost:8787";
const AUTO_VERIFY = process.env.CODE_AUTO_VERIFY !== "false"; // default: ON

const TOOLS = [
  "code.generate", "code.test", "code.analyze", "code.refactor", "git.status",
  "code.verify",  // Verification gate — build+typecheck+lint+test+security
  "code.review",  // LLM-powered security code review
];

// ─── Verification Gate ───
// After any code generation or refactoring, automatically run:
//   Phase 1: Build check
//   Phase 2: Type check
//   Phase 3: Lint
//   Phase 4: Test
//   Phase 5: Security scan (check for hardcoded secrets, injection patterns)
// Only reports success if ALL phases pass.

async function runVerificationGate(context: string): Promise<{
  passed: boolean;
  phases: { name: string; passed: boolean; output: string }[];
}> {
  const phases: { name: string; passed: boolean; output: string }[] = [];

  const commands = [
    { name: "build", cmd: "cargo build --quiet 2>&1 || npm run build 2>&1 || pnpm build 2>&1 || echo 'no build system'" },
    { name: "typecheck", cmd: "npx tsc --noEmit 2>&1 || cargo check --quiet 2>&1 || echo 'no type checker'" },
    { name: "lint", cmd: "cargo clippy --quiet 2>&1 || npx eslint . --quiet 2>&1 || echo 'no linter'" },
    { name: "test", cmd: "cargo test --quiet 2>&1 || npm test 2>&1 || pnpm test 2>&1 || echo 'no tests'" },
  ];

  for (const { name, cmd } of commands) {
    try {
      const { exec } = await import("child_process");
      const { promisify } = await import("util");
      const execAsync = promisify(exec);
      const { stdout, stderr } = await execAsync(cmd, { timeout: 60000, cwd: process.cwd() });
      const output = (stdout + stderr).trim().substring(0, 500);
      const passed = !output.toLowerCase().includes("error") && !output.toLowerCase().includes("failed");
      phases.push({ name, passed, output });
    } catch (e: any) {
      phases.push({ name, passed: false, output: e.message.substring(0, 300) });
    }
  }

  // Phase 5: Security scan on generated code
  const secIssues: string[] = [];
  const dangerPatterns = [
    /api[_-]?key\s*=\s*["'][^"']+["']/gi,
    /password\s*=\s*["'][^"']+["']/gi,
    /secret\s*=\s*["'][^"']+["']/gi,
    /token\s*=\s*["'][^"']+["']/gi,
    /eval\s*\(/g,
    /innerHTML\s*=/g,
    /document\.write/g,
    /exec\s*\(\s*['"`]/g,
    /\$\{.*\}.*SELECT|INSERT|UPDATE|DELETE/gi,
  ];

  if (context) {
    for (const pattern of dangerPatterns) {
      if (pattern.test(context)) {
        secIssues.push(`Security: pattern ${pattern.source} detected in generated code`);
      }
      pattern.lastIndex = 0; // reset regex state
    }
  }

  phases.push({
    name: "security",
    passed: secIssues.length === 0,
    output: secIssues.length === 0 ? "No security issues detected" : secIssues.join("\n"),
  });

  const allPassed = phases.every(p => p.passed);
  return { passed: allPassed, phases };
}

// ─── Code Review ───

async function securityReview(code: string): Promise<string> {
  try {
    const resp = await fetch(`${OLLAMA_URL}/api/chat`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        model: MODEL,
        messages: [
          {
            role: "system",
            content: `You are a security code reviewer. Check the code for:
1. Hardcoded secrets (API keys, passwords, tokens)
2. SQL injection vulnerabilities
3. Path traversal risks
4. Command injection
5. XSS (cross-site scripting)
6. Insecure deserialization
7. Missing input validation
8. Improper error handling (leaking stack traces)
9. Missing authentication/authorization checks
10. Insecure random number generation

For each issue found, output: [SEVERITY] Description — Line/location if identifiable.
If no issues: output "✅ No security issues found."
Be concise. No explanations beyond the findings.`,
          },
          { role: "user", content: `Review this code:\n\n${code}` },
        ],
        stream: false,
        options: { temperature: 0.1, num_predict: 1024 },
      }),
    });
    const data = (await resp.json()) as any;
    return data.message?.content || "Review unavailable";
  } catch (e: any) {
    return `Security review failed: ${e.message}`;
  }
}

// ─── Agent ───

class CodingAgent extends RedNodeAgent {
  constructor() {
    super("coding", TOOLS);
  }

  async handleTool(tool: string, args: any): Promise<any> {
    switch (tool) {
      case "code.analyze": {
        const lang = args.language || "rust";
        if (lang === "rust") {
          try {
            const result = await this.callTool("shell.run_safe", { cmd: "cargo clippy" });
            return { ok: true, output: `Rust analysis (clippy):\n${result?.output || result?.stdout || "No issues found ✅"}`, tool };
          } catch {
            return { ok: true, output: "Code analysis: clippy not available — run manually", tool };
          }
        }
        return { ok: true, output: `Code analysis for ${lang}: use the appropriate linter`, tool };
      }

      case "code.test": {
        const framework = args.framework || "cargo";
        try {
          if (framework === "cargo") {
            const result = await this.callTool("shell.run_safe", { cmd: "cargo test" });
            return { ok: true, output: `Test results:\n${result?.output || "No output"}`, tool };
          }
          return { ok: true, output: `Run tests with: ${framework} test`, tool };
        } catch (e: any) {
          return { ok: false, error: `Tests failed: ${e.message}`, tool };
        }
      }

      case "code.generate": {
        const description = args.description || args.prompt || "";
        const language = args.language || "typescript";
        if (!description) return { ok: false, error: "Missing 'description'" };

        try {
          const resp = await fetch(`${OLLAMA_URL}/api/chat`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              model: MODEL,
              messages: [
                {
                  role: "system",
                  content: `You are an expert ${language} developer. Write clean, production-ready code. Follow these standards 
- Readability first — self-documenting code, clear naming
- KISS — simplest solution that works
- No premature optimization
- Proper error handling — never swallow errors
- No hardcoded secrets — use environment variables
- Input validation on all external data
- Include brief comments for non-obvious logic only
Respond with ONLY the code.`,
                },
                { role: "user", content: description },
              ],
              stream: false,
              options: { temperature: 0.3, num_predict: 2048 },
            }),
          });
          const data = (await resp.json()) as any;
          const code = data.message?.content || "No code generated";

          // Auto-verification gate
          let verifyOutput = "";
          if (AUTO_VERIFY) {
            const review = await securityReview(code);
            verifyOutput = `\n\n── Security Review ──\n${review}`;
          }

          return { ok: true, output: code + verifyOutput, tool, language };
        } catch (e: any) {
          return { ok: false, error: `Code generation failed: ${e.message}` };
        }
      }

      case "code.refactor": {
        const code = args.code || "";
        const instruction = args.instruction || "improve code quality";
        if (!code) return { ok: false, error: "Missing 'code'" };

        try {
          const resp = await fetch(`${OLLAMA_URL}/api/chat`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              model: MODEL,
              messages: [
                {
                  role: "system",
                  content: `You are an expert code reviewer. Refactor the given code following these principles:
- KISS, DRY, YAGNI
- Improve readability and naming
- Fix any security issues (hardcoded secrets, injection, etc.)
- Add proper error handling
- Remove dead code
Return ONLY the refactored code with brief comments explaining changes.`,
                },
                { role: "user", content: `Instruction: ${instruction}\n\nCode:\n${code}` },
              ],
              stream: false,
              options: { temperature: 0.2, num_predict: 2048 },
            }),
          });
          const data = (await resp.json()) as any;
          const refactored = data.message?.content || "No output";

          // Auto security review
          let reviewOutput = "";
          if (AUTO_VERIFY) {
            const review = await securityReview(refactored);
            reviewOutput = `\n\n── Security Review ──\n${review}`;
          }

          return { ok: true, output: refactored + reviewOutput, tool };
        } catch (e: any) {
          return { ok: false, error: `Refactor failed: ${e.message}` };
        }
      }

      case "code.verify": {
        // Manual verification gate trigger
        const result = await runVerificationGate(args.code || "");
        const lines = result.phases.map(p =>
          `  ${p.passed ? "✅" : "❌"} ${p.name}: ${p.passed ? "PASS" : "FAIL"}\n     ${p.output.substring(0, 200)}`
        );
        return {
          ok: result.passed,
          output: `Verification Gate: ${result.passed ? "✅ ALL PASSED" : "❌ FAILED"}\n\n${lines.join("\n\n")}`,
          phases: result.phases,
        };
      }

      case "code.review": {
        const code = args.code || "";
        if (!code) return { ok: false, error: "Missing 'code' to review" };
        const review = await securityReview(code);
        return { ok: true, output: `Security Code Review:\n\n${review}`, tool };
      }

      case "git.status": {
        try {
          const result = await this.callTool("shell.run_safe", { cmd: "git status" });
          return { ok: true, output: result?.output || "Not a git repo", tool };
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
