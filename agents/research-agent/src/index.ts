import { RedNodeAgent } from "../../shared/src/agent.js";

const CNS = process.env.REDNODE_CNS || "http://localhost:8787";
const OLLAMA_URL = process.env.OLLAMA_URL || "http://127.0.0.1:11434";
const MODEL = process.env.REDNODE_MODEL || "qwen2.5:14b-instruct-q4_K_M";
const SEARXNG_URL = process.env.SEARXNG_URL || "http://localhost:8888";

const TOOLS = [
  "research.search", "research.query", "research.deep",
  "kb.query", "kb.ingest",
  "docs.ocr", "docs.ingest_pdf",
];

// ─── Deep Research ───
// 1. Break topic into 3-5 sub-questions
// 2. Search each via SearXNG (parallel)
// 3. Scrape top results via Browser Agent
// 4. Synthesize into a cited report
// 5. Ingest report + sources into RAG memory

async function deepResearch(topic: string): Promise<{
  report: string;
  sources: { title: string; url: string; snippet: string }[];
  sub_questions: string[];
}> {
  // Step 1: Generate sub-questions via LLM
  let subQuestions: string[] = [];
  try {
    const resp = await fetch(`${OLLAMA_URL}/api/chat`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        model: MODEL,
        messages: [
          {
            role: "system",
            content: "You are a research planner. Break the topic into 3-5 specific, searchable sub-questions. Output ONLY a JSON array of strings. No explanation.",
          },
          { role: "user", content: `Topic: "${topic}"` },
        ],
        stream: false,
        options: { temperature: 0.3, num_predict: 256 },
      }),
    });
    const data = (await resp.json()) as any;
    const text = data.message?.content || "[]";
    // Extract JSON array
    const match = text.match(/\[[\s\S]*\]/);
    if (match) {
      subQuestions = JSON.parse(match[0]);
    }
  } catch {
    // Fallback: simple decomposition
    subQuestions = [
      `What is ${topic}?`,
      `Current state of ${topic} in 2026`,
      `Key challenges and opportunities in ${topic}`,
    ];
  }

  if (subQuestions.length === 0) {
    subQuestions = [`${topic} overview`, `${topic} latest developments`];
  }

  // Step 2: Search each sub-question via SearXNG (parallel)
  const allSources: { title: string; url: string; snippet: string }[] = [];
  const searchPromises = subQuestions.map(async (q) => {
    try {
      const resp = await fetch(
        `${SEARXNG_URL}/search?q=${encodeURIComponent(q)}&format=json&categories=general&language=en`,
        { signal: AbortSignal.timeout(15000) }
      );
      const data = (await resp.json()) as any;
      return (data.results || []).slice(0, 5).map((r: any) => ({
        title: r.title || "",
        url: r.url || "",
        snippet: (r.content || "").substring(0, 300),
        question: q,
      }));
    } catch {
      return [];
    }
  });

  const searchResults = await Promise.all(searchPromises);
  for (const results of searchResults) {
    allSources.push(...results);
  }

  // Deduplicate by URL
  const seenUrls = new Set<string>();
  const uniqueSources = allSources.filter((s) => {
    if (seenUrls.has(s.url)) return false;
    seenUrls.add(s.url);
    return true;
  });

  // Step 3: Try to scrape top 3 results for full content via Browser Agent
  const enrichedContent: string[] = [];
  for (const source of uniqueSources.slice(0, 3)) {
    try {
      // Use the browser agent via CNS intent (it has stealth module)
      const resp = await fetch(`${CNS}/intent`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          intent: `read webpage ${source.url}`,
          session_id: "deep-research",
        }),
      });
      const data = (await resp.json()) as any;
      const output = data.results?.[0]?.result?.output || data.results?.[0]?.result?.result?.output;
      if (output && typeof output === "string" && output.length > 100) {
        enrichedContent.push(`Source: ${source.title}\nURL: ${source.url}\n${output.substring(0, 1500)}`);
      }
    } catch {
      // Fallback to snippet
      enrichedContent.push(`Source: ${source.title}\nURL: ${source.url}\n${source.snippet}`);
    }
  }

  // Add remaining sources as snippets
  for (const source of uniqueSources.slice(3, 10)) {
    enrichedContent.push(`Source: ${source.title}\nURL: ${source.url}\n${source.snippet}`);
  }

  // Step 4: Synthesize into a cited report via LLM
  let report = "";
  try {
    const sourceMaterial = enrichedContent.join("\n\n---\n\n");
    const resp = await fetch(`${OLLAMA_URL}/api/chat`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        model: MODEL,
        messages: [
          {
            role: "system",
            content: `You are a research analyst. Synthesize the source material into a comprehensive research report on the given topic. Requirements:
- Start with a 2-3 sentence executive summary
- Organize findings by theme (not by source)
- Cite sources using [Source: title] format
- Include key facts, statistics, and quotes
- End with 2-3 key takeaways
- Be factual and evidence-based — don't make claims without source backing
- Length: 500-1000 words`,
          },
          {
            role: "user",
            content: `Topic: "${topic}"\n\nSub-questions investigated:\n${subQuestions.map((q, i) => `${i + 1}. ${q}`).join("\n")}\n\nSource Material:\n\n${sourceMaterial.substring(0, 8000)}`,
          },
        ],
        stream: false,
        options: { temperature: 0.3, num_predict: 2048 },
      }),
    });
    const data = (await resp.json()) as any;
    report = data.message?.content || "Report generation failed — see raw sources below.";
  } catch (e: any) {
    report = `Report generation failed (${e.message}). Raw sources:\n\n${enrichedContent.map(c => c.substring(0, 300)).join("\n\n")}`;
  }

  // Step 5: Ingest report into RAG memory
  try {
    await fetch(`${CNS}/memory/ingest`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        source: `research/deep/${topic.substring(0, 50).replace(/[^a-zA-Z0-9]/g, "-")}`,
        content: `Deep Research Report: ${topic}\n\n${report}\n\nSources:\n${uniqueSources.map(s => `- ${s.title}: ${s.url}`).join("\n")}`,
      }),
    });
  } catch {}

  return { report, sources: uniqueSources.slice(0, 10), sub_questions: subQuestions };
}

