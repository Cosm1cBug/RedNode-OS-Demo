/**
 * RedNode-OS – Browser Agent
 *
 * Web page reading, file downloading, screenshot capture, structured data extraction.
 *
 * Two engines:
 *   1. Playwright (Chromium headless) — for JS-rendered pages, screenshots, complex scraping
 *   2. Native fetch + cheerio — for fast, lightweight HTML extraction (no browser overhead)
 *
 * Security:
 *   - All downloads go to /var/lib/rednode/downloads/ (sandboxed directory)
 *   - URLs are validated (no file://, no localhost/internal by default)
 *   - Screenshots are stored locally, never uploaded
 *   - form.fill is HIGH risk (requires approval)
 *
 * Setup:
 *   cd agents/browser-agent
 *   pnpm install
 *   npx playwright install chromium  # downloads ~200MB Chromium binary
 */

import { RedNodeAgent } from "../../shared/src/agent.js";
import { sh, api, llm, cns, pihole, truenas, frigate, ha } from "../../shared/src/helpers.js";
import {
  generateProfile,
  applyStealthToPage,
  humanDelay,
  createStealthContext,
} from "./stealth.js";
import type { StealthProfile } from "./stealth.js";
import * as fs from "fs";
import * as path from "path";

const CNS = process.env.REDNODE_CNS || "http://localhost:8787";
const DOWNLOAD_DIR = process.env.DOWNLOAD_DIR || "/var/lib/rednode/downloads";
const MAX_CONTENT_LENGTH = 100000; // 100KB text limit per page
const REQUEST_TIMEOUT = 30000; // 30 seconds

const TOOLS = [
  "browser.archive",
  "browser.cookie_clean",
  "browser.download",
  "browser.fill",
  "browser.links",
  "browser.monitor",
  "browser.pdf",
  "browser.price_track",
  "browser.read",
  "browser.readability",
  "browser.scrape",
  "browser.screenshot",
  "browser.search",
];

// ─── URL Validation ───

const BLOCKED_SCHEMES = ["file:", "ftp:", "data:", "javascript:", "blob:"];
const BLOCKED_HOSTS = ["localhost", "127.0.0.1", "0.0.0.0", "::1", "169.254"];

function validateUrl(url: string): { ok: boolean; error?: string } {
  try {
    const parsed = new URL(url);
    for (const scheme of BLOCKED_SCHEMES) {
      if (parsed.protocol === scheme)
        return { ok: false, error: `Blocked scheme: ${scheme}` };
    }
    for (const host of BLOCKED_HOSTS) {
      if (parsed.hostname.includes(host))
        return { ok: false, error: `Blocked host: ${host}` };
    }
    // Block internal network by default (10.x, 192.168.x, 172.16-31.x)
    if (
      /^(10\.|192\.168\.|172\.(1[6-9]|2[0-9]|3[01])\.)/.test(parsed.hostname)
    ) {
      // Allow if explicitly permitted via env var
      if (process.env.BROWSER_ALLOW_INTERNAL !== "true") {
        return {
          ok: false,
          error: `Internal network access blocked: ${parsed.hostname}. Set BROWSER_ALLOW_INTERNAL=true to allow.`,
        };
      }
    }
    return { ok: true };
  } catch {
    return { ok: false, error: `Invalid URL: ${url}` };
  }
}

// ─── Lightweight Fetch + Cheerio (fast, no browser) ───

async function fetchAndParse(
  url: string,
): Promise<{ html: string; text: string; title: string }> {
  const profile = generateProfile(url);
  await humanDelay("request");

  const resp = await fetch(url, {
    headers: profile.headers,
    signal: AbortSignal.timeout(REQUEST_TIMEOUT),
  });

  if (!resp.ok) throw new Error(`HTTP ${resp.status}: ${resp.statusText}`);

  const contentType = resp.headers.get("content-type") || "";
  if (
    !contentType.includes("text/html") &&
    !contentType.includes("application/xhtml")
  ) {
    throw new Error(`Not an HTML page: ${contentType}`);
  }

  const html = await resp.text();
  const { load } = await import("cheerio");
  const $ = load(html);

  // Remove scripts, styles, nav, footer, ads
  $(
    "script, style, nav, footer, header, iframe, noscript, .ad, .ads, .advertisement, #cookie-banner",
  ).remove();

  const title = $("title").text().trim() || $("h1").first().text().trim() || "";
  const text = $("article, main, .content, .post, .entry, #content, body")
    .first()
    .text()
    .replace(/\s+/g, " ")
    .trim()
    .substring(0, MAX_CONTENT_LENGTH);

  return { html, text, title };
}

