/**
 * RedNode-OS – Threat Intelligence Feed Aggregator
 *
 * Auto-fetches IOCs (Indicators of Compromise) from:
 *   1. AlienVault OTX — free, community-driven threat intel
 *   2. Abuse.ch — malware/botnet IP/domain blocklists
 *   3. Emerging Threats — Proofpoint ET open rules
 *
 * IOCs are:
 *   - Stored locally at /var/lib/rednode/threat-intel/
 *   - Auto-blocked on pfSense firewall via API (if configured)
 *   - Reported as security events to CNS
 *   - Used by Pi-hole (domain IOCs → blocklist)
 *
 * Privacy: all feeds are fetched directly (no third-party proxy).
 * VirusTotal requires API key (free tier: 4 req/min, 500/day).
 */

import * as fs from "fs";
import * as path from "path";

const CNS = process.env.REDNODE_CNS || "http://localhost:8787";
const INTEL_DIR = process.env.THREAT_INTEL_DIR || "/var/lib/rednode/threat-intel";
const PFSENSE_URL = process.env.PFSENSE_URL || "";
const PFSENSE_API_KEY = process.env.PFSENSE_API_KEY || "";
const PFSENSE_API_SECRET = process.env.PFSENSE_API_SECRET || "";
const VT_API_KEY = process.env.VIRUSTOTAL_API_KEY || "";
const OTX_API_KEY = process.env.OTX_API_KEY || "";
const SYNC_INTERVAL = parseInt(process.env.THREAT_INTEL_INTERVAL || "3600000"); // 1 hour
const AUTO_BLOCK = process.env.THREAT_INTEL_AUTO_BLOCK === "true";

// ─── IOC Types ───

interface IOC {
  type: "ip" | "domain" | "url" | "hash";
  value: string;
  source: string;
  severity: "low" | "medium" | "high" | "critical";
  description: string;
  first_seen: string;
  tags: string[];
}

let iocDatabase: IOC[] = [];
const blockedIPs = new Set<string>();
const blockedDomains = new Set<string>();

// ─── Feed Fetchers ───

async function fetchAbuseCH(): Promise<IOC[]> {
  const iocs: IOC[] = [];

  // Feodo Tracker — banking trojan C2 servers
  try {
    const resp = await fetch("https://feodotracker.abuse.ch/downloads/ipblocklist_recommended.txt", {
      signal: AbortSignal.timeout(15000),
    });
    const text = await resp.text();
    for (const line of text.split("\n")) {
      const ip = line.trim();
      if (ip && !ip.startsWith("#") && /^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$/.test(ip)) {
        iocs.push({
          type: "ip", value: ip, source: "abuse.ch/feodo",
          severity: "critical", description: "Feodo Tracker — banking trojan C2",
          first_seen: new Date().toISOString(), tags: ["malware", "c2", "banking"],
        });
      }
    }
    console.log(`[threat-intel] abuse.ch/feodo: ${iocs.length} IPs`);
  } catch (e: any) {
    console.warn(`[threat-intel] abuse.ch/feodo failed: ${e.message}`);
  }

  // SSL Blacklist — malicious SSL certificates
  try {
    const resp = await fetch("https://sslbl.abuse.ch/blacklist/sslipblacklist.txt", {
      signal: AbortSignal.timeout(15000),
    });
    const text = await resp.text();
    let sslCount = 0;
    for (const line of text.split("\n")) {
      const ip = line.trim();
      if (ip && !ip.startsWith("#") && /^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$/.test(ip)) {
        iocs.push({
          type: "ip", value: ip, source: "abuse.ch/sslbl",
          severity: "high", description: "SSL Blacklist — malicious certificate",
          first_seen: new Date().toISOString(), tags: ["malware", "ssl"],
        });
        sslCount++;
      }
    }
    console.log(`[threat-intel] abuse.ch/sslbl: ${sslCount} IPs`);
  } catch (e: any) {
    console.warn(`[threat-intel] abuse.ch/sslbl failed: ${e.message}`);
  }

  // URLhaus — malware distribution URLs
  try {
    const resp = await fetch("https://urlhaus.abuse.ch/downloads/text_online/", {
      signal: AbortSignal.timeout(15000),
    });
    const text = await resp.text();
    let urlCount = 0;
    for (const line of text.split("\n")) {
      const url = line.trim();
      if (url && !url.startsWith("#") && url.startsWith("http")) {
        try {
          const domain = new URL(url).hostname;
          iocs.push({
            type: "domain", value: domain, source: "abuse.ch/urlhaus",
            severity: "high", description: `Malware distribution: ${url.substring(0, 100)}`,
            first_seen: new Date().toISOString(), tags: ["malware", "distribution"],
          });
          urlCount++;
        } catch {}
      }
    }
    console.log(`[threat-intel] abuse.ch/urlhaus: ${urlCount} domains`);
  } catch (e: any) {
    console.warn(`[threat-intel] abuse.ch/urlhaus failed: ${e.message}`);
  }

  return iocs;
}

