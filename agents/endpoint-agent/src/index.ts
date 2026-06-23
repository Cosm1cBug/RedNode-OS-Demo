/**
 * RedNode-OS – Lightweight Endpoint Agent
 *
 * Installed on each machine in your network (Linux, Windows, macOS).
 * Reports installed packages, running services, open ports, and system info
 * to the RedNode CNS for centralized CVE scanning and lateral movement detection.
 *
 * Architecture:
 *   Endpoint Agent (on each PC) → HTTP POST → RedNode CNS (:8787)
 *   - Reports every 5 minutes (configurable)
 *   - Minimal footprint: no NATS dependency, just HTTP
 *   - Works behind NAT/firewall as long as it can reach RedNode
 *
 * Install on endpoints:
 *   Linux:   curl -sL http://rednode:8787/endpoint/install.sh | bash
 *   Windows: iwr http://rednode:8787/endpoint/install.ps1 | iex
 *   macOS:   curl -sL http://rednode:8787/endpoint/install.sh | bash
 *
 * Or run manually:
 *   REDNODE_URL=http://10.0.50.10:8787 npx tsx agents/endpoint-agent/src/index.ts
 */

import { exec } from "child_process";
import { promisify } from "util";
import * as os from "os";
const execAsync = promisify(exec);

const CNS_URL =
  process.env.REDNODE_URL ||
  process.env.REDNODE_CNS ||
  "http://10.0.50.10:8787";
const API_TOKEN = process.env.REDNODE_API_TOKEN || "";
const REPORT_INTERVAL = parseInt(process.env.ENDPOINT_INTERVAL || "300000"); // 5 minutes
const HOSTNAME = os.hostname();
const PLATFORM = os.platform(); // 'linux', 'win32', 'darwin'

interface EndpointReport {
  hostname: string;
  platform: string;
  arch: string;
  uptime: number;
  ip_addresses: string[];
  mac_addresses: string[];
  packages: { name: string; version: string; manager: string }[];
  services: { name: string; status: string }[];
  open_ports: { port: number; process: string }[];
  users: string[];
  cpu: string;
  ram_total_mb: number;
  ram_used_mb: number;
  disk_used_pct: number;
  os_version: string;
  last_boot: string;
  report_ts: string;
}

// ─── Platform-Specific Collectors ───

async function getPackages(): Promise<
  { name: string; version: string; manager: string }[]
> {
  const pkgs: { name: string; version: string; manager: string }[] = [];

  if (PLATFORM === "linux") {
    // dpkg
    try {
      const { stdout } = await execAsync(
        "dpkg-query -W -f='${Package} ${Version}\\n' 2>/dev/null | head -500",
        { timeout: 10000 },
      );
      for (const line of stdout.trim().split("\n")) {
        const [name, version] = line.split(" ");
        if (name)
          pkgs.push({
            name: name.trim(),
            version: (version || "?").trim(),
            manager: "dpkg",
          });
      }
    } catch {}
    // rpm
    if (pkgs.length === 0) {
      try {
        const { stdout } = await execAsync(
          "rpm -qa --queryformat '%{NAME} %{VERSION}\\n' 2>/dev/null | head -500",
          { timeout: 10000 },
        );
        for (const line of stdout.trim().split("\n")) {
          const [name, version] = line.split(" ");
          if (name)
            pkgs.push({
              name: name.trim(),
              version: (version || "?").trim(),
              manager: "rpm",
            });
        }
      } catch {}
    }
    // snap
    try {
      const { stdout } = await execAsync(
        "snap list 2>/dev/null | tail -n +2 | head -100",
        { timeout: 10000 },
      );
      for (const line of stdout.trim().split("\n")) {
        const parts = line.trim().split(/\s+/);
        if (parts[0])
          pkgs.push({
            name: parts[0],
            version: parts[1] || "?",
            manager: "snap",
          });
      }
    } catch {}
    // flatpak
    try {
      const { stdout } = await execAsync(
        "flatpak list --columns=application,version 2>/dev/null | head -100",
        { timeout: 10000 },
      );
      for (const line of stdout.trim().split("\n")) {
        const parts = line.trim().split(/\s+/);
        if (parts[0])
          pkgs.push({
            name: parts[0],
            version: parts[1] || "?",
            manager: "flatpak",
          });
      }
    } catch {}
  }

  if (PLATFORM === "darwin") {
    // Homebrew
    try {
      const { stdout } = await execAsync(
        "brew list --versions 2>/dev/null | head -200",
        { timeout: 15000 },
      );
      for (const line of stdout.trim().split("\n")) {
        const parts = line.trim().split(/\s+/);
        if (parts[0])
          pkgs.push({
            name: parts[0],
            version: parts.slice(1).join(",") || "?",
            manager: "brew",
          });
      }
    } catch {}
  }

  if (PLATFORM === "win32") {
    // Windows — PowerShell
    try {
      const { stdout } = await execAsync(
        'powershell -Command "Get-Package | Select-Object Name,Version | ConvertTo-Json" 2>nul',
        { timeout: 30000 },
      );
      const data = JSON.parse(stdout);
      const arr = Array.isArray(data) ? data : [data];
      for (const p of arr) {
        if (p.Name)
          pkgs.push({
            name: p.Name,
            version: p.Version || "?",
            manager: "winget",
          });
      }
    } catch {}
    // winget
    try {
      const { stdout } = await execAsync(
        "winget list --disable-interactivity 2>nul",
        { timeout: 30000 },
      );
      for (const line of stdout.trim().split("\n").slice(2)) {
        const parts = line.trim().split(/\s{2,}/);
        if (parts[0] && parts[1])
          pkgs.push({ name: parts[0], version: parts[1], manager: "winget" });
      }
    } catch {}
  }

  return pkgs;
}

