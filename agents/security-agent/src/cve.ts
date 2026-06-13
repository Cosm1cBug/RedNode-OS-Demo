// RedNode Security Agent – CVE Auto-Checker
// Privacy-first: offline CVE DB with optional NVD sync
import { exec } from 'child_process';
import { promisify } from 'util';
const execAsync = promisify(exec);

const CNS = process.env.REDNODE_CNS || 'http://localhost:8787';
const CVE_CHECK_INTERVAL = 1000 * 60 * 60 * 6; // 6h
const SMART_SECURITY_MODE = process.env.REDNODE_SECURITY_MODE !== 'off';

interface Package { name: string; version: string; manager: string }
interface CveMatch { cve: string; pkg: string; installed: string; fixed?: string; severity: string; summary: string }

async function getInstalledPackages(): Promise<Package[]> {
  const pkgs: Package[] = [];
  try {
    // dpkg – Debian/Ubuntu
    const { stdout } = await execAsync("dpkg-query -W -f='${Package} ${Version}\\n' 2>/dev/null | head -200");
    for (const line of stdout.trim().split('\n')) {
      const [name, version] = line.split(' ');
      if (name) pkgs.push({ name, version: version || 'unknown', manager: 'dpkg' });
    }
  } catch {}
  if (pkgs.length === 0) {
    // Fallback mock for dev
    pkgs.push(
      { name: 'openssl', version: '3.0.2', manager: 'dpkg' },
      { name: 'nginx', version: '1.18.0', manager: 'dpkg' },
      { name: 'curl', version: '7.81.0', manager: 'dpkg' }
    );
  }
  return pkgs;
}

// Local CVE DB – in production sync from https://services.nvd.nist.gov/rest/json/cves/2.0
// For RedNode Phase 1: embedded minimal DB + allow live fetch with consent
const LOCAL_CVE_DB: CveMatch[] = [
  { cve: 'CVE-2024-1234', pkg: 'openssl', installed: '<3.0.13', fixed: '3.0.13', severity: 'HIGH', summary: 'OpenSSL buffer overflow – simulated for RedNode demo' },
  { cve: 'CVE-2023-5678', pkg: 'nginx', installed: '<1.24.0', fixed: '1.24.0', severity: 'MEDIUM', summary: 'Nginx HTTP/2 rapid reset' },
];

function versionLt(a: string, b: string): boolean {
  // naive semver compare – replace with proper semver in prod
  return a.localeCompare(b, undefined, { numeric: true }) < 0;
}

async function checkCves(): Promise<CveMatch[]> {
  const pkgs = await getInstalledPackages();
  const hits: CveMatch[] = [];
  for (const p of pkgs) {
    for (const cve of LOCAL_CVE_DB) {
      if (cve.pkg === p.name) {
        // crude version check
        const fixed = cve.fixed || '';
        if (fixed && versionLt(p.version, fixed)) {
          hits.push({ ...cve, installed: p.version });
        }
      }
    }
  }
  return hits;
}

async function reportSecurityEvent(severity: string, summary: string, raw: any) {
  try {
    await fetch(`${CNS}/security/events`, {
      method: 'POST',
      headers: {'Content-Type':'application/json'},
      body: JSON.stringify({ severity, source: 'cve-checker', summary, raw })
    });
  } catch (e) {
    console.warn('[cve] failed to report to CNS', e);
  }
}

export async function runCveCheck() {
  console.log('[security-agent] CVE check starting – Smart Security Mode:', SMART_SECURITY_MODE ? 'ON' : 'OFF');
  const hits = await checkCves();
  if (hits.length === 0) {
    console.log('[security-agent] CVE check: clean – 0 vulnerabilities');
    return;
  }
  console.log(`[security-agent] CVE check: ${hits.length} vulnerable package(s) found`);
  for (const h of hits) {
    const summary = `${h.cve} – ${h.pkg} ${h.installed} – ${h.summary}`;
    console.log('  !', summary);
    await reportSecurityEvent(h.severity, summary, h);
    
    // Auto-patcher – only for HIGH/CRITICAL and if Smart Security Mode is ON
    if (SMART_SECURITY_MODE && (h.severity === 'HIGH' || h.severity === 'CRITICAL')) {
      const { autoPatch } = await import('./patcher.js');
      await autoPatch(h.pkg, h.fixed || 'latest', h.cve);
    }
  }
}

// Run on start, then every 6h
setTimeout(runCveCheck, 5000);
setInterval(runCveCheck, CVE_CHECK_INTERVAL);

console.log('[security-agent] CVE auto-checker loaded – interval 6h');
