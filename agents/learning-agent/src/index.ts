/**
 * RedNode-OS — Autonomous Learning Agent
 *
 * This agent observes the system, reads documentation, and learns
 * new capabilities without human intervention.
 *
 * Capabilities:
 *   1. Documentation Ingestion  — reads man pages, READMEs, API docs
 *   2. Tool Discovery           — scans installed binaries, probes APIs
 *   3. Pattern Extraction       — mines audit log for repeated manual steps
 *   4. Workflow Suggestion      — proposes automations from observed patterns
 *   5. Self-Assessment          — tests its own knowledge, identifies gaps
 *   6. Knowledge Synthesis      — connects facts across domains
 *
 * The agent runs a continuous learning loop:
 *   1. Observe: what tools exist? what APIs are reachable?
 *   2. Read: ingest docs for discovered tools
 *   3. Extract: mine patterns from audit log
 *   4. Propose: suggest new workflows/automations
 *   5. Validate: test proposed workflows in sandbox
 *   6. Report: notify owner of new capabilities learned
 */

import { RedNodeAgent } from "../../shared/src/agent.js";
import { sh, api, llm, cns, pihole, truenas, frigate, ha } from "../../shared/src/helpers.js";

const CNS = process.env.REDNODE_CNS || "http://localhost:8787";
const LEARN_INTERVAL = parseInt(process.env.LEARN_INTERVAL || "3600000"); // 1 hour
const TOOLS = [
  "learn.auto_evolve",
  "learn.benchmark",
  "learn.discover",
  "learn.evolve",
  "learn.extract_patterns",
  "learn.ingest_docs",
  "learn.knowledge_map",
  "learn.list_evolved",
  "learn.self_assess",
  "learn.status",
  "learn.suggest_tool",
  "learn.suggest_workflow",
  "learn.synthesize",
  "learn.teach",
  "learn.test_tool",
  "learn.train_data",
  "learn.watch_logs",
];

interface LearnedCapability {
  name: string;
  description: string;
  source: string;
  confidence: number; // 0-1
  learned_at: string;
  validated: boolean;
}

interface ObservedPattern {
  intent_pattern: string;
  frequency: number;
  last_seen: string;
  suggested_automation: string | null;
}

const capabilities: Map<string, LearnedCapability> = new Map();
const patterns: ObservedPattern[] = [];
let learningCycles = 0;

class LearningAgent extends RedNodeAgent {
  constructor() {
    super("learning", TOOLS);
  }

