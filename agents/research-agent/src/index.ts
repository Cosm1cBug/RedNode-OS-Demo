import { RedNodeAgent } from "../../shared/src/agent.js";

const CNS = process.env.REDNODE_CNS || "http://localhost:8787";
const TOOLS = ["research.search", "research.query", "kb.query", "kb.ingest", "docs.ocr", "docs.ingest_pdf"];

class ResearchAgent extends RedNodeAgent {
  constructor() {
    super("research", TOOLS);
  }

  async handleTool(tool: string, args: any): Promise<any> {
    switch (tool) {
      case "research.query":
      case "kb.query": {
        const query = args.query || args.q || "";
        if (!query) return { ok: false, error: "Missing 'query' argument" };

        try {
          const resp = await fetch(`${CNS}/memory/query?q=${encodeURIComponent(query)}&limit=5`);
          const data = await resp.json() as any;
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
        if (!content) return { ok: false, error: "Missing 'content' argument" };

        try {
          const resp = await fetch(`${CNS}/memory/ingest`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ source, content }),
          });
          const data = await resp.json() as any;
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
        // Web search via SearXNG (self-hosted) — if available
        const searxng = process.env.SEARXNG_URL || "http://localhost:8888";
        const query = args.query || args.q || "";
        if (!query) return { ok: false, error: "Missing 'query' argument" };

        try {
          const resp = await fetch(
            `${searxng}/search?q=${encodeURIComponent(query)}&format=json&categories=general&language=en`,
            { signal: AbortSignal.timeout(10000) }
          );
          const data = await resp.json() as any;
          const results = (data.results || []).slice(0, 5);
          const lines = results.map((r: any, i: number) =>
            `[${i + 1}] ${r.title}\n    ${r.url}\n    ${(r.content || "").substring(0, 150)}`
          );
          return {
            ok: true,
            output: lines.join("\n\n") || "No web results found",
            results,
          };
        } catch {
          // Fallback to RAG if SearXNG is not available
          return this.handleTool("research.query", args);
        }
      }

      case "docs.ocr": {
        // OCR a document image using Tesseract
        const filePath = args.path || args.file || "";
        if (!filePath) return { ok: false, error: "Missing 'path' to image/PDF file" };

        try {
          const { exec } = await import("child_process");
          const { promisify } = await import("util");
          const execAsync = promisify(exec);

          // Run tesseract OCR
          const { stdout } = await execAsync(
            `tesseract "${filePath}" stdout --oem 3 --psm 3 2>/dev/null`,
            { timeout: 60000 }
          );
          const text = stdout.trim();

          if (!text) return { ok: true, output: "OCR completed but no text detected in image" };

          // Auto-ingest into RAG memory
          const source = `ocr/${filePath.split("/").pop() || "scan"}`;
          await fetch(`${CNS}/memory/ingest`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({ source, content: text }),
          });

          return {
            ok: true,
            output: `OCR completed — ${text.length} characters extracted and ingested into memory.\n\nText:\n${text.substring(0, 1000)}${text.length > 1000 ? "\n...(truncated)" : ""}`,
            text_length: text.length,
            source,
          };
        } catch (e: any) {
          return { ok: false, error: `OCR failed: ${e.message}. Install tesseract: apt install tesseract-ocr` };
        }
      }

      case "docs.ingest_pdf": {
        // Extract text from PDF and ingest
        const filePath = args.path || args.file || "";
        if (!filePath) return { ok: false, error: "Missing 'path' to PDF file" };

        try {
          const { exec } = await import("child_process");
          const { promisify } = await import("util");
          const execAsync = promisify(exec);

          // Try pdftotext first (poppler-utils)
          const { stdout } = await execAsync(
            `pdftotext "${filePath}" - 2>/dev/null`,
            { timeout: 30000 }
          );
          const text = stdout.trim();

          if (!text) return { ok: true, output: "PDF has no extractable text — try docs.ocr for scanned PDFs" };

          // Chunk and ingest (PDF can be large)
          const CHUNK_SIZE = 2000;
          const chunks = [];
          for (let i = 0; i < text.length; i += CHUNK_SIZE) {
            chunks.push(text.substring(i, i + CHUNK_SIZE));
          }

          const source = `pdf/${filePath.split("/").pop() || "document"}`;
          for (let i = 0; i < chunks.length; i++) {
            await fetch(`${CNS}/memory/ingest`, {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({ source: `${source}/chunk-${i}`, content: chunks[i] }),
            });
          }

          return {
            ok: true,
            output: `PDF ingested — ${text.length} characters in ${chunks.length} chunks.\n\nPreview:\n${text.substring(0, 500)}${text.length > 500 ? "\n...(truncated)" : ""}`,
            chunks: chunks.length,
            text_length: text.length,
          };
        } catch (e: any) {
          return { ok: false, error: `PDF extraction failed: ${e.message}. Install: apt install poppler-utils` };
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
