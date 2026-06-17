import { RedNodeAgent } from "../../shared/src/agent.js";

const TOOLS = [
  "sec.triage",
  "sec.cve_check",
  "sec.harden_ssh",
  "sec.patch",
  "sec.yara",
  "sec.threat_intel",
  "sec.ioc_check",
  "sec.darkweb_search",
];

class SecurityAgent extends RedNodeAgent {
  constructor() { super("security", TOOLS); }
  async handleTool(tool: string, args: any) {
    if (tool === "sec.threat_intel") {
      const { syncThreatIntel, getStats } = await import("./threat-intel.js");
      const result = await syncThreatIntel();
      const stats = getStats();
      return {
        ok: true,
        output: `Threat Intel: ${result.new_iocs} new IOCs | ${result.blocked_ips} IPs blocked on pfSense | ${result.blocked_domains} domains blocked on Pi-hole | Total: ${stats.total}`,
        result, stats,
      };
    }
    if (tool === "sec.ioc_check") {
      const value = args.value || args.ip || args.domain || "";
      if (!value) return { ok: false, error: "Missing 'value' (IP or domain to check)" };
      const { checkIOC } = await import("./threat-intel.js");
      const match = checkIOC(value);
      return {
        ok: true,
        output: match
          ? `⚠️ MATCH: ${value} — ${match.source} — ${match.severity} — ${match.description}`
          : `✅ ${value} — not found in threat intel database`,
        match,
      };
    }
    if (tool === "sec.darkweb_search") {
      const query = args.query || args.q || args.search || "";
      if (!query) return { ok: false, error: "Missing 'query' — what to search for on the dark web" };
      const { darkwebSearch } = await import("./darkweb.js");
      const result = await darkwebSearch(query);
      const resultLines = result.results.map((r, i) =>
        `[${i + 1}] ${r.engine}: ${r.title}\n    ${r.url}\n    ${r.snippet}`
      );
      return {
        ok: true,
        output: `🕵️ Dark Web OSINT: "${query}"\n` +
          `Tor: ${result.tor_connected ? "✅ connected" : "❌ not running (clearnet fallback)"}\n` +
          `Engines searched: ${result.engines_searched}\n` +
          `Results: ${result.results.length}\n\n` +
          (resultLines.length > 0 ? resultLines.join("\n\n") + "\n\n" : "") +
          `── Analysis ──\n${result.analysis}`,
        results: result.results,
        analysis: result.analysis,
        tor_connected: result.tor_connected,
      };
    }
    // All other tools → fall through to Rust executor
    return null;
  }
}

const agent = new SecurityAgent();
await agent.connect();

// Load autonomous sub-modules
import "./cve.js";
import "./falco.js";
import "./patcher.js";
import "./threat-intel.js";

// Start NVD CVE sync
import { startNvdSync } from "./nvd-sync.js";
startNvdSync();

await agent.serve();