async function getOpenPorts(): Promise<{ port: number; process: string }[]> {
  const ports: { port: number; process: string }[] = [];

  try {
    if (PLATFORM === "linux") {
      const { stdout } = await execAsync("ss -tlnp 2>/dev/null | tail -n +2", {
        timeout: 5000,
      });
      for (const line of stdout.trim().split("\n")) {
        const match = line.match(/:(\d+)\s/);
        const proc = line.match(/users:\(\("([^"]+)"/);
        if (match)
          ports.push({ port: parseInt(match[1]), process: proc?.[1] || "?" });
      }
    } else if (PLATFORM === "darwin") {
      const { stdout } = await execAsync(
        "lsof -iTCP -sTCP:LISTEN -P -n 2>/dev/null | tail -n +2",
        { timeout: 5000 },
      );
      for (const line of stdout.trim().split("\n")) {
        const parts = line.trim().split(/\s+/);
        const match = parts[8]?.match(/:(\d+)$/);
        if (match)
          ports.push({ port: parseInt(match[1]), process: parts[0] || "?" });
      }
    } else if (PLATFORM === "win32") {
      const { stdout } = await execAsync(
        'netstat -ano -p TCP | findstr "LISTENING"',
        { timeout: 5000 },
      );
      for (const line of stdout.trim().split("\n")) {
        const match = line.match(/:(\d+)\s.*LISTENING\s+(\d+)/);
        if (match)
          ports.push({ port: parseInt(match[1]), process: `PID:${match[2]}` });
      }
    }
  } catch {}

  return ports;
}

async function getServices(): Promise<{ name: string; status: string }[]> {
  const services: { name: string; status: string }[] = [];

  try {
    if (PLATFORM === "linux") {
      const { stdout } = await execAsync(
        "systemctl list-units --type=service --state=running --no-pager --plain 2>/dev/null | head -50",
        { timeout: 10000 },
      );
      for (const line of stdout.trim().split("\n")) {
        const parts = line.trim().split(/\s+/);
        if (parts[0]?.endsWith(".service")) {
          services.push({
            name: parts[0].replace(".service", ""),
            status: "running",
          });
        }
      }
    } else if (PLATFORM === "darwin") {
      const { stdout } = await execAsync(
        "launchctl list 2>/dev/null | head -50",
        { timeout: 5000 },
      );
      for (const line of stdout.trim().split("\n").slice(1)) {
        const parts = line.trim().split(/\s+/);
        if (parts[2])
          services.push({
            name: parts[2],
            status: parts[0] === "-" ? "running" : `exit:${parts[0]}`,
          });
      }
    } else if (PLATFORM === "win32") {
      const { stdout } = await execAsync(
        "powershell -Command \"Get-Service | Where-Object {$_.Status -eq 'Running'} | Select-Object Name,Status | ConvertTo-Json\" 2>nul",
        { timeout: 10000 },
      );
      const data = JSON.parse(stdout);
      for (const s of Array.isArray(data) ? data : [data]) {
        if (s.Name) services.push({ name: s.Name, status: "running" });
      }
    }
  } catch {}

  return services;
}

async function getNetworkInfo(): Promise<{ ips: string[]; macs: string[] }> {
  const ips: string[] = [];
  const macs: string[] = [];

  const interfaces = os.networkInterfaces();
  for (const [name, addrs] of Object.entries(interfaces)) {
    for (const addr of addrs || []) {
      if (!addr.internal) {
        if (addr.family === "IPv4") ips.push(addr.address);
        if (addr.mac && addr.mac !== "00:00:00:00:00:00") macs.push(addr.mac);
      }
    }
  }

  return { ips: [...new Set(ips)], macs: [...new Set(macs)] };
}

async function getOsVersion(): Promise<string> {
  try {
    if (PLATFORM === "linux") {
      const { stdout } = await execAsync(
        "cat /etc/os-release 2>/dev/null | grep PRETTY_NAME | cut -d'\"' -f2",
        { timeout: 3000 },
      );
      return stdout.trim() || `Linux ${os.release()}`;
    } else if (PLATFORM === "darwin") {
      const { stdout } = await execAsync(
        "sw_vers -productVersion 2>/dev/null",
        { timeout: 3000 },
      );
      return `macOS ${stdout.trim()}`;
    } else if (PLATFORM === "win32") {
      const { stdout } = await execAsync("ver", { timeout: 3000 });
      return stdout.trim();
    }
  } catch {}
  return `${PLATFORM} ${os.release()}`;
}

// ─── Report Generation ───

async function generateReport(): Promise<EndpointReport> {
  const [packages, ports, services, network, osVersion] = await Promise.all([
    getPackages(),
    getOpenPorts(),
    getServices(),
    getNetworkInfo(),
    getOsVersion(),
  ]);

  const totalMem = Math.round(os.totalmem() / 1048576);
  const freeMem = Math.round(os.freemem() / 1048576);

  return {
    hostname: HOSTNAME,
    platform: PLATFORM,
    arch: os.arch(),
    uptime: Math.round(os.uptime()),
    ip_addresses: network.ips,
    mac_addresses: network.macs,
    packages,
    services,
    open_ports: ports,
    users: [], // populated below
    cpu: os.cpus()[0]?.model || "unknown",
    ram_total_mb: totalMem,
    ram_used_mb: totalMem - freeMem,
    disk_used_pct: 0, // populated below
    os_version: osVersion,
    last_boot: new Date(Date.now() - os.uptime() * 1000).toISOString(),
    report_ts: new Date().toISOString(),
  };
}

// ─── Report Submission ───

async function submitReport(report: EndpointReport): Promise<boolean> {
  try {
    const headers: Record<string, string> = {
      "Content-Type": "application/json",
    };
    if (API_TOKEN) headers["Authorization"] = `Bearer ${API_TOKEN}`;

    const resp = await fetch(`${CNS_URL}/security/events`, {
      method: "POST",
      headers,
      body: JSON.stringify({
        severity: "LOW",
        source: `endpoint-agent/${report.hostname}`,
        summary: `Endpoint report: ${report.hostname} (${report.platform}) — ${report.packages.length} packages, ${report.open_ports.length} ports, ${report.services.length} services`,
        raw: report,
      }),
    });

    if (!resp.ok) {
      console.error(`[endpoint] Report submission failed: ${resp.status}`);
      return false;
    }

    // Also ingest into memory for RAG searchability
    const pkgSummary = report.packages
      .slice(0, 50)
      .map((p) => `${p.name} ${p.version}`)
      .join(", ");
    const portSummary = report.open_ports
      .map((p) => `${p.port}/${p.process}`)
      .join(", ");

    await fetch(`${CNS_URL}/memory/ingest`, {
      method: "POST",
      headers,
      body: JSON.stringify({
        source: `endpoint/${report.hostname}`,
        content: `Endpoint ${report.hostname} (${report.os_version}): ${report.packages.length} packages (${pkgSummary}). Open ports: ${portSummary}. Services: ${report.services.length} running.`,
      }),
    });

    return true;
  } catch (e: any) {
    console.error(`[endpoint] Failed to reach RedNode: ${e.message}`);
    return false;
  }
}

// ─── Main Loop ───

async function main() {
  console.log(`[endpoint-agent] RedNode Endpoint Agent starting`);
  console.log(`  Hostname: ${HOSTNAME}`);
  console.log(`  Platform: ${PLATFORM} (${os.arch()})`);
  console.log(`  RedNode:  ${CNS_URL}`);
  console.log(`  Interval: ${REPORT_INTERVAL / 1000}s`);
  console.log();

  const report = async () => {
    console.log(`[endpoint] Collecting system info...`);
    const data = await generateReport();
    console.log(
      `  ${data.packages.length} packages, ${data.open_ports.length} ports, ${data.services.length} services`,
    );
    const ok = await submitReport(data);
    console.log(`  Report ${ok ? "✅ submitted" : "❌ failed"}`);
  };

  // First report immediately
  await report();

  // Then every interval
  setInterval(report, REPORT_INTERVAL);
}

main().catch((e) => {
  console.error("[endpoint-agent] Fatal:", e);
  process.exit(1);
});
