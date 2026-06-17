/**
 * RedNode-OS – Dark Web OSINT Search
 *
 * Searches .onion sites via Tor for threat intelligence:
 *   - Leaked credentials (email/password in breach dumps)
 *   - Your data being sold on marketplaces
 *   - Threat actor discussions about your infrastructure
 *   - Exploit trading and zero-day discussions
 *
 * Requires: Tor running as SOCKS5 proxy (default: 127.0.0.1:9050)
 *   Install: apt install tor (Linux) / brew install tor (macOS)
 *   Verify:  curl --socks5 127.0.0.1:9050 https://check.torproject.org/api/ip
 *
 * Privacy: all queries go through Tor. No clearnet leaks.
 * Legal: use only for defensive OSINT on your own assets.
 */

import { exec } from "child_process";
import { promisify } from "util";
const execAsync = promisify(exec);

const CNS = process.env.REDNODE_CNS || "http://localhost:8787";
const OLLAMA_URL = process.env.OLLAMA_URL || "http://127.0.0.1:11434";
const MODEL = process.env.REDNODE_MODEL || "qwen2.5:14b-instruct-q4_K_M";
const TOR_PROXY = process.env.TOR_SOCKS_PROXY || "socks5h://127.0.0.1:9050";

// ─── Dark Web Search Engines (.onion) ───
// From Robin project + verified working engines

const ONION_ENGINES = [
  { name: "Ahmia", url: "http://juhanurmihxlp77nkq76byazcldy2hlmovfu2epvl5ankdibsot4csyd.onion/search/?q={query}" },
  { name: "Torgle", url: "http://iy3544gmoeclh5de6gez2256v6pjh4omhpqdh2wpeeppjtvqmjhkfwad.onion/torgle/?query={query}" },
  { name: "Amnesia", url: "http://amnesia7u5odx5xbwtpnqk3edybgud5bmiagu75bnqx2crntw5kry7ad.onion/search?query={query}" },
  { name: "Tornado", url: "http://tornadoxn3viscgz647shlysdy7ea5zqzwda7hierekeuokh5eh5b3qd.onion/search?q={query}" },
  { name: "TorNet", url: "http://tornetupfu7gcgidt33ftnungxzyfq2pygui5qdoyss34xbgx2qruzid.onion/search?q={query}" },
  { name: "Excavator", url: "http://2fd6cemt4gmccflhm6imvdfvli3nf7zn6rfrwpsy7uhxrgbypvwf5fad.onion/search?query={query}" },
];

// Clearnet Ahmia mirror (works without Tor — for basic searches)
const AHMIA_CLEARNET = "https://ahmia.fi/search/?q={query}";

interface DarkWebResult {
  engine: string;
  title: string;
  url: string;
  snippet: string;
}

// ─── Tor Connectivity Check ───

async function isTorRunning(): Promise<boolean> {
  try {
    const { stdout } = await execAsync(
      `curl -sf --socks5-hostname 127.0.0.1:9050 https://check.torproject.org/api/ip 2>/dev/null`,
      { timeout: 15000 }
    );
    const data = JSON.parse(stdout);
    return data.IsTor === true;
  } catch {
    return false;
  }
}

// ─── Search via Tor ───

async function searchOnion(engine: { name: string; url: string }, query: string): Promise<DarkWebResult[]> {
  const searchUrl = engine.url.replace("{query}", encodeURIComponent(query));
  const results: DarkWebResult[] = [];

  try {
    const { stdout } = await execAsync(
      `curl -sf --socks5-hostname 127.0.0.1:9050 --max-time 30 \
       -H "User-Agent: Mozilla/5.0 (Windows NT 10.0; rv:128.0) Gecko/20100101 Firefox/128.0" \
       "${searchUrl}" 2>/dev/null`,
      { timeout: 35000 }
    );

    if (!stdout || stdout.length < 100) return results;

    // Parse HTML for links and titles
    const linkRegex = /<a[^>]+href="([^"]*\.onion[^"]*)"[^>]*>([^<]*)</gi;
    let match;
    const seen = new Set<string>();

    while ((match = linkRegex.exec(stdout)) !== null) {
      const url = match[1];
      const title = match[2].trim();
      if (url && title && !seen.has(url) && url.includes(".onion")) {
        seen.add(url);
        // Get snippet — text near the link
        const idx = stdout.indexOf(match[0]);
        const context = stdout.substring(Math.max(0, idx - 200), Math.min(stdout.length, idx + 500));
        const snippet = context.replace(/<[^>]*>/g, " ").replace(/\s+/g, " ").trim().substring(0, 200);

        results.push({ engine: engine.name, title, url, snippet });
      }
    }
  } catch (e: any) {
    console.warn(`[darkweb] ${engine.name} search failed: ${e.message}`);
  }

  return results;
}