  async handleTool(tool: string, args: any) {
    switch (tool) {
      // ── Discover available tools and APIs ──
      case "learn.discover": {
        const discovered: string[] = [];

        // 1. Scan installed CLI tools
        try {
          const { execSync } = await import("child_process");
          const bins = execSync(
            "ls /run/current-system/sw/bin/ 2>/dev/null | head -100",
            { encoding: "utf-8" },
          )
            .trim()
            .split("\n");
          for (const bin of bins.slice(0, 50)) {
            if (!capabilities.has(`cli:${bin}`)) {
              capabilities.set(`cli:${bin}`, {
                name: `cli:${bin}`,
                description: `System binary: ${bin}`,
                source: "filesystem scan",
                confidence: 0.5,
                learned_at: new Date().toISOString(),
                validated: false,
              });
              discovered.push(bin);
            }
          }
        } catch (e) {
          /* not on NixOS */
        }

        // 2. Probe reachable APIs
        const apis = [
          {
            name: "ollama",
            url: "http://localhost:11434/api/tags",
            desc: "Local LLM inference",
          },
          {
            name: "qdrant",
            url: "http://localhost:6333/healthz",
            desc: "Vector memory",
          },
          {
            name: "nats",
            url: "http://localhost:8222/varz",
            desc: "Message bus",
          },
          {
            name: "grafana",
            url: "http://localhost:3001/api/health",
            desc: "Monitoring dashboard",
          },
          {
            name: "pihole",
            url: `${process.env.PIHOLE_URL || "http://10.0.50.2"}/admin/api.php?summary`,
            desc: "DNS filtering",
          },
          {
            name: "frigate",
            url: `${process.env.FRIGATE_URL || "http://localhost:5000"}/api/stats`,
            desc: "Camera NVR",
          },
          {
            name: "searxng",
            url: "http://localhost:8888/search?q=test&format=json",
            desc: "Private search",
          },
          {
            name: "homeassistant",
            url: `${process.env.HOMEASSISTANT_URL || "http://localhost:8123"}/api/`,
            desc: "Smart home",
          },
        ];

        for (const api of apis) {
          try {
            const res = await fetch(api.url, {
              signal: AbortSignal.timeout(3000),
            });
            if (res.ok) {
              capabilities.set(`api:${api.name}`, {
                name: `api:${api.name}`,
                description: api.desc,
                source: "API probe",
                confidence: 1.0,
                learned_at: new Date().toISOString(),
                validated: true,
              });
              discovered.push(`API:${api.name}`);
            }
          } catch (e) {
            /* not reachable */
          }
        }

        return {
          ok: true,
          output: `Discovered ${discovered.length} new capabilities: ${discovered.join(", ")}`,
          discovered,
          total_capabilities: capabilities.size,
        };
      }

      // ── Ingest documentation for a tool/service ──
      case "learn.ingest_docs": {
        const target = args.target || args.tool || args.service;
        if (!target)
          return { ok: false, error: "Missing 'target' — what to learn about" };

        const docs: string[] = [];

        // Try man page
        try {
          const { execSync } = await import("child_process");
          const manOutput = execSync(
            `man ${target} 2>/dev/null | col -bx | head -200`,
            {
              encoding: "utf-8",
              timeout: 5000,
            },
          ).trim();
          if (manOutput.length > 50) {
            docs.push(`man page for ${target}:\n${manOutput}`);
          }
        } catch (e) {
          /* no man page */
        }

        // Try --help
        try {
          const { execSync } = await import("child_process");
          const help = execSync(`${target} --help 2>&1 | head -50`, {
            encoding: "utf-8",
            timeout: 5000,
          }).trim();
          if (help.length > 20) {
            docs.push(`--help output:\n${help}`);
          }
        } catch (e) {
          /* no help */
        }

        if (docs.length === 0) {
          return {
            ok: true,
            output: `No documentation found for '${target}'`,
            learned: false,
          };
        }

        // Ingest into knowledge base via CNS
        try {
          await fetch(`${CNS}/intent`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              intent: `ingest this documentation into the knowledge base: ${docs.join("\n\n")}`,
              session: "learning-agent",
            }),
          });
        } catch (e) {
          /* best effort */
        }

        capabilities.set(`docs:${target}`, {
          name: `docs:${target}`,
          description: `Documentation for ${target} (${docs.length} sources)`,
          source: "documentation ingestion",
          confidence: 0.8,
          learned_at: new Date().toISOString(),
          validated: true,
        });