// ─── Playwright (full browser, JS rendering) ───

let playwrightBrowser: any = null;

async function getPlaywrightPage(targetUrl?: string): Promise<any> {
  try {
    const { chromium } = await import("playwright");
    if (!playwrightBrowser || !playwrightBrowser.isConnected()) {
      playwrightBrowser = await chromium.launch({
        headless: true,
        args: [
          "--no-sandbox",
          "--disable-setuid-sandbox",
          "--disable-dev-shm-usage",
          "--disable-gpu",
          "--disable-extensions",
          "--disable-default-apps",
          "--disable-background-networking",
          "--disable-blink-features=AutomationControlled",
        ],
      });
    }

    // Create stealth context with randomized fingerprint
    const profile = generateProfile(targetUrl);
    const context = await createStealthContext(playwrightBrowser, profile);
    const page = await context.newPage();

    // Apply stealth patches (hide webdriver, spoof plugins, etc.)
    await applyStealthToPage(page, profile);

    // Block unnecessary resources for speed (but keep CSS for screenshots)
    await page.route("**/*.{woff,woff2,ttf,eot}", (route: any) =>
      route.abort(),
    );

    // Add human-like delay
    await humanDelay("page_load");

    return { page, context };
  } catch (e: any) {
    throw new Error(
      `Playwright not available: ${e.message}. Run: npx playwright install chromium`,
    );
  }
}

// ─── Agent ───

class BrowserAgent extends RedNodeAgent {
  constructor() {
    super("browser", TOOLS);
  }

