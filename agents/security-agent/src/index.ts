import { RedNodeAgent } from "../../shared/src/agent.js";
import { sh, api, llm, cns } from "../../shared/src/helpers.js";

const TOOLS = [
  "sec.audit_log",
  "sec.audit_verify",
  "sec.cert_monitor",
  "sec.compliance_check",
  "sec.cve_check",
  "sec.darkweb_search",
  "sec.dns_leak_check",
  "sec.entropy",
  "sec.fail2ban_ban",
  "sec.fail2ban_status",
  "sec.fail2ban_unban",
  "sec.firewall_test",
  "sec.harden_ssh",
  "sec.honeypot_status",
  "sec.ids_alerts",
  "sec.ids_stats",
  "sec.ioc_check",
  "sec.log_anomaly",
  "sec.password_audit",
  "sec.patch",
  "sec.port_scan",
  "sec.rootkit_scan",
  "sec.ssh_audit",
  "sec.ssl_check",
  "sec.threat_intel",
  "sec.triage",
  "sec.vuln_scan",
  "sec.yara",
];

class SecurityAgent extends RedNodeAgent {
  constructor() {
    super("security", TOOLS);
  }

  async handleTool(tool: string, args: any): Promise<any> {
    switch (tool) {
      case "sec.threat_intel": {
        const { syncThreatIntel, getStats } = await import("./threat-intel.js");
        const result = await syncThreatIntel();
        const stats = getStats();
        return {
          ok: true,
          output: `Threat Intel: ${result.new_iocs} new IOCs | ${result.blocked_ips} IPs blocked on pfSense | ${result.blocked_domains} domains blocked on Pi-hole | Total: ${stats.total}`,
          result,
          stats,
        };
      }

      case "sec.ioc_check": {
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

      case "sec.darkweb_search": {
        const query = args.query || args.q || args.search || "";
        if (!query) return { ok: false, error: "Missing 'query' — what to search for on the dark web" };
        const { darkwebSearch } = await import("./darkweb.js");
        const result = await darkwebSearch(query);
        const resultLines = result.results.map(
          (r: any, i: number) =>
            `[${i + 1}] ${r.engine}: ${r.title}\n    ${r.url}\n    ${r.snippet}`,
        );
        return {
          ok: true,
          output:
            `🕵️ Dark Web OSINT: "${query}"\n` +
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

      case "sec.triage": {
        const r = await sh("journalctl --since '1 hour ago' --no-pager -p warning 2>&1 | tail -30");
        return { ok: r.ok, output: r.output || "No warnings in last hour", tool };
      }

      case "sec.cve_check": {
        return null; // Handled by cve.ts sub-module + Rust executor
      }

      case "sec.harden_ssh": {
        return null; // High-risk: requires approval via Rust executor
      }

      case "sec.patch": {
        return null; // High-risk: requires approval via Rust executor
      }

      case "sec.yara": {
        return null; // Handled by Rust executor with sandboxing
      }

      case "sec.ssh_audit": {
        const r = await sh("ssh-audit localhost 2>&1 | head -40 || echo 'ssh-audit not installed'", 15000);
        return { ok: r.ok, output: r.output, tool };
      }

      case "sec.ids_alerts": {
        const evePath = process.env.SURICATA_EVE_LOG || "/var/log/suricata/eve.json";
        const r = await sh(`tail -100 ${evePath} 2>/dev/null | grep '"event_type":"alert"' | tail -20 || echo 'No IDS alerts or Suricata not running'`);
        return { ok: r.ok, output: r.output, tool };
      }

      case "sec.ids_stats": {
        const r = await sh("suricatasc -c 'iface-stat enp0s31f6' 2>/dev/null || echo 'Suricata not running'");
        return { ok: r.ok, output: r.output, tool };
      }

      case "sec.ssl_check": {
        const domain = args.domain || args.host || "";
        if (!domain) return { ok: false, error: "Missing 'domain' to check SSL cert" };
        const r = await sh(`echo | openssl s_client -connect ${domain}:443 -servername ${domain} 2>/dev/null | openssl x509 -noout -dates -subject -issuer 2>&1`);
        return { ok: r.ok, output: r.output, tool };
      }

      case "sec.vuln_scan": {
        const target = args.target || "localhost";
        const r = await sh(`nmap --script vuln ${target} 2>&1 | head -60`, 60000);
        return { ok: r.ok, output: r.output, tool };
      }

      case "sec.audit_log": {
        const r = await cns("/memory/audit?limit=" + (args.limit || 20));
        return { ok: r.ok, output: r.output, tool };
      }

      case "sec.audit_verify": {
        const r = await cns("/memory/audit/verify");
        return { ok: r.ok, output: r.output, tool };
      }

      case "sec.fail2ban_status": {
        const r = await sh("fail2ban-client status 2>/dev/null || echo 'fail2ban not installed'");
        return { ok: r.ok, output: r.output, tool };
      }

      case "sec.fail2ban_ban": {
        return null; // High-risk: requires approval
      }

      case "sec.fail2ban_unban": {
        return null; // High-risk: requires approval
      }

      case "sec.firewall_test": {
        const host = args.host || "localhost";
        const port = args.port || 22;
        const r = await sh(`nmap -p ${port} ${host} 2>&1`);
        return { ok: r.ok, output: r.output, tool };
      }

      case "sec.entropy": {
        const r = await sh("cat /proc/sys/kernel/random/entropy_avail");
        if (!r.ok) return { ok: false, error: r.output };
        const val = parseInt(r.output.trim());
        const status = val > 256 ? "healthy" : val > 64 ? "low" : "critical";
        return { ok: true, output: `Entropy pool: ${val} bits (${status})`, tool, entropy: val, status };
      }

      case "sec.rootkit_scan": {
        const r = await sh("chkrootkit 2>&1 | tail -20 || rkhunter --check --sk 2>&1 | tail -20 || echo 'No rootkit scanner installed'", 30000);
        return { ok: r.ok, output: r.output, tool };
      }

      case "sec.password_audit": {
        const r = await sh("lynis audit system --quick --no-colors 2>&1 | grep -A2 'password\\|credential' | head -30 || echo 'lynis not installed'", 30000);
        return { ok: r.ok, output: r.output, tool };
      }

      case "sec.port_scan": {
        const target = args.target || args.ip || "localhost";
        const r = await sh(`nmap -sV --top-ports 100 ${target} 2>&1`, 30000);
        return { ok: r.ok, output: r.output, tool };
      }

      case "sec.dns_leak_check": {
        const piholeIp = process.env.PIHOLE_URL || "10.0.50.2";
        const r = await sh("cat /etc/resolv.conf | grep nameserver | head -1 | awk '{print $2}'");
        const resolver = r.output.trim();
        const leak = resolver !== piholeIp && !resolver.includes("127.0.0.1");
        return {
          ok: true,
          output: leak
            ? `⚠️ DNS leak detected: resolver is ${resolver}, should be ${piholeIp}`
            : `✅ DNS goes through Pi-hole (${resolver})`,
          tool,
          leak,
        };
      }

      case "sec.cert_monitor": {
        const domains = (args.domains || args.domain || "").split(",").filter(Boolean);
        if (!domains.length) return { ok: false, error: "Missing domains (comma-separated)" };
        const results: string[] = [];
        for (const d of domains) {
          const r = await sh(`echo | openssl s_client -connect ${d.trim()}:443 2>/dev/null | openssl x509 -noout -enddate 2>&1`);
          results.push(`${d.trim()}: ${r.output}`);
        }
        return { ok: true, output: results.join("\n"), tool };
      }

      case "sec.log_anomaly": {
        const r = await sh("journalctl --since '1 hour ago' --no-pager -p err 2>&1 | tail -30");
        return { ok: r.ok, output: r.output || "No errors in last hour", tool };
      }

      case "sec.honeypot_status": {
        const r = await sh("systemctl status honeypot 2>&1 || echo 'Honeypot service not configured'");
        return { ok: r.ok, output: r.output, tool };
      }

      case "sec.compliance_check": {
        const r = await sh("lynis audit system --quick --no-colors 2>&1 | tail -30 || echo 'lynis not installed'", 60000);
        return { ok: r.ok, output: r.output, tool };
      }

      default:
        return null;
    }
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
