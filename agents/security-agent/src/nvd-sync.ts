/**
 * RedNode-OS – NVD CVE Database Sync
 *
 * Fetches CVE data from NIST National Vulnerability Database.
 * Privacy-first: direct API access, no third-party proxy.
 *
 * NVD API v2.0: https://services.nvd.nist.gov/rest/json/cves/2.0
 *   - Free tier: 5 requests per 30 seconds (no key)
 *   - With API key: 50 requests per 30 seconds
 *   - Get key: https://nvd.nist.gov/developers/request-an-api-key
 *
 * Sync strategy:
 *   - Initial: fetch CVEs modified in last 30 days for our installed packages
 *   - Incremental: fetch CVEs modified since last sync timestamp
 *   - Match against installed package inventory (dpkg/rpm/nix)
 */

import * as fs from "fs";
import * as path from "path";

const NVD_API = "https://services.nvd.nist.gov/rest/json/cves/2.0";
const NVD_API_KEY = process.env.NVD_API_KEY || "";
const CVE_DB_PATH = process.env.CVE_DB_PATH || "/var/lib/rednode/cve-db.json";
const SYNC_STATE_PATH = process.env.CVE_SYNC_STATE || "/var/lib/rednode/nvd-sync-state.json";
const NVD_SYNC_INTERVAL = parseInt(process.env.NVD_SYNC_INTERVAL || "86400000"); // 24 hours

interface NvdCve {
  cve: string;
  pkg: string;
  affected_before: string;
  fixed: string;
  severity: string;
  summary: string;
}

interface SyncState {
  last_sync: string;
  total_cves: number;
}

// ─── NVD API ───

async function nvdFetch(params: Record<string, string>): Promise<any> {
  const url = new URL(NVD_API);
  for (const [k, v] of Object.entries(params)) {
    url.searchParams.set(k, v);
  }

  const headers: Record<string, string> = {};
  if (NVD_API_KEY) {
    headers["apiKey"] = NVD_API_KEY;
  }

  // Rate limiting: 5 req/30s without key, 50/30s with key
  const delay = NVD_API_KEY ? 700 : 6500; // ms between requests
  await new Promise(r => setTimeout(r, delay));

  const resp = await fetch(url.toString(), {
    headers,
    signal: AbortSignal.timeout(30000),
  });

  if (resp.status === 403) {
    throw new Error("NVD API rate limited — wait 30 seconds and retry");
  }
  if (!resp.ok) {
    throw new Error(`NVD API error: ${resp.status} ${resp.statusText}`);
  }

  return resp.json();
}

// ─── Severity Mapping ───

function mapCvss(score: number): string {
  if (score >= 9.0) return "CRITICAL";
  if (score >= 7.0) return "HIGH";
  if (score >= 4.0) return "MEDIUM";
  return "LOW";
}

// ─── Sync ───

