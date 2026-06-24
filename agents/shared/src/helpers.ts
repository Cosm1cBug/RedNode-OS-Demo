/**
 * RedNode-OS — Shared Agent Helpers
 * 
 * Common utilities for all agents:
 *   - Shell command execution with timeout and error handling
 *   - API calls with auth and error handling
 *   - CNS/Ollama API helpers
 */

import { exec } from "child_process";
import { promisify } from "util";
const execAsync = promisify(exec);

const CNS = process.env.REDNODE_CNS || "http://localhost:8787";
const OLLAMA = process.env.OLLAMA_URL || "http://localhost:11434";
const MODEL = process.env.REDNODE_MODEL || "qwen2.5:7b-instruct-q4_K_M";

/** Run a shell command safely with timeout */
export async function sh(cmd: string, timeoutMs = 10000): Promise<{ ok: boolean; output: string }> {
  try {
    const { stdout, stderr } = await execAsync(cmd, { timeout: timeoutMs, maxBuffer: 1024 * 1024 });
    return { ok: true, output: (stdout || stderr || "").trim() };
  } catch (e: any) {
    const output = e.stdout?.trim() || e.stderr?.trim() || e.message || "Command failed";
    return { ok: false, output };
  }
}

/** Fetch a JSON API with optional auth headers */
export async function api(url: string, opts: { method?: string; headers?: Record<string, string>; body?: any; timeout?: number } = {}): Promise<{ ok: boolean; data: any; output: string }> {
  try {
    const controller = new AbortController();
    const timer = setTimeout(() => controller.abort(), opts.timeout || 10000);
    const res = await fetch(url, {
      method: opts.method || "GET",
      headers: { "Content-Type": "application/json", ...opts.headers },
      body: opts.body ? JSON.stringify(opts.body) : undefined,
      signal: controller.signal,
    });
    clearTimeout(timer);
    const text = await res.text();
    let data: any;
    try { data = JSON.parse(text); } catch { data = text; }
    return { ok: res.ok, data, output: typeof data === "string" ? data : JSON.stringify(data, null, 2) };
  } catch (e: any) {
    return { ok: false, data: null, output: `API error: ${e.message}` };
  }
}

/** Call Ollama LLM for text generation */
export async function llm(prompt: string, system?: string): Promise<string> {
  try {
    const res = await fetch(`${OLLAMA}/api/generate`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        model: MODEL,
        prompt,
        system: system || "You are RedNode-OS, a helpful AI assistant. Be concise.",
        stream: false,
        options: { temperature: 0.3, num_predict: 500 },
      }),
    });
    const data = await res.json() as any;
    return data.response || "No response from LLM";
  } catch (e: any) {
    return `LLM unavailable: ${e.message}`;
  }
}

/** Call CNS API endpoint */
export async function cns(path: string, opts: { method?: string; body?: any } = {}): Promise<any> {
  const token = process.env.REDNODE_API_TOKEN || "";
  return api(`${CNS}${path}`, {
    method: opts.method,
    headers: token ? { Authorization: `Bearer ${token}` } : {},
    body: opts.body,
  });
}

/** Pi-hole API helper */
export async function pihole(endpoint: string): Promise<{ ok: boolean; data: any; output: string }> {
  const url = process.env.PIHOLE_URL || "http://10.0.50.2";
  const pw = process.env.PIHOLE_PASSWORD || "";
  const authParam = pw ? `&auth=${pw}` : "";
  return api(`${url}/admin/api.php?${endpoint}${authParam}`);
}

/** TrueNAS API helper */
export async function truenas(path: string, method = "GET", body?: any): Promise<{ ok: boolean; data: any; output: string }> {
  const url = process.env.TRUENAS_URL || "https://10.0.50.3";
  const key = process.env.TRUENAS_API_KEY || "";
  return api(`${url}/api/v2.0${path}`, {
    method,
    headers: { Authorization: `Bearer ${key}` },
    body,
  });
}

/** Frigate API helper */
export async function frigate(path: string): Promise<{ ok: boolean; data: any; output: string }> {
  const url = process.env.FRIGATE_URL || "http://localhost:5000";
  return api(`${url}/api${path}`);
}

/** Home Assistant API helper */
export async function ha(path: string, method = "GET", body?: any): Promise<{ ok: boolean; data: any; output: string }> {
  const url = process.env.HOMEASSISTANT_URL || "http://localhost:8123";
  const token = process.env.HOMEASSISTANT_TOKEN || "";
  return api(`${url}/api${path}`, {
    method,
    headers: { Authorization: `Bearer ${token}` },
    body,
  });
}