async function fetchAlienVaultOTX(): Promise<IOC[]> {
  if (!OTX_API_KEY) {
    console.log("[threat-intel] AlienVault OTX: no API key (set OTX_API_KEY for premium feeds)");
    return [];
  }

  const iocs: IOC[] = [];
  try {
    // Fetch recent pulses (threat reports)
    const resp = await fetch("https://otx.alienvault.com/api/v1/pulses/subscribed?limit=10&page=1", {
      headers: { "X-OTX-API-KEY": OTX_API_KEY },
      signal: AbortSignal.timeout(20000),
    });
    const data = await resp.json() as any;

    for (const pulse of data.results || []) {
      for (const indicator of pulse.indicators || []) {
        const type = indicator.type === "IPv4" ? "ip" :
                     indicator.type === "domain" ? "domain" :
                     indicator.type === "URL" ? "url" :
                     indicator.type === "FileHash-SHA256" ? "hash" : null;
        if (type) {
          iocs.push({
            type: type as any,
            value: indicator.indicator,
            source: `otx/${pulse.id}`,
            severity: "high",
            description: `${pulse.name}: ${pulse.description?.substring(0, 100) || ""}`,
            first_seen: indicator.created || new Date().toISOString(),
            tags: pulse.tags || [],
          });
        }
      }
    }
    console.log(`[threat-intel] AlienVault OTX: ${iocs.length} IOCs from ${data.results?.length || 0} pulses`);
  } catch (e: any) {
    console.warn(`[threat-intel] AlienVault OTX failed: ${e.message}`);
  }
  return iocs;
}

async function fetchEmergingThreats(): Promise<IOC[]> {
  const iocs: IOC[] = [];
  try {
    // ET compromised IPs
    const resp = await fetch("https://rules.emergingthreats.net/blockrules/compromised-ips.txt", {
      signal: AbortSignal.timeout(15000),
    });
    const text = await resp.text();
    for (const line of text.split("\n")) {
      const ip = line.trim();
      if (ip && !ip.startsWith("#") && /^\d{1,3}\.\d{1,3}\.\d{1,3}\.\d{1,3}$/.test(ip)) {
        iocs.push({
          type: "ip", value: ip, source: "emergingthreats",
          severity: "high", description: "Emerging Threats — compromised IP",
          first_seen: new Date().toISOString(), tags: ["compromised"],
        });
      }
    }
    console.log(`[threat-intel] Emerging Threats: ${iocs.length} IPs`);
  } catch (e: any) {
    console.warn(`[threat-intel] Emerging Threats failed: ${e.message}`);
  }
  return iocs;
}

// ─── pfSense Auto-Block ───

