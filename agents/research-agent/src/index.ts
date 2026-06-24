import { RedNodeAgent } from "../../shared/src/agent.js";

const CNS = process.env.REDNODE_CNS || "http://localhost:8787";
const OLLAMA_URL = process.env.OLLAMA_URL || "http://127.0.0.1:11434";
const MODEL = process.env.REDNODE_MODEL || "qwen2.5:14b-instruct-q4_K_M";
const SEARXNG_URL = process.env.SEARXNG_URL || "http://localhost:8888";

const TOOLS = [
  "docs.ingest_pdf",
  "docs.ocr",
  "kb.export",
  "kb.graph_visualize",
  "kb.ingest",
  "kb.query",
  "kb.stats",
  "kg.add",
  "kg.entities",
  "kg.relationships",
  "podcast.download",
  "podcast.summarize",
  "podcast.transcribe",
  "research.arxiv",
  "research.compare",
  "research.deep",
  "research.fact_check",
  "research.news",
  "research.query",
  "research.search",
  "research.summarize_url",
  "research.timeline",
  "research.translate",
  "research.weather",
  "research.wikipedia",
  "rss.digest",
  "rss.fetch",
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
            content:
              "You are a research planner. Break the topic into 3-5 specific, searchable sub-questions. Output ONLY a JSON array of strings. No explanation.",
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
        { signal: AbortSignal.timeout(15000) },
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
      const output =
        data.results?.[0]?.result?.output ||
        data.results?.[0]?.result?.result?.output;
      if (output && typeof output === "string" && output.length > 100) {
        enrichedContent.push(
          `Source: ${source.title}\nURL: ${source.url}\n${output.substring(0, 1500)}`,
        );
      }
    } catch {
      // Fallback to snippet
      enrichedContent.push(
        `Source: ${source.title}\nURL: ${source.url}\n${source.snippet}`,
      );
    }
  }

  // Add remaining sources as snippets
  for (const source of uniqueSources.slice(3, 10)) {
    enrichedContent.push(
      `Source: ${source.title}\nURL: ${source.url}\n${source.snippet}`,
    );
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
    report =
      data.message?.content ||
      "Report generation failed — see raw sources below.";
  } catch (e: any) {
    report = `Report generation failed (${e.message}). Raw sources:\n\n${enrichedContent.map((c) => c.substring(0, 300)).join("\n\n")}`;
  }

  // Step 5: Ingest report into RAG memory
  try {
    await fetch(`${CNS}/memory/ingest`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        source: `research/deep/${topic.substring(0, 50).replace(/[^a-zA-Z0-9]/g, "-")}`,
        content: `Deep Research Report: ${topic}\n\n${report}\n\nSources:\n${uniqueSources.map((s) => `- ${s.title}: ${s.url}`).join("\n")}`,
      }),
    });
  } catch {}

  return {
    report,
    sources: uniqueSources.slice(0, 10),
    sub_questions: subQuestions,
  };
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
        if (!topic)
          return { ok: false, error: "Missing 'topic' for deep research" };

        const result = await deepResearch(topic);
        return {
          ok: true,
          output:
            `📊 Deep Research Report: ${topic}\n\n` +
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
          const resp = await fetch(
            `${CNS}/memory/query?q=${encodeURIComponent(query)}&limit=5`,
          );
          const data = (await resp.json()) as any;
          const results = data.results || [];

          if (results.length === 0) {
            return {
              ok: true,
              output: `No results found for: "${query}"`,
              results: [],
            };
          }

          const lines = results.map(
            (r: any, i: number) =>
              `[${i + 1}] Source: ${r.source} (score: ${(r.score * 100).toFixed(0)}%)\n    ${r.content.substring(0, 200)}${r.content.length > 200 ? "…" : ""}`,
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
          return {
            ok: true,
            output: `Document ingested: ${data.id || "ok"} (source: ${source}, ${content.length} chars)`,
            id: data.id,
          };
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
            { signal: AbortSignal.timeout(10000) },
          );
          const data = (await resp.json()) as any;
          const results = (data.results || []).slice(0, 5);
          const lines = results.map(
            (r: any, i: number) =>
              `[${i + 1}] ${r.title}\n    ${r.url}\n    ${(r.content || "").substring(0, 150)}`,
          );
          return {
            ok: true,
            output: lines.join("\n\n") || "No web results found",
            results,
          };
        } catch {
          return this.handleTool("research.query", args);
        }
      }

      case "research.weather": {
        // Weather via wttr.in (free, no API key, privacy-friendly)
        const location =
          args.location || args.city || process.env.WEATHER_LOCATION || "";
        const loc = encodeURIComponent(location || "");
        try {
          const resp = await fetch(`https://wttr.in/${loc}?format=j1`, {
            headers: { "User-Agent": "RedNode-OS/0.5.0" },
            signal: AbortSignal.timeout(10000),
          });
          const data = (await resp.json()) as any;
          const current = data.current_condition?.[0] || {};
          const area = data.nearest_area?.[0] || {};
          const forecast = (data.weather || []).slice(0, 3);

          const areaName = area.areaName?.[0]?.value || location || "Unknown";
          const country = area.country?.[0]?.value || "";

          let output = `🌤️ Weather for ${areaName}${country ? `, ${country}` : ""}\n\n`;
          output += `Current: ${current.weatherDesc?.[0]?.value || "?"}\n`;
          output += `Temperature: ${current.temp_C || "?"}°C (feels like ${current.FeelsLikeC || "?"}°C)\n`;
          output += `Humidity: ${current.humidity || "?"}%\n`;
          output += `Wind: ${current.windspeedKmph || "?"}km/h ${current.winddir16Point || ""}\n`;
          output += `Visibility: ${current.visibility || "?"}km\n`;
          output += `UV Index: ${current.uvIndex || "?"}\n\n`;

          if (forecast.length > 0) {
            output += "Forecast:\n";
            for (const day of forecast) {
              output += `  ${day.date}: ${day.mintempC}°-${day.maxtempC}°C, ${day.hourly?.[4]?.weatherDesc?.[0]?.value || "?"}\n`;
            }
          }

          return { ok: true, output, current, forecast, location: areaName };
        } catch (e: any) {
          return { ok: false, error: `Weather fetch failed: ${e.message}` };
        }
      }

      case "research.news": {
        // News via SearXNG (privacy-first) or RSS
        const topic = args.topic || args.query || "latest news";
        const region = args.region || process.env.NEWS_REGION || "";

        // Try SearXNG news category
        try {
          const query = region ? `${topic} ${region}` : topic;
          const resp = await fetch(
            `${SEARXNG_URL}/search?q=${encodeURIComponent(query)}&format=json&categories=news&language=en&time_range=day`,
            { signal: AbortSignal.timeout(15000) },
          );
          const data = (await resp.json()) as any;
          const articles = (data.results || []).slice(0, 8);

          if (articles.length === 0) {
            return { ok: true, output: `No recent news for: "${topic}"` };
          }

          const lines = articles.map((a: any, i: number) => {
            const age = a.publishedDate
              ? ` (${new Date(a.publishedDate).toLocaleDateString()})`
              : "";
            return `[${i + 1}] ${a.title}${age}\n    ${a.url}\n    ${(a.content || "").substring(0, 150)}`;
          });

          // Summarize with LLM if available
          let summary = "";
          try {
            const articleText = articles
              .map(
                (a: any) =>
                  `${a.title}: ${(a.content || "").substring(0, 200)}`,
              )
              .join("\n");
            const resp = await fetch(`${OLLAMA_URL}/api/chat`, {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                model: MODEL,
                messages: [
                  {
                    role: "system",
                    content:
                      "Summarize these news headlines in 3-4 bullet points. Be brief.",
                  },
                  { role: "user", content: articleText },
                ],
                stream: false,
                options: { temperature: 0.3, num_predict: 256 },
              }),
            });
            const sData = (await resp.json()) as any;
            summary = sData.message?.content || "";
          } catch {}

          let output = `📰 News: ${topic}\n\n`;
          if (summary) output += `Summary:\n${summary}\n\n`;
          output += `Articles:\n${lines.join("\n\n")}`;

          // Ingest into memory
          await fetch(`${CNS}/memory/ingest`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              source: `news/${new Date().toISOString().slice(0, 10)}`,
              content: `News (${topic}): ${summary || articles.map((a: any) => a.title).join("; ")}`,
            }),
          }).catch(() => {});

          return { ok: true, output, articles: articles.length, summary };
        } catch (e: any) {
          return {
            ok: false,
            error: `News fetch failed: ${e.message}. Is SearXNG running?`,
          };
        }
      }

      case "docs.ocr": {
        const filePath = args.path || args.file || "";
        if (!filePath) return { ok: false, error: "Missing 'path'" };

        try {
          const { exec } = await import("child_process");
          const { promisify } = await import("util");
          const execAsync = promisify(exec);
          const { stdout } = await execAsync(
            `tesseract "${filePath}" stdout --oem 3 --psm 3 2>/dev/null`,
            { timeout: 60000 },
          );
          const text = stdout.trim();

          if (!text) return { ok: true, output: "OCR: no text detected" };

          const source = `ocr/${filePath.split("/").pop() || "scan"}`;
          await fetch(`${CNS}/memory/ingest`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ source, content: text }),
          });

          return {
            ok: true,
            output: `OCR: ${text.length} chars extracted → ingested\n\n${text.substring(0, 1000)}`,
            text_length: text.length,
          };
        } catch (e: any) {
          return {
            ok: false,
            error: `OCR failed: ${e.message}. Install: apt install tesseract-ocr`,
          };
        }
      }

      case "docs.ingest_pdf": {
        const filePath = args.path || args.file || "";
        if (!filePath) return { ok: false, error: "Missing 'path'" };

        try {
          const { exec } = await import("child_process");
          const { promisify } = await import("util");
          const execAsync = promisify(exec);
          const { stdout } = await execAsync(
            `pdftotext "${filePath}" - 2>/dev/null`,
            { timeout: 30000 },
          );
          const text = stdout.trim();

          if (!text)
            return {
              ok: true,
              output: "PDF has no extractable text — try docs.ocr",
            };

          const CHUNK = 2000;
          const chunks = [];
          for (let i = 0; i < text.length; i += CHUNK)
            chunks.push(text.substring(i, i + CHUNK));

          const source = `pdf/${filePath.split("/").pop() || "doc"}`;
          for (let i = 0; i < chunks.length; i++) {
            await fetch(`${CNS}/memory/ingest`, {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                source: `${source}/chunk-${i}`,
                content: chunks[i],
              }),
            });
          }

          return {
            ok: true,
            output: `PDF: ${text.length} chars in ${chunks.length} chunks → ingested\n\n${text.substring(0, 500)}`,
            chunks: chunks.length,
          };
        } catch (e: any) {
          return {
            ok: false,
            error: `PDF failed: ${e.message}. Install: apt install poppler-utils`,
          };
        }
      }
      case "kg.entities": {
        const r = await cns("/memory/query?type=entity&limit=30"); return { ok: r.ok, output: r.output, tool }; Kuzu graph query
      }

      case "kg.relationships": {
        const r = await cns("/memory/query?type=relationship&limit=30"); return { ok: r.ok, output: r.output, tool }; Kuzu graph query
      }

      case "kg.add": {
        const entity = args.entity || args.name || ""; const type = args.type || "unknown"; if (!entity) return { ok: false, error: "Missing entity name" }; const r = await cns("/memory/store", { method: "POST", body: { type: "entity", key: entity, value: { name: entity, type, properties: args.properties || {} } } }); return { ok: r.ok, output: `Entity added: ${entity} (${type})`, tool }; Kuzu graph insert
      }

      case "rss.fetch": {
        const feedUrl = args.url || args.feed || ""; if (!feedUrl) return { ok: false, error: "Missing RSS feed URL" }; const r = await sh(`curl -sL "${feedUrl}" 2>&1 | head -500`); return { ok: r.ok, output: r.output, tool }; HTTP fetch + XML parse
      }

      case "rss.digest": {
        const feeds = (process.env.RSS_FEEDS || "").split("|").filter(Boolean); if (!feeds.length) return { ok: true, output: "No RSS feeds configured — set RSS_FEEDS in .env", tool }; const items: string[] = []; for (const feed of feeds.slice(0, 3)) { const r = await sh(`curl -sL "${feed}" 2>&1 | grep -oP "<title>[^<]+" | head -5 | sed "s/<title>//"`, 10000); if (r.ok && r.output) items.push(r.output); } const summary = await llm(`Summarize these news headlines concisely:\n${items.join("\n")}`); return { ok: true, output: summary, tool }; fetch + LLM summarize
      }

      case "podcast.download": {
        const url = args.url || ""; if (!url) return { ok: false, error: "Missing podcast episode URL" }; const dest = process.env.PODCAST_DOWNLOAD_DIR || "/var/lib/rednode/podcasts"; const r = await sh(`mkdir -p ${dest} && curl -sLo "${dest}/episode-${Date.now()}.mp3" "${url}" 2>&1 && echo "Downloaded to ${dest}"`, 120000); return { ok: r.ok, output: r.output, tool }; wget podcast episode
      }

      case "podcast.transcribe": {
        const file = args.file || ""; if (!file) return { ok: false, error: "Missing audio file path" }; const r = await sh(`whisper "${file}" --model small --output_format txt 2>&1 || echo "Whisper not installed — pip install openai-whisper"`, 300000); return { ok: r.ok, output: r.output, tool }; Whisper transcription
      }

      case "podcast.summarize": {
        const text = args.text || args.transcript || ""; if (!text) return { ok: false, error: "Missing transcript text" }; const summary = await llm(`Summarize this podcast transcript in 3-5 bullet points:\n${text.substring(0, 3000)}`); return { ok: true, output: summary, tool }; LLM summarization
      }

      case "research.arxiv": {
        const query = args.query || args.topic || "";
                if (!query) return { ok: false, error: "Missing 'query' topic" };
                try {
                  const res = await fetch(\`http://export.arxiv.org/api/query?search_query=all:\${encodeURIComponent(query)}&max_results=5\`);
                  const xml = await res.text();
                  return { ok: true, output: xml.substring(0, 3000), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "research.wikipedia": {
        const query = args.query || args.topic || "";
                if (!query) return { ok: false, error: "Missing 'query' topic" };
                try {
                  const res = await fetch(\`https://en.wikipedia.org/api/rest_v1/page/summary/\${encodeURIComponent(query)}\`);
                  const data = await res.json() as any;
                  return { ok: true, output: data.extract || "No article found", tool, title: data.title };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "research.translate": {
        const text = args.text || ""; const target = args.target || args.language || "English"; if (!text) return { ok: false, error: "Missing text to translate" }; const translated = await llm(`Translate the following to ${target}. Output only the translation:\n${text}`); return { ok: true, output: translated, tool }; LLM translation
      }

      case "research.summarize_url": {
        const url = args.url || ""; if (!url) return { ok: false, error: "Missing URL" }; const r = await sh(`curl -sL "${url}" 2>&1 | sed "s/<[^>]*>//g" | sed "/^$/d" | head -200`); if (!r.ok) return r; const summary = await llm(`Summarize this webpage content:\n${r.output.substring(0, 3000)}`); return { ok: true, output: summary, tool }; fetch URL + LLM summarize
      }

      case "research.compare": {
        const a = args.a || args.topic1 || ""; const b = args.b || args.topic2 || ""; if (!a || !b) return { ok: false, error: "Missing two topics to compare" }; const comparison = await llm(`Compare "${a}" vs "${b}" in a structured table format. Cover key differences, pros/cons.`); return { ok: true, output: comparison, tool }; LLM comparison
      }

      case "research.timeline": {
        const topic = args.topic || args.query || ""; if (!topic) return { ok: false, error: "Missing topic" }; const timeline = await llm(`Create a chronological timeline of key events for: "${topic}". Use format: YYYY - Event description.`); return { ok: true, output: timeline, tool }; LLM timeline generation
      }

      case "research.fact_check": {
        const claim = args.claim || args.statement || ""; if (!claim) return { ok: false, error: "Missing claim to fact-check" }; const analysis = await llm(`Fact-check this claim: "${claim}". Provide: verdict (true/false/partially true), evidence, confidence level.`); return { ok: true, output: analysis, tool }; multi-source search + LLM verify
      }

      case "kb.export": {
        const r = await cns("/memory/query?limit=100"); return { ok: r.ok, output: r.output, tool }; PostgreSQL dump to markdown
      }

      case "kb.stats": {
        const r = await cns("/memory/stats"); return { ok: r.ok, output: r.output || "Knowledge base stats unavailable", tool }; PostgreSQL + Qdrant stats
      }

      case "kb.graph_visualize": {
        const r = await cns("/memory/query?type=entity&limit=50"); return { ok: r.ok, output: "Knowledge graph nodes:\n" + r.output, tool }; Kuzu → DOT format
      }



      default:
        return null;
    }
  }
}

const agent = new ResearchAgent();
await agent.connect();
await agent.serve();