export async function syncNVD(packages: string[]): Promise<NvdCve[]> {
  console.log(`[nvd-sync] Starting NVD sync for ${packages.length} packages...`);

  // Load sync state
  let state: SyncState = { last_sync: "", total_cves: 0 };
  try {
    if (fs.existsSync(SYNC_STATE_PATH)) {
      state = JSON.parse(fs.readFileSync(SYNC_STATE_PATH, "utf-8"));
    }
  } catch {}

  const newCves: NvdCve[] = [];

  // Query NVD for each package (batch by keyword)
  // Focus on high-value packages that are commonly vulnerable
  const priorityPackages = packages.filter(p =>
    ["openssl", "openssh", "nginx", "curl", "glibc", "linux", "docker",
     "postgresql", "nodejs", "python", "go", "rust", "cups", "bind",
     "nats", "grafana", "prometheus", "systemd", "sudo", "bash",
     "xz", "gnutls", "libxml", "libpng", "zlib", "sqlite",
    ].includes(p.toLowerCase())
  );

  const searchPackages = priorityPackages.length > 0 ? priorityPackages : packages.slice(0, 20);

  for (const pkg of searchPackages) {
    try {
      // Build time range for incremental sync
      const params: Record<string, string> = {
        keywordSearch: pkg,
        keywordExactMatch: "",
        resultsPerPage: "20",
      };

      if (state.last_sync) {
        params.lastModStartDate = state.last_sync;
        params.lastModEndDate = new Date().toISOString();
      } else {
        // Initial sync: last 90 days
        const days90 = new Date(Date.now() - 90 * 24 * 60 * 60 * 1000);
        params.lastModStartDate = days90.toISOString();
        params.lastModEndDate = new Date().toISOString();
      }

      const data = await nvdFetch(params);

      for (const vuln of data.vulnerabilities || []) {
        const cve = vuln.cve;
        const id = cve.id; // e.g., CVE-2024-6387

        // Get severity from CVSS
        const metrics = cve.metrics?.cvssMetricV31?.[0] || cve.metrics?.cvssMetricV30?.[0] || cve.metrics?.cvssMetricV2?.[0];
        const score = metrics?.cvssData?.baseScore || 0;
        const severity = mapCvss(score);

        // Get description
        const description = cve.descriptions?.find((d: any) => d.lang === "en")?.value || "";

        // Try to find affected/fixed versions from configurations
        let affectedBefore = "";
        let fixed = "";
        for (const node of cve.configurations?.[0]?.nodes || []) {
          for (const match of node.cpeMatch || []) {
            if (match.vulnerable && match.criteria?.toLowerCase().includes(pkg.toLowerCase())) {
              affectedBefore = match.versionEndExcluding || match.versionEndIncluding || "";
              fixed = match.versionEndExcluding || "";
            }
          }
        }

        if (severity === "HIGH" || severity === "CRITICAL" || affectedBefore) {
          newCves.push({
            cve: id,
            pkg,
            affected_before: affectedBefore || "unknown",
            fixed: fixed || "latest",
            severity,
            summary: description.substring(0, 200),
          });
        }
      }

      console.log(`[nvd-sync] ${pkg}: ${data.totalResults || 0} CVEs found`);
    } catch (e: any) {
      console.warn(`[nvd-sync] ${pkg} query failed: ${e.message}`);
      if (e.message.includes("rate limited")) break; // stop on rate limit
    }
  }

  // Merge with existing CVE DB
  if (newCves.length > 0) {
    try {
      let existingDb: NvdCve[] = [];
      if (fs.existsSync(CVE_DB_PATH)) {
        existingDb = JSON.parse(fs.readFileSync(CVE_DB_PATH, "utf-8"));
      }

      // Deduplicate by CVE ID
      const existing = new Set(existingDb.map(c => c.cve));
      const added = newCves.filter(c => !existing.has(c.cve));
      const merged = [...existingDb, ...added];

      const dir = path.dirname(CVE_DB_PATH);
      if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
      fs.writeFileSync(CVE_DB_PATH, JSON.stringify(merged, null, 2));
      console.log(`[nvd-sync] CVE DB updated: ${added.length} new, ${merged.length} total`);
    } catch (e: any) {
      console.error(`[nvd-sync] Failed to save CVE DB: ${e.message}`);
    }
  }

  // Update sync state
  state.last_sync = new Date().toISOString();
  state.total_cves = newCves.length;
  try {
    const dir = path.dirname(SYNC_STATE_PATH);
    if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
    fs.writeFileSync(SYNC_STATE_PATH, JSON.stringify(state, null, 2));
  } catch {}

  console.log(`[nvd-sync] Sync complete: ${newCves.length} new CVEs`);
  return newCves;
}

// ─── Auto-schedule ───

export function startNvdSync() {
  // First sync after 2 minutes (let everything stabilize)
  setTimeout(async () => {
    try {
      // Get installed packages
      const { exec } = await import("child_process");
      const { promisify } = await import("util");
      const execAsync = promisify(exec);

      let packages: string[] = [];
      try {
        const { stdout } = await execAsync(
          "dpkg-query -W -f='${Package}\\n' 2>/dev/null | head -100",
          { timeout: 10000 }
        );
        packages = stdout.trim().split("\n").filter(p => p.length > 0);
      } catch {}

      if (packages.length === 0) {
        // Fallback: check common packages
        packages = ["openssl", "openssh", "curl", "nginx", "docker", "postgresql", "nodejs", "python3"];
      }

      await syncNVD(packages);
    } catch (e: any) {
      console.error(`[nvd-sync] Scheduled sync failed: ${e.message}`);
    }
  }, 120000);

  // Then every 24 hours
  setInterval(async () => {
    try {
      const { exec } = await import("child_process");
      const { promisify } = await import("util");
      const execAsync = promisify(exec);
      const { stdout } = await execAsync("dpkg-query -W -f='${Package}\\n' 2>/dev/null | head -100", { timeout: 10000 });
      const packages = stdout.trim().split("\n").filter((p: string) => p.length > 0);
      await syncNVD(packages.length > 0 ? packages : ["openssl", "openssh", "curl"]);
    } catch {}
  }, NVD_SYNC_INTERVAL);
}

console.log(`[security-agent] NVD sync module loaded — interval: ${NVD_SYNC_INTERVAL / 3600000}h`);