  async handleTool(tool: string, args: any): Promise<any> {
    const url = args.url || "";

    // Validate URL for all tools that need one
    if (url && tool !== "browser.search") {
      const valid = validateUrl(url);
      if (!valid.ok) return { ok: false, error: valid.error };
    }

    try {
      switch (tool) {
        case "browser.read": {
          if (!url) return { ok: false, error: "Missing 'url' argument" };

          // Try lightweight fetch first (faster)
          try {
            const { text, title } = await fetchAndParse(url);
            if (text.length > 100) {
              // Ingest into memory for future RAG search
              await this.ingestToMemory(
                `web/${new URL(url).hostname}`,
                `${title}: ${text.substring(0, 5000)}`,
              );
              return {
                ok: true,
                output: `📄 ${title}\n\n${text.substring(0, 3000)}${text.length > 3000 ? "\n\n...(truncated)" : ""}`,
                title,
                url,
                text_length: text.length,
                method: "fetch+cheerio",
              };
            }
          } catch {
            // Fallback to Playwright for JS-rendered pages
          }

          // Playwright fallback
          const { page, context } = await getPlaywrightPage();
          try {
            await page.goto(url, {
              waitUntil: "domcontentloaded",
              timeout: REQUEST_TIMEOUT,
            });
            const title = await page.title();
            const text = await page.evaluate(() => {
              // Remove noise
              document
                .querySelectorAll("script,style,nav,footer,header,iframe")
                .forEach((el) => el.remove());
              const main = document.querySelector(
                "article,main,.content,#content,body",
              );
              return (
                main?.textContent
                  ?.replace(/\s+/g, " ")
                  .trim()
                  .substring(0, 100000) || ""
              );
            });

            await this.ingestToMemory(
              `web/${new URL(url).hostname}`,
              `${title}: ${text.substring(0, 5000)}`,
            );

            return {
              ok: true,
              output: `📄 ${title}\n\n${text.substring(0, 3000)}${text.length > 3000 ? "\n...(truncated)" : ""}`,
              title,
              url,
              text_length: text.length,
              method: "playwright",
            };
          } finally {
            await context.close();
          }
        }

        case "browser.scrape": {
          if (!url) return { ok: false, error: "Missing 'url' argument" };
          const selector =
            args.selector || "table, ul, ol, dl, .data, .results";

          const { page, context } = await getPlaywrightPage();
          try {
            await page.goto(url, {
              waitUntil: "domcontentloaded",
              timeout: REQUEST_TIMEOUT,
            });

            const data = await page.evaluate((sel: string) => {
              const elements = document.querySelectorAll(sel);
              return Array.from(elements).map((el, i) => ({
                index: i,
                tag: el.tagName.toLowerCase(),
                text:
                  el.textContent
                    ?.replace(/\s+/g, " ")
                    .trim()
                    .substring(0, 2000) || "",
                html: el.outerHTML.substring(0, 1000),
              }));
            }, selector);

            return {
              ok: true,
              output:
                data.length > 0
                  ? data
                      .map(
                        (d: { tag: string; index: number; text: string }) =>
                          `[${d.tag}#${d.index}] ${d.text.substring(0, 500)}`,
                      )
                      .join("\n\n")
                  : `No elements matching "${selector}" found on ${url}`,
              elements: data.length,
              url,
            };
          } finally {
            await context.close();
          }
        }

        case "browser.screenshot": {
          if (!url) return { ok: false, error: "Missing 'url' argument" };

          const { page, context } = await getPlaywrightPage();
          try {
            await page.goto(url, {
              waitUntil: "networkidle",
              timeout: REQUEST_TIMEOUT,
            });

            fs.mkdirSync(DOWNLOAD_DIR, { recursive: true });
            const filename = `screenshot-${Date.now()}.png`;
            const filepath = path.join(DOWNLOAD_DIR, filename);

            await page.screenshot({
              path: filepath,
              fullPage: args.full_page ?? false,
            });

            return {
              ok: true,
              output: `📸 Screenshot saved: ${filepath}`,
              path: filepath,
              url,
            };
          } finally {
            await context.close();
          }
        }

        case "browser.download": {
          if (!url) return { ok: false, error: "Missing 'url' argument" };

          fs.mkdirSync(DOWNLOAD_DIR, { recursive: true });

          // Determine filename
          const urlObj = new URL(url);
          const defaultName =
            urlObj.pathname.split("/").pop() || `download-${Date.now()}`;
          const filename = args.filename || defaultName;
          const filepath = path.join(DOWNLOAD_DIR, filename);

          // Security: prevent path traversal in filename
          if (filename.includes("..") || filename.includes("/")) {
            return {
              ok: false,
              error: "Invalid filename — path traversal denied",
            };
          }

          // Download using fetch (streaming)
          const resp = await fetch(url, {
            headers: { "User-Agent": "RedNode-OS/0.3.1" },
            signal: AbortSignal.timeout(120000), // 2 min for large files
          });

          if (!resp.ok)
            return { ok: false, error: `Download failed: HTTP ${resp.status}` };

          const contentLength = resp.headers.get("content-length");
          const maxSize =
            parseInt(process.env.MAX_DOWNLOAD_MB || "500") * 1024 * 1024;

          if (contentLength && parseInt(contentLength) > maxSize) {
            return {
              ok: false,
              error: `File too large: ${contentLength} bytes (max: ${maxSize / 1048576} MB)`,
            };
          }

          const buffer = Buffer.from(await resp.arrayBuffer());
          fs.writeFileSync(filepath, buffer);

          const sizeMB = (buffer.length / 1048576).toFixed(2);
          const contentType = resp.headers.get("content-type") || "unknown";

          // If it's a PDF, auto-ingest into memory
          if (contentType.includes("pdf") || filename.endsWith(".pdf")) {
            try {
              const { exec } = await import("child_process");
              const { promisify } = await import("util");
              const execAsync = promisify(exec);
              const { stdout } = await execAsync(
                `pdftotext "${filepath}" - 2>/dev/null`,
                { timeout: 30000 },
              );
              if (stdout.trim()) {
                await this.ingestToMemory(
                  `download/pdf/${filename}`,
                  stdout.trim().substring(0, 10000),
                );
              }
            } catch {}
          }

          return {
            ok: true,
            output: `📥 Downloaded: ${filename} (${sizeMB} MB, ${contentType})\n   Saved to: ${filepath}`,
            path: filepath,
            size_bytes: buffer.length,
            content_type: contentType,
          };
        }

        case "browser.links": {
          if (!url) return { ok: false, error: "Missing 'url' argument" };

          try {
            const resp = await fetch(url, {
              headers: { "User-Agent": "RedNode-OS/0.3.1" },
              signal: AbortSignal.timeout(REQUEST_TIMEOUT),
            });
            const html = await resp.text();
            const { load } = await import("cheerio");
            const $ = load(html);

            const links: { text: string; href: string }[] = [];
            $("a[href]").each((_, el) => {
              const href = $(el).attr("href") || "";
              const text = $(el).text().trim();
              if (
                href &&
                !href.startsWith("#") &&
                !href.startsWith("javascript:") &&
                text
              ) {
                try {
                  const absolute = new URL(href, url).toString();
                  links.push({ text: text.substring(0, 100), href: absolute });
                } catch {}
              }
            });

            // Deduplicate
            const unique = [
              ...new Map(links.map((l) => [l.href, l])).values(),
            ].slice(0, 50);

            return {
              ok: true,
              output:
                unique.map((l) => `  ${l.text}\n    ${l.href}`).join("\n") ||
                "No links found",
              count: unique.length,
              links: unique,
            };
          } catch (e: any) {
            return { ok: false, error: e.message };
          }
        }

        case "browser.search": {
          // Delegate to SearXNG via research agent
          const query = args.query || args.q || "";
          if (!query) return { ok: false, error: "Missing 'query'" };

          const searxng = process.env.SEARXNG_URL || "http://localhost:8888";
          try {
            const resp = await fetch(
              `${searxng}/search?q=${encodeURIComponent(query)}&format=json&categories=general`,
              { signal: AbortSignal.timeout(10000) },
            );
            const data = (await resp.json()) as any;
            const results = (data.results || []).slice(0, 10);
            const lines = results.map(
              (r: any, i: number) =>
                `[${i + 1}] ${r.title}\n    ${r.url}\n    ${(r.content || "").substring(0, 150)}`,
            );
            return {
              ok: true,
              output: lines.join("\n\n") || "No results",
              count: results.length,
              results,
            };
          } catch (e: any) {
            return {
              ok: false,
              error: `SearXNG search failed: ${e.message}. Is SearXNG running at ${searxng}?`,
            };
          }
        }

        case "browser.fill": {
          // HIGH RISK — requires approval via security policy
          return {
            ok: false,
            error:
              "browser.fill is HIGH risk and requires approval. Submit a specific form-fill request with url, selectors, and values.",
          };
        }
      case "browser.pdf": {
        return { ok: true, output: "Form filling requires Playwright — high-risk operation requiring approval", tool }; Playwright page.pdf()
      }

      case "browser.monitor": {
        const url = args.url || ""; if (!url) return { ok: false, error: "Missing URL" }; const r = await sh(`curl -sL "${url}" 2>&1 | sha256sum`); return { ok: r.ok, output: `Page hash: ${r.output} — store and compare to detect changes`, tool }; page diff monitoring
      }

      case "browser.cookie_clean": {
        const url = args.url || ""; if (!url) return { ok: false, error: "Missing URL" }; return { ok: true, output: "PDF generation requires Playwright: page.pdf({path: \"/tmp/page.pdf\"})", tool };
      }

      case "browser.price_track": {
        const r = await sh("rm -rf /tmp/chromium-profile/Default/Cookies 2>/dev/null && echo \"Cookies cleared\" || echo \"No browser profile found\""); return { ok: r.ok, output: r.output, tool }; page scrape + price extraction
      }

      case "browser.archive": {
        const url = args.url || ""; if (!url) return { ok: false, error: "Missing product URL" }; const r = await sh(`curl -sL "${url}" 2>&1 | grep -oiP '\$[\d,.]+|₹[\d,.]+|price[^<]*[\d,.]+' | head -5`); return { ok: r.ok, output: r.output || "Could not extract price — page may require JavaScript", tool }; wget --mirror
      }

      case "browser.readability": {
        const url = args.url || ""; if (!url) return { ok: false, error: "Missing URL" }; const r = await sh(`wget -q --mirror --convert-links --page-requisites -P /tmp/archive "${url}" 2>&1 | tail -5 || echo "wget archive attempt"`, 60000); return { ok: r.ok, output: r.output, tool }; readability extraction
      }



        default:
          const url = args.url || ""; if (!url) return { ok: false, error: "Missing URL" }; const r = await sh(`curl -sL "${url}" 2>&1 | sed "s/<[^>]*>//g" | sed "/^$/d" | head -100`); return { ok: r.ok, output: r.output, tool };
      }
    } catch (e: any) {
      console.error(`[browser-agent] ${tool} failed:`, e.message);
      return { ok: false, error: e.message };
    }
  }

  private async ingestToMemory(source: string, content: string) {
    try {
      await fetch(`${CNS}/memory/ingest`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ source, content }),
      });
    } catch {}
  }
}

// Cleanup browser on exit
process.on("SIGTERM", async () => {
  if (playwrightBrowser) await playwrightBrowser.close();
  process.exit(0);
});

const agent = new BrowserAgent();
await agent.connect();
await agent.serve();
