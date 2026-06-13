const CNS = process.env.NEXT_PUBLIC_CNS || 'http://localhost:8787';

export async function sendIntent(intent: string, session_id = 'web') {
  const r = await fetch(`${CNS}/intent`, { method: 'POST', headers: {'Content-Type':'application/json'}, body: JSON.stringify({ intent, session_id }) });
  return r.json();
}
export async function getApprovals() {
  const r = await fetch(`${CNS}/approvals`, { cache: 'no-store' });
  return r.json();
}
export async function approve(id: string, approved: boolean) {
  const r = await fetch(`${CNS}/approvals/${id}/approve`, { method: 'POST', headers: {'Content-Type':'application/json'}, body: JSON.stringify({ approved }) });
  return r.json();
}
export async function getMemory(q: string) {
  const r = await fetch(`${CNS}/memory/query?q=${encodeURIComponent(q)}`, { cache: 'no-store' });
  return r.json();
}
export async function getSecurityEvents() {
  const r = await fetch(`${CNS}/security/events`, { cache: 'no-store' });
  return r.json();
}
export async function ackSecurityEvent(id: string) {
  const r = await fetch(`${CNS}/security/events/${id}/ack`, { method: 'POST' });
  return r.json();
}
export async function getAudit(limit = 100) {
  const r = await fetch(`${CNS}/audit?limit=${limit}`, { cache: 'no-store' });
  return r.json();
}
export async function getAgents() {
  const r = await fetch(`${CNS}/agents/status`, { cache: 'no-store' });
  return r.json();
}

export async function getSentience() {
  const r = await fetch(`${CNS}/sentience`, { cache: 'no-store' });
  return r.json();
}