async function blockOnPfSense(ips: string[]): Promise<{ blocked: number; errors: number }> {
  if (!PFSENSE_URL || !PFSENSE_API_KEY) {
    return { blocked: 0, errors: 0 };
  }

  let blocked = 0;
  let errors = 0;

  // pfSense REST API — add firewall alias entries
  // This uses pfSense-FauxAPI or pfSense API package
  try {
    // First, get or create the RedNode blocklist alias
    const aliasName = "rednode_threat_blocklist";

    // Add IPs to the alias (batch)
    const batchSize = 100;
    for (let i = 0; i < ips.length; i += batchSize) {
      const batch = ips.slice(i, i + batchSize);

      try {
        const resp = await fetch(`${PFSENSE_URL}/api/v1/firewall/alias`, {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
            "Authorization": `${PFSENSE_API_KEY} ${PFSENSE_API_SECRET}`,
          },
          body: JSON.stringify({
            name: aliasName,
            type: "host",
            descr: `RedNode Threat Intel — auto-updated ${new Date().toISOString()}`,
            address: batch.join(" "),
            detail: batch.map(() => "RedNode threat intel").join("||"),
          }),
        });

        if (resp.ok) {
          blocked += batch.length;
        } else {
          errors++;
          console.warn(`[threat-intel] pfSense batch block failed: ${resp.status}`);
        }
      } catch (e: any) {
        errors++;
        console.warn(`[threat-intel] pfSense API error: ${e.message}`);
      }
    }

    // Apply the firewall rules
    if (blocked > 0) {
      await fetch(`${PFSENSE_URL}/api/v1/firewall/apply`, {
        method: "POST",
        headers: {
          "Authorization": `${PFSENSE_API_KEY} ${PFSENSE_API_SECRET}`,
        },
      });
      console.log(`[threat-intel] pfSense: ${blocked} IPs blocked via alias '${aliasName}'`);
    }
  } catch (e: any) {
    console.error(`[threat-intel] pfSense integration failed: ${e.message}`);
    errors++;
  }

  return { blocked, errors };
}

// ─── Pi-hole Domain Blocking ───

async function blockDomainsOnPihole(domains: string[]): Promise<number> {
  const PIHOLE_URL = process.env.PIHOLE_URL || "";
  const PIHOLE_PASSWORD = process.env.PIHOLE_PASSWORD || "";
  if (!PIHOLE_URL || !PIHOLE_PASSWORD || domains.length === 0) return 0;

  let blocked = 0;
  try {
    // Authenticate
    const authResp = await fetch(`${PIHOLE_URL}/api/auth`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ password: PIHOLE_PASSWORD }),
    });
    const authData = await authResp.json() as any;
    const sid = authData?.session?.sid;
    if (!sid) return 0;

    // Add domains to blocklist
    for (const domain of domains.slice(0, 500)) { // limit to 500 per sync
      try {
        await fetch(`${PIHOLE_URL}/api/lists?sid=${sid}`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
            address: domain,
            type: "deny",
            comment: `RedNode threat intel — ${new Date().toISOString()}`,
          }),
        });
        blocked++;
      } catch {}
    }

    // Logout
    await fetch(`${PIHOLE_URL}/api/auth?sid=${sid}`, { method: "DELETE" });
    console.log(`[threat-intel] Pi-hole: ${blocked} malicious domains blocked`);
  } catch (e: any) {
    console.warn(`[threat-intel] Pi-hole domain blocking failed: ${e.message}`);
  }
  return blocked;
}

// ─── Main Sync ───

