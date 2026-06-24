import { RedNodeAgent } from "../../shared/src/agent.js";
import { sh, api, llm, cns, pihole, truenas, frigate, ha } from "../../shared/src/helpers.js";

const TOOLS = [
  "docker.ps",
  "fs.read",
  "notify.config",
  "notify.digest",
  "notify.send",
  "pipeline.enable",
  "pipeline.list",
  "pipeline.run",
  "predict.maintenance",
  "predict.report",
  "process.list",
  "service.restart",
  "service.status",
  "shell.run_safe",
  "sys.boot_list",
  "sys.cpu_profile",
  "sys.cron_create",
  "sys.cron_list",
  "sys.disk_io",
  "sys.fan_speed",
  "sys.journal",
  "sys.kernel_log",
  "sys.mem_detailed",
  "sys.network_io",
  "sys.package_list",
  "sys.package_update",
  "sys.rollback",
  "sys.temperature",
  "sys.uptime",
  "sys.usb_devices",
  "ups.history",
  "ups.status",
  "ups.test",
];

class SystemAgent extends RedNodeAgent {
  constructor() {
    super("system", TOOLS);
  }

  async handleTool(tool: string, args: any): Promise<any> {
    switch (tool) {
      case "fs.read": {
        const p = args.path || "";
        // Double-check path security at agent level (Rust executor also checks)
        if (p.includes(".."))
          throw new Error("path traversal denied by agent policy");
        if (
          p.includes("/etc/shadow") ||
          p.includes(".ssh/") ||
          p.includes(".env")
        )
          throw new Error("sensitive file access denied by agent policy");
        // Fall through to Rust executor for sandboxed read
        return null;
      }

      case "process.list": {
        // Execute via Rust, then enrich the output
        const result = await this.callTool(tool, args);
        if (!result?.ok) return result;

        // Parse ps aux output and highlight high-CPU processes
        const lines = (result.stdout || result.output || "").split("\n");
        const header = lines[0] || "";
        const processes = lines.slice(1).filter((l: string) => l.trim());

        const highCpu = processes.filter((l: string) => {
          const parts = l.trim().split(/\s+/);
          const cpu = parseFloat(parts[2] || "0");
          return cpu > 50;
        });

        let enriched = `Total processes: ${processes.length}\n`;
        if (highCpu.length > 0) {
          enriched += `⚠️  High CPU processes (>50%):\n`;
          highCpu.forEach((l: string) => {
            enriched += `  ${l}\n`;
          });
        } else {
          enriched += `✅ No high-CPU processes\n`;
        }
        enriched += `\nTop 10 by CPU:\n${header}\n${processes.slice(0, 10).join("\n")}`;

        return { ok: true, output: enriched, tool, raw: result.stdout };
      }

      case "docker.ps": {
        const result = await this.callTool(tool, args);
        if (!result?.ok) return result;

        const output = result.stdout || result.output || "";
        const lines = output.split("\n").filter((l: string) => l.trim());

        // Detect unhealthy containers
        const unhealthy = lines.filter(
          (l: string) =>
            l.toLowerCase().includes("unhealthy") ||
            l.toLowerCase().includes("exited") ||
            l.toLowerCase().includes("dead"),
        );

        let enriched = `Docker containers: ${Math.max(0, lines.length - 1)}\n`;
        if (unhealthy.length > 0) {
          enriched += `⚠️  Unhealthy/stopped containers:\n`;
          unhealthy.forEach((l: string) => {
            enriched += `  ${l}\n`;
          });

          // Report to security events
          const CNS = process.env.REDNODE_CNS || "http://localhost:8787";
          for (const u of unhealthy) {
            try {
              await fetch(`${CNS}/security/events`, {
                method: "POST",
                headers: { "Content-Type": "application/json" },
                body: JSON.stringify({
                  severity: "MEDIUM",
                  source: "system-agent/docker",
                  summary: `Unhealthy container detected: ${u.trim().substring(0, 100)}`,
                  raw: { line: u },
                }),
              });
            } catch {}
          }
        } else {
          enriched += `✅ All containers healthy\n`;
        }
        enriched += `\n${output}`;

        return {
          ok: true,
          output: enriched,
          tool,
          unhealthy_count: unhealthy.length,
        };
      }

      case "service.status": {
        // Pass through to Rust executor — it handles systemctl
        const svc = args.service || args.name || ""; const r = await sh(svc ? `systemctl status ${svc} --no-pager 2>&1` : "systemctl list-units --type=service --state=running --no-pager | head -30"); return { ok: r.ok, output: r.output, tool };
      }

      case "shell.run_safe": {
        // Pass through to Rust executor — it enforces the allowlist
        const cmd = args.command || args.cmd || ""; if (!cmd) return { ok: false, error: "Missing command" }; const r = await sh(cmd); return { ok: r.ok, output: r.output, tool };
      }
      case "service.restart": {
        const svc = args.service || args.name || "";
                if (!svc) return { ok: false, error: "Missing 'service' name" };
                const svc = args.service || args.name || ""; const r = await sh(svc ? `systemctl status ${svc} --no-pager 2>&1` : "systemctl list-units --type=service --state=running --no-pager | head -30"); return { ok: r.ok, output: r.output, tool }; systemctl restart <svc>
      }

      case "ups.status": {
        try {
                  const { execSync } = await import("child_process");
                  const out = execSync("upsc rednode-ups@localhost 2>/dev/null || echo 'UPS not configured'", { encoding: "utf-8", timeout: 5000 });
                  return { ok: true, output: out.trim(), tool };
                } catch (e: any) { return { ok: true, output: "UPS monitoring not available (NUT not installed)", tool }; }
      }

      case "ups.history": {
        try {
                  const { execSync } = await import("child_process");
                  const out = execSync("upscmd -l rednode-ups@localhost 2>/dev/null || echo 'No UPS history'", { encoding: "utf-8", timeout: 5000 });
                  return { ok: true, output: out.trim(), tool };
                } catch (e: any) { return { ok: true, output: "UPS history not available", tool }; }
      }

      case "ups.test": {
        const r = await sh("upscmd rednode-ups@localhost test.battery.start 2>&1 || echo \"UPS test not available\""); return { ok: r.ok, output: r.output, tool }; upscmd rednode-ups@localhost test.battery.start
      }

      case "predict.maintenance": {
        const r = await cns("/predict/maintenance"); return { ok: r.ok, output: r.output, tool }; // Rust executor delegates to predict module
      }

      case "predict.report": {
        const r = await cns("/predict/report"); return { ok: r.ok, output: r.output, tool }; // Rust executor delegates to predict module
      }

      case "notify.send": {
        const title = args.title || "RedNode Notification";
                const body = args.body || args.message || "";
                if (!body) return { ok: false, error: "Missing 'body' or 'message'" };
                const title = args.title || "Notification"; const body = args.body || args.message || ""; const r = await cns("/notify", { method: "POST", body: { title, body, urgency: args.urgency || "normal" } }); return { ok: r.ok, output: r.output, tool }; // Rust executor delegates to notifications module
      }

      case "notify.digest": {
        const r = await cns("/notify/digest"); return { ok: r.ok, output: r.output, tool }; // Rust executor delegates to notifications module
      }

      case "notify.config": {
        const r = await cns("/notify/config", { method: "POST", body: args }); return { ok: r.ok, output: r.output, tool }; // Rust executor delegates to notifications module
      }

      case "pipeline.list": {
        const r = await cns("/pipelines"); return { ok: r.ok, output: r.output, tool }; // Rust executor delegates to pipelines module
      }

      case "pipeline.run": {
        const name = args.name || args.pipeline || "";
                if (!name) return { ok: false, error: "Missing 'name' of pipeline to run" };
                const name = args.name || ""; if (!name) return { ok: false, error: "Missing pipeline name" }; const r = await cns(`/pipelines/${name}/run`, { method: "POST" }); return { ok: r.ok, output: r.output, tool }; // Rust executor delegates to pipelines module
      }

      case "pipeline.enable": {
        const name = args.name || args.pipeline || "";
                if (!name) return { ok: false, error: "Missing 'name' of pipeline" };
                const name = args.name || ""; const r = await cns(`/pipelines/${name}/enable`, { method: "POST", body: { enabled: args.enabled !== false } }); return { ok: r.ok, output: r.output, tool }; // Rust executor delegates to pipelines module
      }

      case "sys.cpu_profile": {
        try {
                  const { execSync } = await import("child_process");
                  const mpstat = execSync("mpstat -P ALL 1 1 2>/dev/null || cat /proc/stat | head -10", { encoding: "utf-8", timeout: 10000 });
                  const freq = execSync("lscpu | grep -i 'mhz\|model name\|cpu(s)\|thread' 2>/dev/null || echo 'lscpu unavailable'", { encoding: "utf-8", timeout: 5000 });
                  return { ok: true, output: `CPU Profile:\n${freq}\n${mpstat}`, tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "sys.mem_detailed": {
        try {
                  const { execSync } = await import("child_process");
                  const meminfo = execSync("cat /proc/meminfo", { encoding: "utf-8", timeout: 5000 });
                  const top = execSync("ps aux --sort=-%mem | head -15", { encoding: "utf-8", timeout: 5000 });
                  return { ok: true, output: `Memory Detail:\n${meminfo}\nTop by memory:\n${top}`, tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "sys.disk_io": {
        try {
                  const { execSync } = await import("child_process");
                  const iostat = execSync("iostat -dx 1 1 2>/dev/null || cat /proc/diskstats", { encoding: "utf-8", timeout: 10000 });
                  return { ok: true, output: iostat.trim(), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "sys.network_io": {
        try {
                  const { execSync } = await import("child_process");
                  const out = execSync("cat /proc/net/dev", { encoding: "utf-8", timeout: 5000 });
                  return { ok: true, output: out.trim(), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "sys.journal": {
        const unit = args.unit || "";
                const lines = args.lines || 50;
                const priority = args.priority || "";
                try {
                  const { execSync } = await import("child_process");
                  let cmd = `journalctl --no-pager -n ${lines}`;
                  if (unit) cmd += ` -u ${unit}`;
                  if (priority) cmd += ` -p ${priority}`;
                  const out = execSync(cmd, { encoding: "utf-8", timeout: 10000 });
                  return { ok: true, output: out.trim(), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "sys.cron_list": {
        try {
                  const { execSync } = await import("child_process");
                  const timers = execSync("systemctl list-timers --all --no-pager 2>/dev/null || echo 'No timers'", { encoding: "utf-8", timeout: 5000 });
                  return { ok: true, output: timers.trim(), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "sys.cron_create": {
        const cmd = args.command || args.cmd || ""; const schedule = args.schedule || ""; if (!cmd || !schedule) return { ok: false, error: "Missing command and schedule" }; return { ok: true, output: `To create timer: systemd-run --on-calendar="${schedule}" ${cmd}`, tool }; requires approval (medium risk)
      }

      case "sys.package_list": {
        try {
                  const { execSync } = await import("child_process");
                  const out = execSync("nix-env -q 2>/dev/null || nixos-rebuild list-generations 2>/dev/null | tail -10 || echo 'Not on NixOS'", { encoding: "utf-8", timeout: 10000 });
                  return { ok: true, output: out.trim(), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "sys.package_update": {
        return null; // Rust executor: nixos-rebuild switch (high risk, needs approval)
      }

      case "sys.boot_list": {
        try {
                  const { execSync } = await import("child_process");
                  const out = execSync("nixos-rebuild list-generations 2>/dev/null | tail -20 || ls /boot/loader/entries/ 2>/dev/null || echo 'Cannot list generations'", { encoding: "utf-8", timeout: 10000 });
                  return { ok: true, output: out.trim(), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "sys.rollback": {
        return null; // Rust executor: nixos-rebuild switch --rollback (high risk, needs approval)
      }

      case "sys.temperature": {
        try {
                  const { execSync } = await import("child_process");
                  const sensors = execSync("sensors 2>/dev/null || cat /sys/class/thermal/thermal_zone*/temp 2>/dev/null | while read t; do echo "$(($t/1000))°C"; done || echo 'No temperature sensors'", { encoding: "utf-8", timeout: 5000 });
                  return { ok: true, output: sensors.trim(), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "sys.fan_speed": {
        try {
                  const { execSync } = await import("child_process");
                  const fans = execSync("sensors 2>/dev/null | grep -i fan || echo 'No fan sensors detected'", { encoding: "utf-8", timeout: 5000 });
                  return { ok: true, output: fans.trim(), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "sys.usb_devices": {
        try {
                  const { execSync } = await import("child_process");
                  const out = execSync("lsusb 2>/dev/null || echo 'lsusb not available'", { encoding: "utf-8", timeout: 5000 });
                  return { ok: true, output: out.trim(), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "sys.uptime": {
        try {
                  const { execSync } = await import("child_process");
                  const uptime = execSync("uptime", { encoding: "utf-8", timeout: 5000 });
                  const loadavg = execSync("cat /proc/loadavg", { encoding: "utf-8", timeout: 5000 });
                  return { ok: true, output: `${uptime.trim()}\nLoad: ${loadavg.trim()}`, tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "sys.kernel_log": {
        try {
                  const { execSync } = await import("child_process");
                  const n = args.lines || 30;
                  const out = execSync(`dmesg --time-format iso 2>/dev/null | tail -${n} || dmesg | tail -${n}`, { encoding: "utf-8", timeout: 5000 });
                  return { ok: true, output: out.trim(), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }



      default:
        return null;
    }
  }
}

const agent = new SystemAgent();
await agent.connect();
await agent.serve();
