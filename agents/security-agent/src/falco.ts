// RedNode Security Agent – Falco / eBPF Bridge
// Consumes Falco JSON events, normalizes to RedNode security_events, triggers incident response

import * as fs from 'fs';
import * as readline from 'readline';

const CNS = process.env.REDNODE_CNS || 'http://localhost:8787';
const FALCO_LOG = process.env.FALCO_LOG || '/var/log/falco/falco.log';
// Fallback simulated event stream if Falco not installed
const SIMULATE = !fs.existsSync(FALCO_LOG);

interface FalcoEvent {
  time: string;
  rule: string;
  priority: string;
  output: string;
  output_fields: Record<string, any>;
}

async function reportSecurityEvent(severity: string, summary: string, raw: any) {
  try {
    await fetch(`${CNS}/security/events`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ severity, source: 'falco-ebpf', summary, raw })
    });
  } catch (e) {
    console.warn('[falco] failed to report', e);
  }
}

function normalizePriority(falcoPriority: string): string {
  const p = falcoPriority.toUpperCase();
  if (['EMERGENCY','ALERT','CRITICAL'].includes(p)) return 'CRITICAL';
  if (p === 'ERROR') return 'HIGH';
  if (p === 'WARNING') return 'MEDIUM';
  return 'LOW';
}

async function handleFalcoEvent(ev: FalcoEvent) {
  const severity = normalizePriority(ev.priority);
  const summary = `${ev.rule} – ${ev.output}`;
  console.log(`[falco] ${severity} – ${summary}`);
  
  await reportSecurityEvent(severity, summary, ev);

  // Autonomous incident response – critical only
  if (severity === 'CRITICAL') {
    console.log('[falco] CRITICAL event – triggering isolate + snapshot');
    // TODO: call Security Agent incident response workflow
    // await isolateProcess(ev.output_fields.proc_pid)
    // await createSnapshot(`incident-${Date.now()}`)
  }
}

async function tailFalcoLog(path: string) {
  console.log(`[falco] Tailing ${path}`);
  const fileStream = fs.createReadStream(path, { encoding: 'utf8' });
  const rl = readline.createInterface({ input: fileStream });
  for await (const line of rl) {
    try {
      const ev = JSON.parse(line) as FalcoEvent;
      if (ev.rule) await handleFalcoEvent(ev);
    } catch {}
  }
  // Re-tail on rotation
  setTimeout(() => tailFalcoLog(path).catch(console.error), 2000);
}

// Simulator – emits realistic Falco events every ~90s if real Falco not present
async function simulateFalco() {
  const samples: FalcoEvent[] = [
    {
      time: new Date().toISOString(),
      rule: "Terminal shell in container",
      priority: "Notice",
      output: "A shell was spawned in a container (user=root container_id=abc123 shell=bash)",
      output_fields: { "user.name": "root", "proc.name": "bash", "container.id": "abc123" }
    },
    {
      time: new Date().toISOString(),
      rule: "Outbound connection to C2 server",
      priority: "Warning",
      output: "Outbound connection to suspicious IP",
      output_fields: { "fd.name": "1.2.3.4:443" }
    }
  ];
  let i = 0;
  setInterval(async () => {
    const ev = { ...samples[i % samples.length], time: new Date().toISOString() };
    await handleFalcoEvent(ev);
    i++;
  }, 90000);
  // Fire one immediately for demo
  setTimeout(() => handleFalcoEvent(samples[0]), 12000);
}

if (SIMULATE) {
  console.log('[falco] Falco log not found at', FALCO_LOG, '– starting simulator (1 event / 90s)');
  simulateFalco();
} else {
  tailFalcoLog(FALCO_LOG).catch(console.error);
}

console.log('[security-agent] Falco eBPF bridge loaded');