export async function syncThreatIntel(): Promise<{
  total_iocs: number;
  new_iocs: number;
  blocked_ips: number;
  blocked_domains: number;
}> {
  console.log("[threat-intel] Syncing threat intelligence feeds...");
  const startTime = Date.now();

  // Fetch from all sources
  const [abuseCH, otx, et] = await Promise.all([
    fetchAbuseCH(),
    fetchAlienVaultOTX(),
    fetchEmergingThreats(),
  ]);

  const allIOCs = [...abuseCH, ...otx, ...et];

  // Deduplicate
  const seen = new Set<string>();
  const newIOCs: IOC[] = [];
  for (const ioc of allIOCs) {
    const key = `${ioc.type}:${ioc.value}`;
    if (!seen.has(key) && !blockedIPs.has(ioc.value) && !blockedDomains.has(ioc.value)) {
      seen.add(key);
      newIOCs.push(ioc);
    }
  }

  // Update database
  iocDatabase = [...iocDatabase, ...newIOCs];
  // Keep last 50k IOCs
  if (iocDatabase.length > 50000) {
    iocDatabase = iocDatabase.slice(-50000);
  }

  // Save to disk
  try {
    fs.mkdirSync(INTEL_DIR, { recursive: true });
    fs.writeFileSync(
      path.join(INTEL_DIR, "iocs.json"),
      JSON.stringify({ updated: new Date().toISOString(), count: iocDatabase.length, iocs: iocDatabase.slice(-1000) }, null, 2)
    );
  } catch {}

  // Auto-block on pfSense (IPs)
  let pfBlocked = 0;
  if (AUTO_BLOCK) {
    const newIPs = newIOCs.filter(i => i.type === "ip" && (i.severity === "high" || i.severity === "critical")).map(i => i.value);
    if (newIPs.length > 0) {
      const result = await blockOnPfSense(newIPs);
      pfBlocked = result.blocked;
      newIPs.forEach(ip => blockedIPs.add(ip));
    }
  }

  // Auto-block malicious domains on Pi-hole
  const newDomains = newIOCs.filter(i => i.type === "domain" && (i.severity === "high" || i.severity === "critical")).map(i => i.value);
  const phBlocked = await blockDomainsOnPihole(newDomains);
  newDomains.forEach(d => blockedDomains.add(d));

  // Report to CNS
  const elapsed = ((Date.now() - startTime) / 1000).toFixed(1);
  const summary = `Threat intel sync: ${newIOCs.length} new IOCs (${allIOCs.length} total) | ` +
    `${pfBlocked} IPs blocked on pfSense | ${phBlocked} domains blocked on Pi-hole | ${elapsed}s`;
  console.log(`[threat-intel] ${summary}`);

  try {
    await fetch(`${CNS}/security/events`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        severity: newIOCs.length > 100 ? "MEDIUM" : "LOW",
        source: "threat-intel",
        summary,
        raw: {
          sources: { abuse_ch: abuseCH.length, otx: otx.length, emerging_threats: et.length },
          new_iocs: newIOCs.length,
          blocked_ips: pfBlocked,
          blocked_domains: phBlocked,
        },
      }),
    });
  } catch {}

  return {
    total_iocs: iocDatabase.length,
    new_iocs: newIOCs.length,
    blocked_ips: pfBlocked,
    blocked_domains: phBlocked,
  };
}

/**
 * Check if a specific IP or domain is in the threat intel database.
 */
export function checkIOC(value: string): IOC | null {
  return iocDatabase.find(i => i.value === value) || null;
}

/**
 * Get threat intel statistics.
 */
export function getStats(): { total: number; by_source: Record<string, number>; by_type: Record<string, number> } {
  const by_source: Record<string, number> = {};
  const by_type: Record<string, number> = {};
  for (const ioc of iocDatabase) {
    by_source[ioc.source] = (by_source[ioc.source] || 0) + 1;
    by_type[ioc.type] = (by_type[ioc.type] || 0) + 1;
  }
  return { total: iocDatabase.length, by_source, by_type };
}

// ─── Load existing IOCs from disk ───

try {
  const dbPath = path.join(INTEL_DIR, "iocs.json");
  if (fs.existsSync(dbPath)) {
    const data = JSON.parse(fs.readFileSync(dbPath, "utf-8"));
    iocDatabase = data.iocs || [];
    console.log(`[threat-intel] Loaded ${iocDatabase.length} IOCs from disk`);
  }
} catch {}

// ─── Schedule ───

// First sync after 30 seconds
setTimeout(syncThreatIntel, 30000);
// Then every hour (or configured interval)
setInterval(syncThreatIntel, SYNC_INTERVAL);

console.log(`[threat-intel] Feed aggregator loaded — sync interval: ${SYNC_INTERVAL / 60000}min | auto-block: ${AUTO_BLOCK}`);
