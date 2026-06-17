#!/usr/bin/env bash
# RedNode-OS — Endpoint Agent Installer (Linux/macOS)
# Run: curl -sL http://REDNODE_IP:8787/endpoint/install.sh | REDNODE_URL=http://REDNODE_IP:8787 bash
set -euo pipefail

REDNODE_URL="${REDNODE_URL:-http://10.0.50.10:8787}"
INSTALL_DIR="${HOME}/.rednode-endpoint"
SERVICE_NAME="rednode-endpoint"

echo "🧠 RedNode-OS — Endpoint Agent Installer"
echo "  RedNode: ${REDNODE_URL}"
echo ""

# Check Node.js
if ! command -v node >/dev/null 2>&1; then
  echo "Installing Node.js..."
  if command -v apt-get >/dev/null; then
    sudo apt-get update -qq && sudo apt-get install -y nodejs npm
  elif command -v brew >/dev/null; then
    brew install node
  elif command -v dnf >/dev/null; then
    sudo dnf install -y nodejs
  else
    echo "❌ Please install Node.js first: https://nodejs.org"
    exit 1
  fi
fi

# Install agent
mkdir -p "$INSTALL_DIR"
cat > "$INSTALL_DIR/agent.mjs" << 'AGENT_EOF'
// RedNode Endpoint Agent — Lightweight
import { exec } from "child_process";
import { promisify } from "util";
import * as os from "os";
const execAsync = promisify(exec);
const CNS = process.env.REDNODE_URL || "http://10.0.50.10:8787";
const TOKEN = process.env.REDNODE_API_TOKEN || "";
const INTERVAL = 300000;

async function run(cmd) {
  try { const { stdout } = await execAsync(cmd, { timeout: 10000 }); return stdout.trim(); } catch { return ""; }
}

async function report() {
  const pkgs = [];
  const dpkg = await run("dpkg-query -W -f='${Package} ${Version}\\n' 2>/dev/null | head -300");
  if (dpkg) for (const l of dpkg.split("\n")) { const [n,v] = l.split(" "); if(n) pkgs.push({name:n,version:v||"?",manager:"dpkg"}); }
  const brew = await run("brew list --versions 2>/dev/null | head -200");
  if (brew) for (const l of brew.split("\n")) { const p = l.split(/\s+/); if(p[0]) pkgs.push({name:p[0],version:p[1]||"?",manager:"brew"}); }

  const ports = [];
  const ss = await run("ss -tlnp 2>/dev/null | tail -n +2");
  if (ss) for (const l of ss.split("\n")) { const m = l.match(/:(\d+)\s/); if(m) ports.push({port:+m[1],process:"?"}); }

  const headers = {"Content-Type":"application/json"};
  if (TOKEN) headers["Authorization"] = `Bearer ${TOKEN}`;

  const data = {
    hostname: os.hostname(), platform: os.platform(), arch: os.arch(),
    ip_addresses: Object.values(os.networkInterfaces()).flat().filter(a=>a&&!a.internal&&a.family==="IPv4").map(a=>a.address),
    packages: pkgs, open_ports: ports, services: [],
    ram_total_mb: Math.round(os.totalmem()/1048576),
    ram_used_mb: Math.round((os.totalmem()-os.freemem())/1048576),
    os_version: `${os.platform()} ${os.release()}`,
    report_ts: new Date().toISOString(),
  };

  try {
    await fetch(`${CNS}/security/events`, { method:"POST", headers, body: JSON.stringify({
      severity:"LOW", source:`endpoint-agent/${data.hostname}`,
      summary:`Endpoint: ${data.hostname} — ${pkgs.length} pkgs, ${ports.length} ports`,
      raw: data
    })});
    await fetch(`${CNS}/memory/ingest`, { method:"POST", headers, body: JSON.stringify({
      source:`endpoint/${data.hostname}`,
      content:`Endpoint ${data.hostname}: ${pkgs.length} packages, ports: ${ports.map(p=>p.port).join(",")}`
    })});
    console.log(`[endpoint] ${new Date().toISOString()} — reported ${pkgs.length} pkgs, ${ports.length} ports`);
  } catch(e) { console.error("[endpoint] report failed:", e.message); }
}

report();
setInterval(report, INTERVAL);
console.log(`[rednode-endpoint] Running — ${os.hostname()} → ${CNS} — interval ${INTERVAL/1000}s`);
AGENT_EOF

# Create systemd service (Linux only)
if [ "$(uname)" = "Linux" ] && command -v systemctl >/dev/null; then
  sudo tee /etc/systemd/system/${SERVICE_NAME}.service > /dev/null << EOF
[Unit]
Description=RedNode Endpoint Agent
After=network.target

[Service]
Type=simple
ExecStart=$(which node) ${INSTALL_DIR}/agent.mjs
Restart=always
RestartSec=30
Environment=REDNODE_URL=${REDNODE_URL}
Environment=REDNODE_API_TOKEN=${REDNODE_API_TOKEN:-}

[Install]
WantedBy=multi-user.target
EOF
  sudo systemctl daemon-reload
  sudo systemctl enable ${SERVICE_NAME}
  sudo systemctl start ${SERVICE_NAME}
  echo "✅ Installed as systemd service: ${SERVICE_NAME}"
  echo "   Status: sudo systemctl status ${SERVICE_NAME}"
else
  echo "✅ Installed at: ${INSTALL_DIR}/agent.mjs"
  echo "   Run: REDNODE_URL=${REDNODE_URL} node ${INSTALL_DIR}/agent.mjs"
fi

echo ""
echo "🧠 Endpoint agent reporting to RedNode every 5 minutes."
