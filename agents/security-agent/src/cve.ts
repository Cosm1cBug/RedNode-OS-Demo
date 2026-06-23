// RedNode Security Agent – CVE Auto-Checker
// Scans installed packages against known vulnerabilities
// Privacy-first: offline local DB with optional NVD sync
import { exec } from "child_process";
import { promisify } from "util";
import * as fs from "fs";
const execAsync = promisify(exec);

const CNS = process.env.REDNODE_CNS || "http://localhost:8787";
const CVE_CHECK_INTERVAL = 1000 * 60 * 60 * 6; // 6 hours
const SMART_SECURITY_MODE = process.env.REDNODE_SECURITY_MODE !== "off";
const CVE_DB_PATH = process.env.CVE_DB_PATH || "/var/lib/rednode/cve-db.json";

interface Package {
  name: string;
  version: string;
  manager: string;
}
interface CveMatch {
  cve: string;
  pkg: string;
  installed: string;
  fixed?: string;
  severity: string;
  summary: string;
}

// ─── Package Inventory — Real dpkg/rpm/nix scan ───

async function getInstalledPackages(): Promise<Package[]> {
  const pkgs: Package[] = [];

  // Try dpkg (Debian/Ubuntu/NixOS containers)
  try {
    const { stdout } = await execAsync(
      "dpkg-query -W -f='${Package} ${Version}\\n' 2>/dev/null | head -500",
      { timeout: 10000 },
    );
    for (const line of stdout.trim().split("\n")) {
      const [name, version] = line.split(" ");
      if (name && name.length > 0) {
        pkgs.push({
          name: name.trim(),
          version: (version || "unknown").trim(),
          manager: "dpkg",
        });
      }
    }
  } catch {}

  // Try rpm (RHEL/Fedora)
  if (pkgs.length === 0) {
    try {
      const { stdout } = await execAsync(
        "rpm -qa --queryformat '%{NAME} %{VERSION}-%{RELEASE}\\n' 2>/dev/null | head -500",
        { timeout: 10000 },
      );
      for (const line of stdout.trim().split("\n")) {
        const [name, version] = line.split(" ");
        if (name && name.length > 0) {
          pkgs.push({
            name: name.trim(),
            version: (version || "unknown").trim(),
            manager: "rpm",
          });
        }
      }
    } catch {}
  }

  // Try nix (NixOS)
  if (pkgs.length === 0) {
    try {
      const { stdout } = await execAsync(
        "nix-store --query --requisites /run/current-system 2>/dev/null | sed 's|.*/||' | head -300",
        { timeout: 15000 },
      );
      for (const line of stdout.trim().split("\n")) {
        // NixOS store paths look like: openssl-3.0.13
        const match = line.match(/^([a-zA-Z0-9_-]+?)-([0-9].*)$/);
        if (match) {
          pkgs.push({ name: match[1], version: match[2], manager: "nix" });
        }
      }
    } catch {}
  }

  if (pkgs.length === 0) {
    console.warn("[cve] No package manager found — inventory empty");
  } else {
    console.log(
      `[cve] Package inventory: ${pkgs.length} packages (${pkgs[0]?.manager})`,
    );
  }

  return pkgs;
}

// ─── CVE Database — Local file with known vulnerabilities ───
// In production: sync from NVD API (https://services.nvd.nist.gov/rest/json/cves/2.0)
// For now: load from local JSON file, seeded with common CVEs

interface CveDbEntry {
  cve: string;
  pkg: string;
  affected_before: string; // versions below this are vulnerable
  fixed: string;
  severity: string; // LOW, MEDIUM, HIGH, CRITICAL
  summary: string;
}

let cveDb: CveDbEntry[] = [];