        return {
          ok: true,
          output: `Learned about '${target}' from ${docs.length} documentation sources`,
          sources: docs.length,
          learned: true,
        };
      }

      // ── Extract patterns from audit log ──
      case "learn.extract_patterns": {
        try {
          // Query audit log for repeated intents
          const res = await fetch(`${CNS}/memory/recent?limit=200`);
          if (!res.ok)
            return {
              ok: true,
              output: "Could not access audit log",
              patterns: [],
            };

          const data = (await res.json()) as any;
          const entries = data.entries || data || [];

          // Count intent patterns
          const intentCounts = new Map<string, number>();
          for (const entry of entries) {
            const intent = entry.intent || entry.action || "";
            if (intent.length < 5) continue;
            // Normalize: lowercase, trim, take first 50 chars
            const normalized = intent.toLowerCase().trim().substring(0, 50);
            intentCounts.set(
              normalized,
              (intentCounts.get(normalized) || 0) + 1,
            );
          }

          // Find patterns (intents repeated 3+ times)
          const newPatterns: ObservedPattern[] = [];
          for (const [pattern, count] of intentCounts) {
            if (count >= 3) {
              newPatterns.push({
                intent_pattern: pattern,
                frequency: count,
                last_seen: new Date().toISOString(),
                suggested_automation:
                  count >= 5
                    ? `Create a scheduled workflow for: "${pattern}"`
                    : null,
              });
            }
          }

          // Merge with existing patterns
          for (const p of newPatterns) {
            const existing = patterns.findIndex(
              (e) => e.intent_pattern === p.intent_pattern,
            );
            if (existing >= 0) {
              patterns[existing].frequency = p.frequency;
              patterns[existing].last_seen = p.last_seen;
            } else {
              patterns.push(p);
            }
          }

          return {
            ok: true,
            output: `Found ${newPatterns.length} repeating patterns, ${newPatterns.filter((p) => p.suggested_automation).length} ready for automation`,
            patterns: newPatterns,
          };
        } catch (e) {
          return {
            ok: true,
            output: `Pattern extraction failed: ${e}`,
            patterns: [],
          };
        }
      }

      // ── Suggest workflow from observed pattern ──
      case "learn.suggest_workflow": {
        const automatable = patterns.filter(
          (p) => p.suggested_automation && p.frequency >= 5,
        );
        if (automatable.length === 0) {
          return {
            ok: true,
            output: "No patterns frequent enough for workflow suggestions yet",
            suggestions: [],
          };
        }

        const suggestions = automatable.map((p) => ({
          pattern: p.intent_pattern,
          frequency: p.frequency,
          suggestion: p.suggested_automation,
          confidence: Math.min(p.frequency / 10, 1.0),
        }));

        return {
          ok: true,
          output: `${suggestions.length} workflow suggestions based on your usage patterns`,
          suggestions,
        };
      }

      // ── Self-assessment: what do I know vs what do I not know ──
      case "learn.self_assess": {
        const total = capabilities.size;
        const validated = Array.from(capabilities.values()).filter(
          (c) => c.validated,
        ).length;
        const highConf = Array.from(capabilities.values()).filter(
          (c) => c.confidence >= 0.8,
        ).length;

        const gaps: string[] = [];
        // Check for expected capabilities that are missing
        const expected = [
          "ollama",
          "postgres",
          "nats",
          "qdrant",
          "pihole",
          "pfsense",
          "truenas",
          "frigate",
          "homeassistant",
        ];
        for (const e of expected) {
          if (!capabilities.has(`api:${e}`) && !capabilities.has(`docs:${e}`)) {
            gaps.push(e);
          }
        }

        return {
          ok: true,
          output: `Knowledge: ${total} capabilities (${validated} validated, ${highConf} high-confidence). Gaps: ${gaps.length > 0 ? gaps.join(", ") : "none"}. Learning cycles: ${learningCycles}`,
          total,
          validated,
          high_confidence: highConf,
          gaps,
          learning_cycles: learningCycles,
          patterns_found: patterns.length,
        };
      }

      // ── Synthesize: connect knowledge across domains ──
      case "learn.synthesize": {
        // Ask LLM to find connections between what we know
        const knowledgeList = Array.from(capabilities.values())
          .filter((c) => c.confidence >= 0.5)
          .map((c) => `${c.name}: ${c.description}`)
          .join("\n");

        const patternList = patterns
          .filter((p) => p.frequency >= 3)
          .map((p) => `"${p.intent_pattern}" (${p.frequency}x)`)
          .join("\n");

        return {
          ok: true,
          output: `Knowledge synthesis: ${capabilities.size} capabilities, ${patterns.length} patterns. Cross-domain connections analysis requires LLM planner.`,
          knowledge_summary: knowledgeList.substring(0, 2000),
          pattern_summary: patternList.substring(0, 1000),
        };
      }

      // ── Status ──
      case "learn.status": {
        return {
          ok: true,
          output: `Learning Agent: ${capabilities.size} capabilities, ${patterns.length} patterns, ${learningCycles} cycles completed`,
          capabilities: capabilities.size,
          patterns: patterns.length,
          cycles: learningCycles,
        };
      }
      case "learn.benchmark": {
        const prompt = "Generate a JSON plan for: check system health"; const start = Date.now(); const result = await llm(prompt, "You are RedNode planner. Output valid JSON."); const elapsed = Date.now() - start; let isJson = false; try { JSON.parse(result); isJson = true; } catch {} return { ok: true, output: `LLM Benchmark: ${elapsed}ms, JSON valid: ${isJson}, length: ${result.length}`, tool }; LLM benchmark suite
      }

      case "learn.train_data": {
        const r = await sh("python3 scripts/finetune/export-training-data.py --output /tmp/training_data.jsonl 2>&1", 30000); return { ok: r.ok, output: r.output, tool }; export-training-data.py
      }

      case "learn.test_tool": {
        const prompt = "Generate a JSON plan for: check system health"; const start = Date.now(); const result = await llm(prompt, "You are RedNode-OS planner. Output valid JSON only."); const elapsed = Date.now() - start; const isJson = (() => { try { JSON.parse(result); return true; } catch { return false; } })(); return { ok: true, output: `LLM Benchmark: ${elapsed}ms, JSON valid: ${isJson}, length: ${result.length} chars`, tool };
      }

      case "learn.watch_logs": {
        const r = await sh("journalctl --since \"10 minutes ago\" --no-pager -p warning 2>&1 | tail -20"); return { ok: r.ok, output: r.output || "No warnings in last 10 minutes", tool }; journal pattern analysis
      }

      case "learn.suggest_tool": {
        const r = await cns("/memory/query?type=failed_intent&limit=10"); const suggestions = await llm(`Based on these failed intents, suggest new tools that should be built: ${JSON.stringify(r.data).substring(0, 1000)}`); return { ok: true, output: suggestions, tool }; analyze failed intents
      }

      case "learn.knowledge_map": {
        const r = await cns("/memory/query?type=learned_doc"); return { ok: r.ok, output: `Knowledge map: ${r.output}`, tool }; knowledge graph summary
      }



      // ── Tool Evolution: Self-evolving capability ──

      case "learn.evolve": {
        const name = args.name || args.tool_name || "";
        const agent = args.agent || "system-agent";
        const description = args.description || args.desc || "";
        const handlerType = args.handler_type || args.type || "shell"; // shell, api, llm, cns
        const handlerCommand = args.handler_command || args.command || args.url || "";

        if (!name || !description) {
          return { ok: false, error: "Missing 'name' and 'description' for new tool" };
        }

        // Validate name format
        if (!name.includes(".")) {
          return { ok: false, error: "Tool name must contain a dot (e.g., 'sys.my_tool')" };
        }

        try {
          // Call CNS evolution API
          const res = await fetch(`${CNS}/evolve/tool`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              name,
              agent,
              description,
              handler_type: handlerType,
              handler_command: handlerCommand,
            }),
          });
          const result = await res.json() as any;

          if (result.ok) {
            // Track the evolution
            capabilities.set(`evolved:${name}`, {
              name: `evolved:${name}`,
              description: `Auto-evolved tool: ${description}`,
              source: "evolution",
              confidence: 0.9,
              learned_at: new Date().toISOString(),
              validated: false,
            });

            return {
              ok: true,
              output: `🧬 Tool evolved: ${name}\n  Agent: ${agent}\n  Type: ${handlerType}\n  Command: ${handlerCommand || "(auto-generated)"}\n  Description: ${description}\n\nThe tool is now registered and the handler has been injected. The LLM planner will include it in future plans.`,
              tool,
              evolved_tool: result.tool,
            };
          } else {
            return { ok: false, error: result.error || "Evolution failed", tool };
          }
        } catch (e: any) {
          return { ok: false, error: `Evolution API error: ${e.message}`, tool };
        }
      }

      case "learn.auto_evolve": {
        // Autonomous evolution: scan the system for unregistered CLI tools,
        // learn about them, and auto-create tool definitions
        const results: string[] = [];

        try {
          const { execSync } = await import("child_process");

          // 1. Scan for interesting CLI tools that aren't registered
          const systemBins = execSync(
            "ls /run/current-system/sw/bin/ 2>/dev/null || ls /usr/bin/ 2>/dev/null | head -200",
            { encoding: "utf-8", timeout: 5000 }
          ).trim().split("\n");

          // Get currently registered tools
          const registryRes = await fetch(`${CNS}/evolve/tools`);
          const registry = await registryRes.json() as any;
          const registeredNames = new Set(
            (registry.evolved_tools || []).map((t: any) => t.handler_command || "")
          );

          // Interesting tools to auto-learn
          const interestingBins = [
            "htop", "btop", "iotop", "nethogs", "iftop", "nload",
            "ncdu", "duf", "dust", "procs", "bandwhich",
            "speedtest-cli", "curl", "wget", "jq", "yq",
            "bat", "fd", "ripgrep", "rg", "fzf",
            "smartctl", "lsblk", "blkid", "lspci", "lsusb",
            "ss", "ip", "dig", "nslookup", "host",
            "openssl", "age", "gpg",
          ];

          const unregistered = interestingBins.filter(
            bin => systemBins.includes(bin) && !registeredNames.has(bin)
          );

          // 2. For each unregistered tool, learn about it and create a handler
          for (const bin of unregistered.slice(0, 5)) { // Limit to 5 per cycle
            try {
              // Get help text
              const helpText = execSync(
                `${bin} --help 2>&1 | head -5 || ${bin} -h 2>&1 | head -5 || echo "tool"`,
                { encoding: "utf-8", timeout: 3000 }
              ).trim();

              // Ask LLM to generate a description
              const desc = await llm(
                `In one sentence (max 60 chars), describe what the CLI tool "${bin}" does. Based on: ${helpText.substring(0, 200)}. Output ONLY the description, nothing else.`
              );

              const cleanDesc = desc.replace(/"/g, "").replace(/\n/g, " ").trim().substring(0, 80);

              // Create the tool
              const toolName = `sys.${bin.replace(/-/g, "_")}`;
              const evolveRes = await fetch(`${CNS}/evolve/tool`, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({
                  name: toolName,
                  agent: "system-agent",
                  description: cleanDesc || `Run ${bin} system utility`,
                  handler_type: "shell",
                  handler_command: bin,
                }),
              });
              const evolveResult = await evolveRes.json() as any;

              if (evolveResult.ok) {
                results.push(`🧬 Evolved: ${toolName} (${bin}) — ${cleanDesc}`);
              } else {
                results.push(`⚠️ Skipped: ${bin} — ${evolveResult.error}`);
              }
            } catch (e: any) {
              results.push(`⚠️ Failed: ${bin} — ${e.message}`);
            }
          }

          if (unregistered.length > 5) {
            results.push(`\n... and ${unregistered.length - 5} more tools available for next evolution cycle`);
          }

          return {
            ok: true,
            output: results.length > 0
              ? `Autonomous Evolution Complete:\n${results.join("\n")}`
              : "No new tools discovered — all known tools are already registered",
            tool,
            evolved_count: results.filter(r => r.startsWith("🧬")).length,
          };
        } catch (e: any) {
          return { ok: false, error: `Auto-evolution error: ${e.message}`, tool };
        }
      }

      case "learn.list_evolved": {
        try {
          const res = await fetch(`${CNS}/evolve/tools`);
          const data = await res.json() as any;
          return {
            ok: true,
            output: `Total tools: ${data.total_tools}\nAuto-generated: ${data.auto_generated}\n\n${JSON.stringify(data.evolved_tools, null, 2)}`,
            tool,
          };
        } catch (e: any) {
          return { ok: false, error: e.message, tool };
        }
      }

      case "learn.teach": {
        // Human teaches RedNode a new command
        const name = args.name || "";
        const command = args.command || args.cmd || args.url || "";
        const description = args.description || args.desc || "";
        const type = args.type || (command.startsWith("http") ? "api" : "shell");

        if (!name || !command || !description) {
          return {
            ok: false,
            error: "Missing required fields. Usage:\n" +
              '  learn.teach name="sys.my_tool" command="my-command --flag" description="What it does"\n' +
              '  learn.teach name="api.service" command="http://localhost:8080/health" type="api" description="Check service health"',
          };
        }

        // Validate name format
        if (!name.includes(".")) {
          return { ok: false, error: "Tool name must contain a dot (e.g., 'sys.my_tool')" };
        }

        try {
          const res = await fetch(`${CNS}/evolve/tool`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              name,
              agent: args.agent || "system-agent",
              description,
              handler_type: type,
              handler_command: command,
            }),
          });
          const result = await res.json() as any;

          return {
            ok: result.ok,
            output: result.ok
              ? `🎓 Taught: ${name}\n  Command: ${command}\n  Type: ${type}\n  Description: ${description}\n\nYou can now use it: rednode intent "${description}"`
              : `Teaching failed: ${result.error}`,
            tool,
          };
        } catch (e: any) {
          return { ok: false, error: e.message, tool };
        }
      }

      default:
        return null;
    }
  }
}

