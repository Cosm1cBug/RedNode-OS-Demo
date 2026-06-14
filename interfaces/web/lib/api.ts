const CNS = process.env.NEXT_PUBLIC_CNS || 'http://localhost:8787';

async function f(path: string, opts?: RequestInit) {
  const r = await fetch(`${CNS}${path}`, { cache: 'no-store', ...opts });
  return r.json();
}

// ─── Core ───
export const sendIntent = (intent: string, session_id = 'web') =>
  f('/intent', { method: 'POST', headers: {'Content-Type':'application/json'}, body: JSON.stringify({ intent, session_id }) });

export const getHealth = () => f('/health');
export const getSentience = () => f('/sentience');
export const getAgents = () => f('/agents/status');

// ─── Security ───
export const getSecurityEvents = () => f('/security/events');
export const ackSecurityEvent = (id: string) =>
  f(`/security/events/${id}/ack`, { method: 'POST' });

// ─── Approvals ───
export const getApprovals = () => f('/approvals');
export const approve = (id: string, approved: boolean) =>
  f(`/approvals/${id}/approve`, { method: 'POST', headers: {'Content-Type':'application/json'}, body: JSON.stringify({ approved }) });

// ─── Memory ───
export const getMemory = (q: string) => f(`/memory/query?q=${encodeURIComponent(q)}`);

// ─── Audit ───
export const getAudit = (limit = 100) => f(`/audit?limit=${limit}`);

// ─── Infrastructure (Pi-hole) — via intent routing ───
export const getPiholeStats = () =>
  sendIntent('show pihole stats', 'dashboard');

// ─── Storage (TrueNAS) — via intent routing ───
export const getNasHealth = () =>
  sendIntent('check TrueNAS pool health', 'dashboard');
export const getNasUsage = () =>
  sendIntent('show TrueNAS storage usage', 'dashboard');

// ─── Cameras (Frigate) — via intent routing ───
export const getCameraStatus = () =>
  sendIntent('show camera status', 'dashboard');
export const getCameraEvents = () =>
  sendIntent('show recent camera person detections', 'dashboard');

// ─── Email (Comms) — via intent routing ───
export const getEmailSummary = () =>
  sendIntent('summarize my recent emails', 'dashboard');
export const getNotificationDigest = () =>
  sendIntent('show notification digest', 'dashboard');

// ─── Workflows — via intent routing ───
export const runWorkflow = (name: string) =>
  sendIntent(`run workflow ${name}`, 'dashboard');
export const listWorkflows = () =>
  sendIntent('list available workflows', 'dashboard');