// ─── Agent ───

class ResearchAgent extends RedNodeAgent {
  constructor() {
    super("research", TOOLS);
  }

  async handleTool(tool: string, args: any): Promise<any> {
    switch (tool) {
      case "research.deep": {
        const topic = args.topic || args.query || args.q || "";
        if (!topic) return { ok: false, error: "Missing 'topic' for deep research" };

        const result = await deepResearch(topic);
        return {
          ok: true,
          output: `📊 Deep Research Report: ${topic}\n\n` +
            `Sub-questions: ${result.sub_questions.length}\n` +
            `Sources found: ${result.sources.length}\n\n` +
            `${result.report}\n\n` +
            `── Sources ──\n${result.sources.map((s, i) => `[${i + 1}] ${s.title}\n    ${s.url}`).join("\n")}`,
          report: result.report,
          sources: result.sources,
          sub_questions: result.sub_questions,
        };
      }

      case "research.query":
      case "kb.query": {
        const query = args.query || args.q || "";
        if (!query) return { ok: false, error: "Missing 'query'" };

        try {
          const resp = await fetch(`${CNS}/memory/query?q=${encodeURIComponent(query)}&limit=5`);
          const data = (await resp.json()) as any;
          const results = data.results || [];

          if (results.length === 0) {
            return { ok: true, output: `No results found for: "${query}"`, results: [] };
          }

          const lines = results.map((r: any, i: number) =>
            `[${i + 1}] Source: ${r.source} (score: ${(r.score * 100).toFixed(0)}%)\n    ${r.content.substring(0, 200)}${r.content.length > 200 ? "…" : ""}`
          );

          return {
            ok: true,
            output: `Found ${results.length} results for "${query}":\n\n${lines.join("\n\n")}`,
            results,
          };
        } catch (e: any) {
          return { ok: false, error: `RAG query failed: ${e.message}` };
        }
      }

      case "kb.ingest": {
        const source = args.source || "manual";
        const content = args.content || "";
        if (!content) return { ok: false, error: "Missing 'content'" };

        try {
          const resp = await fetch(`${CNS}/memory/ingest`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ source, content }),
          });
          const data = (await resp.json()) as any;
          return { ok: true, output: `Document ingested: ${data.id || "ok"} (source: ${source}, ${content.length} chars)`, id: data.id };
        } catch (e: any) {
          return { ok: false, error: `Ingest failed: ${e.message}` };
        }
      }

      case "research.search": {
        const query = args.query || args.q || "";
        if (!query) return { ok: false, error: "Missing 'query'" };

        try {
          const resp = await fetch(
            `${SEARXNG_URL}/search?q=${encodeURIComponent(query)}&format=json&categories=general&language=en`,
            { signal: AbortSignal.timeout(10000) }
          );
          const data = (await resp.json()) as any;
          const results = (data.results || []).slice(0, 5);
          const lines = results.map((r: any, i: number) =>
            `[${i + 1}] ${r.title}\n    ${r.url}\n    ${(r.content || "").substring(0, 150)}`
          );
          return { ok: true, output: lines.join("\n\n") || "No web results found", results };
        } catch {
          return this.handleTool("research.query", args);
        }
      }

      case "docs.ocr": {
        const filePath = args.path || args.file || "";
        if (!filePath) return { ok: false, error: "Missing 'path'" };

        try {
          const { exec } = await import("child_process");
          const { promisify } = await import("util");
          const execAsync = promisify(exec);
          const { stdout } = await execAsync(`tesseract "${filePath}" stdout --oem 3 --psm 3 2>/dev/null`, { timeout: 60000 });
          const text = stdout.trim();

          if (!text) return { ok: true, output: "OCR: no text detected" };

          const source = `ocr/${filePath.split("/").pop() || "scan"}`;
          await fetch(`${CNS}/memory/ingest`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ source, content: text }),
          });

          return { ok: true, output: `OCR: ${text.length} chars extracted → ingested\n\n${text.substring(0, 1000)}`, text_length: text.length };
        } catch (e: any) {
          return { ok: false, error: `OCR failed: ${e.message}. Install: apt install tesseract-ocr` };
        }
      }

      case "docs.ingest_pdf": {
        const filePath = args.path || args.file || "";
        if (!filePath) return { ok: false, error: "Missing 'path'" };

        try {
          const { exec } = await import("child_process");
          const { promisify } = await import("util");
          const execAsync = promisify(exec);
          const { stdout } = await execAsync(`pdftotext "${filePath}" - 2>/dev/null`, { timeout: 30000 });
          const text = stdout.trim();

          if (!text) return { ok: true, output: "PDF has no extractable text — try docs.ocr" };

          const CHUNK = 2000;
          const chunks = [];
          for (let i = 0; i < text.length; i += CHUNK) chunks.push(text.substring(i, i + CHUNK));

          const source = `pdf/${filePath.split("/").pop() || "doc"}`;
          for (let i = 0; i < chunks.length; i++) {
            await fetch(`${CNS}/memory/ingest`, {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({ source: `${source}/chunk-${i}`, content: chunks[i] }),
            });
          }

          return { ok: true, output: `PDF: ${text.length} chars in ${chunks.length} chunks → ingested\n\n${text.substring(0, 500)}`, chunks: chunks.length };
        } catch (e: any) {
          return { ok: false, error: `PDF failed: ${e.message}. Install: apt install poppler-utils` };
        }
      }

      default:
        return null;
    }
  }
}

const agent = new ResearchAgent();
await agent.connect();
await agent.serve();