function loadCveDb() {
  // Try loading from persistent file
  try {
    if (fs.existsSync(CVE_DB_PATH)) {
      const data = JSON.parse(fs.readFileSync(CVE_DB_PATH, "utf-8"));
      cveDb = data;
      console.log(`[cve] Loaded ${cveDb.length} CVEs from ${CVE_DB_PATH}`);
      return;
    }
  } catch (e: any) {
    console.warn(`[cve] Failed to load CVE DB from ${CVE_DB_PATH}:`, e.message);
  }

  // Seed with known real CVEs for common packages
  cveDb = [
    {
      cve: "CVE-2024-5535",
      pkg: "openssl",
      affected_before: "3.0.14",
      fixed: "3.0.14",
      severity: "HIGH",
      summary: "OpenSSL SSL_select_next_proto buffer overread",
    },
    {
      cve: "CVE-2024-6119",
      pkg: "openssl",
      affected_before: "3.0.15",
      fixed: "3.0.15",
      severity: "MEDIUM",
      summary: "OpenSSL possible DoS in X.509 name checks",
    },
    {
      cve: "CVE-2024-2961",
      pkg: "glibc",
      affected_before: "2.39",
      fixed: "2.39",
      severity: "HIGH",
      summary: "glibc iconv buffer overflow in ISO-2022-CN-EXT",
    },
    {
      cve: "CVE-2024-47176",
      pkg: "cups",
      affected_before: "2.4.11",
      fixed: "2.4.11",
      severity: "CRITICAL",
      summary:
        "CUPS cups-browsed binds to UDP 631 on all interfaces — RCE chain",
    },
    {
      cve: "CVE-2024-3094",
      pkg: "xz-utils",
      affected_before: "5.6.2",
      fixed: "5.6.2",
      severity: "CRITICAL",
      summary: "xz/liblzma backdoor — supply chain compromise",
    },
    {
      cve: "CVE-2023-44487",
      pkg: "nginx",
      affected_before: "1.25.3",
      fixed: "1.25.3",
      severity: "HIGH",
      summary: "HTTP/2 Rapid Reset attack DoS",
    },
    {
      cve: "CVE-2024-21626",
      pkg: "runc",
      affected_before: "1.1.12",
      fixed: "1.1.12",
      severity: "HIGH",
      summary: "runc container breakout via /proc/self/fd",
    },
    {
      cve: "CVE-2024-0567",
      pkg: "gnutls",
      affected_before: "3.8.3",
      fixed: "3.8.3",
      severity: "MEDIUM",
      summary: "GnuTLS DoS via crafted certificate chain",
    },
    {
      cve: "CVE-2023-50387",
      pkg: "bind9",
      affected_before: "9.18.24",
      fixed: "9.18.24",
      severity: "HIGH",
      summary: "BIND KeyTrap — DNSSEC validation CPU exhaustion",
    },
    {
      cve: "CVE-2024-6387",
      pkg: "openssh",
      affected_before: "9.8",
      fixed: "9.8p1",
      severity: "CRITICAL",
      summary: "regreSSHion — OpenSSH unauthenticated RCE on glibc-based Linux",
    },
  ];

  // Save to disk for persistence
  try {
    const dir = CVE_DB_PATH.substring(0, CVE_DB_PATH.lastIndexOf("/"));
    if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
    fs.writeFileSync(CVE_DB_PATH, JSON.stringify(cveDb, null, 2));
    console.log(
      `[cve] Seeded CVE DB with ${cveDb.length} entries → ${CVE_DB_PATH}`,
    );
  } catch {}
}

// ─── Version Comparison ───

function parseVersion(v: string): number[] {
  return v
    .replace(/[^0-9.]/g, "")
    .split(".")
    .map((n) => parseInt(n) || 0);
}

function versionLt(a: string, b: string): boolean {
  const pa = parseVersion(a);
  const pb = parseVersion(b);
  const len = Math.max(pa.length, pb.length);
  for (let i = 0; i < len; i++) {
    const av = pa[i] || 0;
    const bv = pb[i] || 0;
    if (av < bv) return true;
    if (av > bv) return false;
  }
  return false; // equal
}

// ─── CVE Check ───

async function checkCves(): Promise<CveMatch[]> {
  const pkgs = await getInstalledPackages();
  if (pkgs.length === 0) return [];

  const hits: CveMatch[] = [];
  for (const p of pkgs) {
    for (const cve of cveDb) {
      if (cve.pkg === p.name && cve.fixed) {
        if (versionLt(p.version, cve.affected_before)) {
          hits.push({
            cve: cve.cve,
            pkg: p.name,
            installed: p.version,
            fixed: cve.fixed,
            severity: cve.severity,
            summary: cve.summary,
          });
        }
      }
    }
  }
  return hits;
}

// ─── Report to CNS ───

async function reportSecurityEvent(
  severity: string,
  summary: string,
  raw: any,
) {
  try {
    await fetch(`${CNS}/security/events`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ severity, source: "cve-checker", summary, raw }),
    });
  } catch (e: any) {
    console.warn("[cve] Failed to report to CNS:", e.message);
  }
}

// ─── Main Check Loop ───

export async function runCveCheck() {
  console.log(
    `[security-agent] CVE check starting — Smart Security Mode: ${SMART_SECURITY_MODE ? "ON" : "OFF"} — DB: ${cveDb.length} CVEs`,
  );

  const hits = await checkCves();
  if (hits.length === 0) {
    console.log(
      "[security-agent] CVE check: clean — 0 vulnerabilities found ✅",
    );
    await reportSecurityEvent(
      "LOW",
      "CVE scan completed: 0 vulnerabilities found",
      {
        status: "clean",
        packages_scanned: (await getInstalledPackages()).length,
      },
    );
    return;
  }

  console.log(
    `[security-agent] CVE check: ${hits.length} vulnerable package(s) found ⚠️`,
  );
  for (const h of hits) {
    const summary = `${h.cve} — ${h.pkg} ${h.installed} (fix: ${h.fixed}) — ${h.summary}`;
    console.log(`  ! [${h.severity}] ${summary}`);
    await reportSecurityEvent(h.severity, summary, h);

    // Auto-patcher — only HIGH/CRITICAL in Smart Security Mode
    if (
      SMART_SECURITY_MODE &&
      (h.severity === "HIGH" || h.severity === "CRITICAL")
    ) {
      const { autoPatch } = await import("./patcher.js");
      await autoPatch(h.pkg, h.fixed || "latest", h.cve);
    }
  }
}

// ─── Initialize and Schedule ───

loadCveDb();

// First check after 10 seconds (let everything stabilize)
setTimeout(runCveCheck, 10000);
// Then every 6 hours
setInterval(runCveCheck, CVE_CHECK_INTERVAL);

console.log(
  `[security-agent] CVE auto-checker loaded — ${cveDb.length} CVEs in DB — interval 6h`,
);
