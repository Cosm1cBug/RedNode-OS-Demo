# RedNode-OS — Endpoint Agent Installer (Windows)
# Run: iwr http://REDNODE_IP:8787/endpoint/install.ps1 | iex
# Or: Set-ExecutionPolicy Bypass -Scope Process; .\endpoint-install-windows.ps1

$RedNodeUrl = if ($env:REDNODE_URL) { $env:REDNODE_URL } else { "http://10.0.50.10:8787" }
$InstallDir = "$env:APPDATA\RedNode-Endpoint"
$TaskName = "RedNodeEndpoint"

Write-Host "🧠 RedNode-OS — Endpoint Agent Installer (Windows)"
Write-Host "  RedNode: $RedNodeUrl"
Write-Host ""

# Create install directory
New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null

# Create the agent script
@'
// RedNode Endpoint Agent — Windows
const os = require("os");
const https = require("https");
const http = require("http");
const { execSync } = require("child_process");

const CNS = process.env.REDNODE_URL || "http://10.0.50.10:8787";
const TOKEN = process.env.REDNODE_API_TOKEN || "";
const INTERVAL = 300000;

function run(cmd) {
  try { return execSync(cmd, { timeout: 15000, encoding: "utf-8" }).trim(); } catch { return ""; }
}

function report() {
  const pkgs = [];
  try {
    const out = run('powershell -Command "Get-Package | Select-Object Name,Version | ConvertTo-Csv -NoTypeInformation"');
    for (const line of out.split("\n").slice(1)) {
      const m = line.match(/"([^"]+)","([^"]+)"/);
      if (m) pkgs.push({ name: m[1], version: m[2], manager: "windows" });
    }
  } catch {}

  const ports = [];
  try {
    const out = run('netstat -ano -p TCP | findstr "LISTENING"');
    for (const line of out.split("\n")) {
      const m = line.match(/:(\d+)\s.*LISTENING\s+(\d+)/);
      if (m) ports.push({ port: parseInt(m[1]), process: `PID:${m[2]}` });
    }
  } catch {}

  const data = {
    hostname: os.hostname(), platform: "win32", arch: os.arch(),
    ip_addresses: Object.values(os.networkInterfaces()).flat().filter(a => a && !a.internal && a.family === "IPv4").map(a => a.address),
    packages: pkgs, open_ports: ports,
    ram_total_mb: Math.round(os.totalmem() / 1048576),
    ram_used_mb: Math.round((os.totalmem() - os.freemem()) / 1048576),
    os_version: `Windows ${os.release()}`,
    report_ts: new Date().toISOString(),
  };

  const body = JSON.stringify({
    severity: "LOW",
    source: `endpoint-agent/${data.hostname}`,
    summary: `Endpoint: ${data.hostname} — ${pkgs.length} pkgs, ${ports.length} ports`,
    raw: data,
  });

  const url = new URL(`${CNS}/security/events`);
  const mod = url.protocol === "https:" ? https : http;
  const req = mod.request({ hostname: url.hostname, port: url.port, path: url.pathname, method: "POST",
    headers: { "Content-Type": "application/json", ...(TOKEN ? { Authorization: `Bearer ${TOKEN}` } : {}) }
  }, (res) => { console.log(`[endpoint] ${new Date().toISOString()} — ${pkgs.length} pkgs, ${ports.length} ports — ${res.statusCode}`); });
  req.on("error", (e) => console.error("[endpoint]", e.message));
  req.write(body);
  req.end();
}

report();
setInterval(report, INTERVAL);
console.log(`[rednode-endpoint] Running — ${os.hostname()} → ${CNS}`);
'@ | Out-File -FilePath "$InstallDir\agent.js" -Encoding UTF8

# Create scheduled task
$Action = New-ScheduledTaskAction -Execute "node.exe" -Argument "$InstallDir\agent.js"
$Trigger = New-ScheduledTaskTrigger -AtStartup
$Settings = New-ScheduledTaskSettingsSet -RestartCount 3 -RestartInterval (New-TimeSpan -Minutes 1)

Register-ScheduledTask -TaskName $TaskName -Action $Action -Trigger $Trigger -Settings $Settings -Description "RedNode Endpoint Agent" -RunLevel Limited -Force | Out-Null

# Also start it now
Start-ScheduledTask -TaskName $TaskName

Write-Host "✅ Installed as scheduled task: $TaskName"
Write-Host "   Script: $InstallDir\agent.js"
Write-Host "   Status: Get-ScheduledTask -TaskName $TaskName"
Write-Host ""
Write-Host "🧠 Endpoint agent reporting to RedNode every 5 minutes."