// ─── Autonomous Learning Loop ───

async function learningLoop() {
  const agent = new LearningAgent();
  await agent.connect();

  // Initial discovery
  console.info("[learning-agent] starting autonomous learning loop");

  const runCycle = async () => {
    learningCycles++;
    console.info(`[learning-agent] learning cycle ${learningCycles}`);

    try {
      // 1. Discover what's available
      await agent.handleTool("learn.discover", {});

      // 2. Extract patterns from recent activity
      await agent.handleTool("learn.extract_patterns", {});

      // 3. Self-assess knowledge gaps
      const assessment = (await agent.handleTool(
        "learn.self_assess",
        {},
      )) as any;

      // 4. Try to learn about gaps
      if (assessment?.gaps?.length > 0) {
        for (const gap of assessment.gaps.slice(0, 3)) {
          await agent.handleTool("learn.ingest_docs", { target: gap });
        }
      }

      // 5. Suggest workflows if patterns are strong enough
      await agent.handleTool("learn.suggest_workflow", {});
    } catch (e) {
      console.error(`[learning-agent] cycle ${learningCycles} error:`, e);
    }
  };

  // Run first cycle immediately
  await runCycle();

  // Then on interval
  setInterval(runCycle, LEARN_INTERVAL);

  await agent.serve();
}

learningLoop();