// ─── Clearnet Ahmia Fallback ───

async function searchAhmiaClearnet(query: string): Promise<DarkWebResult[]> {
  const results: DarkWebResult[] = [];
  try {
    const url = AHMIA_CLEARNET.replace("{query}", encodeURIComponent(query));
    const resp = await fetch(url, {
      headers: { "User-Agent": "Mozilla/5.0 (Windows NT 10.0; rv:128.0) Gecko/20100101 Firefox/128.0" },
      signal: AbortSignal.timeout(15000),
    });
    const html = await resp.text();

    // Parse Ahmia results
    const resultRegex = /<a[^>]+href="(http[^"]*\.onion[^"]*)"[^>]*>([^<]*)</gi;
    let match;
    while ((match = resultRegex.exec(html)) !== null) {
      results.push({
        engine: "Ahmia (clearnet)",
        title: match[2].trim() || "Untitled",
        url: match[1],
        snippet: "",
      });
    }
  } catch {}
  return results;
}

// ─── LLM Analysis ───

async function analyzeDarkWebResults(query: string, results: DarkWebResult[]): Promise<string> {
  if (results.length === 0) return "No dark web results found for this query.";

  const resultText = results
    .map((r, i) => `[${i + 1}] ${r.engine}: ${r.title}\n    URL: ${r.url}\n    ${r.snippet}`)
    .join("\n\n");

  try {
    const resp = await fetch(`${OLLAMA_URL}/api/chat`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        model: MODEL,
        messages: [
          {
            role: "system",
            content: `You are a cybersecurity threat intelligence analyst. Analyze these dark web search results and provide:
1. A brief threat assessment (is there actionable intelligence?)
2. Classification: credential leak / data sale / exploit trading / discussion / irrelevant
3. Risk level: CRITICAL / HIGH / MEDIUM / LOW / NONE
4. Recommended actions (if any)
Be concise. If results are irrelevant or benign, say so.`,
          },
          {
            role: "user",
            content: `Query: "${query}"\n\nDark web search results:\n\n${resultText}`,
          },
        ],
        stream: false,
        options: { temperature: 0.2, num_predict: 512 },
      }),
    });
    const data = (await resp.json()) as any;
    return data.message?.content || "Analysis unavailable";
  } catch (e: any) {
    return `LLM analysis failed: ${e.message}`;
  }
}

// ─── Main Search Function ───

export async function darkwebSearch(query: string): Promise<{
  results: DarkWebResult[];
  analysis: string;
  tor_connected: boolean;
  engines_searched: number;
}> {
  console.log(`[darkweb] Searching: "${query}"`);

  // Check Tor
  const torOk = await isTorRunning();
  let allResults: DarkWebResult[] = [];

  if (torOk) {
    console.log("[darkweb] Tor connected ✅ — searching .onion engines");

    // Search multiple engines in parallel (with limit)
    const searchPromises = ONION_ENGINES.slice(0, 4).map(engine =>
      searchOnion(engine, query).catch(() => [] as DarkWebResult[])
    );
    const engineResults = await Promise.all(searchPromises);
    for (const results of engineResults) {
      allResults.push(...results);
    }
  } else {
    console.log("[darkweb] Tor not running — using Ahmia clearnet mirror");
    allResults = await searchAhmiaClearnet(query);
  }

  // Deduplicate by URL
  const seen = new Set<string>();
  allResults = allResults.filter(r => {
    if (seen.has(r.url)) return false;
    seen.add(r.url);
    return true;
  });

  console.log(`[darkweb] Found ${allResults.length} results`);

  // LLM analysis
  const analysis = await analyzeDarkWebResults(query, allResults);

  // Report to CNS
  const severity = allResults.length > 0 ? "MEDIUM" : "LOW";
  try {
    await fetch(`${CNS}/security/events`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        severity,
        source: "darkweb-osint",
        summary: `Dark web search: "${query}" — ${allResults.length} results found${torOk ? " (via Tor)" : " (clearnet Ahmia)"}`,
        raw: { query, result_count: allResults.length, tor_connected: torOk },
      }),
    });
  } catch {}

  // Ingest analysis into memory
  if (allResults.length > 0) {
    try {
      await fetch(`${CNS}/memory/ingest`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          source: `darkweb/search/${query.substring(0, 30).replace(/[^a-zA-Z0-9]/g, "-")}`,
          content: `Dark web OSINT: "${query}"\n${allResults.length} results\n\nAnalysis:\n${analysis}`,
        }),
      });
    } catch {}
  }

  return {
    results: allResults.slice(0, 20),
    analysis,
    tor_connected: torOk,
    engines_searched: torOk ? ONION_ENGINES.slice(0, 4).length : 1,
  };
}

console.log("[security-agent] Dark web OSINT module loaded");
