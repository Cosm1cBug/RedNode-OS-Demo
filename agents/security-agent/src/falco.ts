// RedNode Security Agent – Falco / eBPF Bridge
// Consumes real Falco JSON events, normalizes to RedNode security_events
// Falls back to auditd/journalctl monitoring if Falco is not installed

import * as fs from "fs";
import * as readline from "readline";
import { exec } from "child_process";
import { promisify } from "util";
const execAsync = promisify(exec);

const CNS = process.env.REDNODE_CNS || "http://localhost:8787";
const FALCO_LOG = process.env.FALCO_LOG || "/var/log/falco/falco.log";
const FALCO_INSTALLED =
  fs.existsSync(FALCO_LOG) || fs.existsSync("/usr/bin/falco");
const JOURNALCTL_POLL_INTERVAL = 60000; // 1 minute

interface FalcoEvent {
  time: string;
  rule: string;
  priority: string;
  output: string;
  output_fields: Record<string, any>;
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
      body: JSON.stringify({ severity, source: "falco-ebpf", summary, raw }),
    });
  } catch (e: any) {
    console.warn("[falco] Failed to report:", e.message);
  }
}

function normalizePriority(falcoPriority: string): string {
  const p = falcoPriority.toUpperCase();
  if (["EMERGENCY", "ALERT", "CRITICAL"].includes(p)) return "CRITICAL";
  if (p === "ERROR") return "HIGH";
  if (p === "WARNING") return "MEDIUM";
  return "LOW";
}

// ─── Handle Falco Event ───

async function handleFalcoEvent(ev: FalcoEvent) {
  const severity = normalizePriority(ev.priority);
  const summary = `${ev.rule} — ${ev.output}`;
  console.log(`[falco] ${severity} — ${summary}`);

  await reportSecurityEvent(severity, summary, ev);

  // Autonomous incident response — critical events
  if (severity === "CRITICAL") {
    console.log("[falco] CRITICAL event — triggering incident response");
    // Request CNS to run security triage
    try {
      await fetch(`${CNS}/intent`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
          intent: `Critical security event detected: ${ev.rule}. Run security triage and check system logs.`,
          session_id: "falco-incident",
        }),
      });
    } catch {}
  }
}

// ─── Tail Real Falco Log ───

async function tailFalcoLog(path: string) {
  console.log(`[falco] Tailing real Falco log: ${path}`);

  // Start from end of file (don't replay old events)
  const stat = fs.statSync(path);
  let position = stat.size;

  const checkForNew = () => {
    try {
      const currentStat = fs.statSync(path);
      if (currentStat.size > position) {
        const stream = fs.createReadStream(path, {
          encoding: "utf8",
          start: position,
        });
        const rl = readline.createInterface({ input: stream });
        rl.on("line", async (line) => {
          try {
            const ev = JSON.parse(line) as FalcoEvent;
            if (ev.rule) await handleFalcoEvent(ev);
          } catch {} // skip non-JSON lines
        });
        rl.on("close", () => {
          position = currentStat.size;
        });
      } else if (currentStat.size < position) {
        // Log was rotated
        position = 0;
      }
    } catch (e: any) {
      // File might have been rotated or deleted
      if (!fs.existsSync(path)) {
        console.warn(
          `[falco] Log file disappeared: ${path} — waiting for recreation`,
        );
      }
    }
  };

  // Poll every 2 seconds
  setInterval(checkForNew, 2000);
}

// ─── Fallback: Monitor journalctl for security-relevant events ───

async function monitorJournalctl() {
  console.log(
    "[falco] Falco not installed — falling back to journalctl security monitoring",
  );

  let lastCheck = new Date(Date.now() - JOURNALCTL_POLL_INTERVAL);

  const checkLogs = async () => {
    const since = lastCheck.toISOString();
    lastCheck = new Date();

    try {
      // Check for authentication failures
      const { stdout: authFails } = await execAsync(
        `journalctl -p err --since "${since}" --no-pager -o json 2>/dev/null | head -20`,
        { timeout: 10000 },
      );

      if (authFails.trim()) {
        const lines = authFails.trim().split("\n");
        let sshFailCount = 0;
        let otherErrors: string[] = [];

        for (const line of lines) {
          try {
            const entry = JSON.parse(line);
            const msg = entry.MESSAGE || "";
            const unit =
              entry._SYSTEMD_UNIT || entry.SYSLOG_IDENTIFIER || "system";

            if (
              msg.includes("Failed password") ||
              msg.includes("authentication failure")
            ) {
              sshFailCount++;
            } else if (
              msg.includes("segfault") ||
              msg.includes("oom-kill") ||
              msg.includes("Out of memory")
            ) {
              await reportSecurityEvent(
                "HIGH",
                `System error: ${msg.substring(0, 200)}`,
                {
                  source: "journalctl",
                  unit,
                  message: msg,
                },
              );
            } else {
              otherErrors.push(`[${unit}] ${msg.substring(0, 100)}`);
            }
          } catch {} // skip unparseable lines
        }

        // Report SSH brute force attempts
        if (sshFailCount >= 3) {
          await reportSecurityEvent(
            sshFailCount >= 10 ? "CRITICAL" : "HIGH",
            `${sshFailCount} failed authentication attempts detected in last ${JOURNALCTL_POLL_INTERVAL / 1000}s — possible brute force`,
            { count: sshFailCount, source: "journalctl" },
          );
        }

        // Report other errors if there are many
        if (otherErrors.length >= 5) {
          await reportSecurityEvent(
            "MEDIUM",
            `${otherErrors.length} system errors in last minute`,
            {
              errors: otherErrors.slice(0, 10),
              source: "journalctl",
            },
          );
        }
      }

      // Check for kernel security messages
      const { stdout: kernelMsgs } = await execAsync(
        `journalctl -k --since "${since}" --no-pager -o short 2>/dev/null | grep -i "segfault\\|oops\\|panic\\|apparmor.*DENIED\\|seccomp" | head -5`,
        { timeout: 10000 },
      );
      if (kernelMsgs.trim()) {
        for (const line of kernelMsgs.trim().split("\n")) {
          const severity =
            line.includes("panic") || line.includes("oops")
              ? "CRITICAL"
              : "MEDIUM";
          await reportSecurityEvent(
            severity,
            `Kernel security event: ${line.substring(0, 200)}`,
            {
              source: "kernel",
              raw: line,
            },
          );
        }
      }
    } catch (e: any) {
      // journalctl not available — silently skip
    }
  };

  // Initial check
  await checkLogs();
  // Then poll every minute
  setInterval(checkLogs, JOURNALCTL_POLL_INTERVAL);
}

// ─── Start ───

if (FALCO_INSTALLED && fs.existsSync(FALCO_LOG)) {
  tailFalcoLog(FALCO_LOG);
  console.log(
    "[security-agent] Falco eBPF bridge loaded — real-time threat detection active",
  );
} else {
  monitorJournalctl();
  console.log(
    "[security-agent] Falco not found — using journalctl fallback (install Falco for real eBPF monitoring)",
  );
}
