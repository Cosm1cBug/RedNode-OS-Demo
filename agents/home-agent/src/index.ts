import { RedNodeAgent } from "../../shared/src/agent.js";
import { sh, api, llm, cns, pihole, truenas, frigate, ha } from "../../shared/src/helpers.js";

const HA_URL = process.env.HOME_ASSISTANT_URL || "http://localhost:8123";
const HA_TOKEN = process.env.HOME_ASSISTANT_TOKEN || "";

const TOOLS = [
  "home.alarm",
  "home.automation",
  "home.battery_status",
  "home.climate",
  "home.device_info",
  "home.door_lock",
  "home.energy",
  "home.entities",
  "home.garage",
  "home.history",
  "home.irrigation",
  "home.lights",
  "home.logbook",
  "home.media_player",
  "home.notification",
  "home.scenes",
  "home.status",
  "home.switch",
  "home.vacuum",
];

async function haGet(path: string): Promise<any> {
  const resp = await fetch(`${HA_URL}/api${path}`, {
    headers: {
      Authorization: `Bearer ${HA_TOKEN}`,
      "Content-Type": "application/json",
    },
  });
  if (!resp.ok) throw new Error(`Home Assistant API: ${resp.status}`);
  return resp.json();
}

async function haPost(path: string, body: any): Promise<any> {
  const resp = await fetch(`${HA_URL}/api${path}`, {
    method: "POST",
    headers: {
      Authorization: `Bearer ${HA_TOKEN}`,
      "Content-Type": "application/json",
    },
    body: JSON.stringify(body),
  });
  return resp.ok ? resp.json().catch(() => ({})) : { error: `${resp.status}` };
}

async function callService(
  domain: string,
  service: string,
  entityId: string,
  data: any = {},
): Promise<any> {
  return haPost(`/services/${domain}/${service}`, {
    entity_id: entityId,
    ...data,
  });
}

class HomeAgent extends RedNodeAgent {
  constructor() {
    super("home", TOOLS);
  }

  async handleTool(tool: string, args: any): Promise<any> {
    if (!HA_TOKEN) {
      return {
        ok: false,
        error:
          "Home Assistant not configured. Set HOME_ASSISTANT_URL and HOME_ASSISTANT_TOKEN.",
      };
    }

    try {
      switch (tool) {
        case "home.status": {
          const states = await haGet("/states");
          // Group by domain
          const domains: Record<string, number> = {};
          for (const s of states) {
            const domain = (s.entity_id as string).split(".")[0];
            domains[domain] = (domains[domain] || 0) + 1;
          }
          const lines = Object.entries(domains)
            .sort((a, b) => b[1] - a[1])
            .map(([d, c]) => `  ${d}: ${c} entities`);
          return {
            ok: true,
            output: `Home Assistant — ${states.length} entities:\n${lines.join("\n")}`,
          };
        }

        case "home.entities": {
          const domain = args.domain || args.type || "light";
          const states = await haGet("/states");
          const filtered = states.filter((s: any) =>
            (s.entity_id as string).startsWith(`${domain}.`),
          );
          const lines = filtered.map(
            (s: any) =>
              `  ${s.attributes?.friendly_name || s.entity_id}: ${s.state}`,
          );
          return {
            ok: true,
            output: `${domain} entities (${filtered.length}):\n${lines.join("\n")}`,
          };
        }

        case "home.lights": {
          const action = args.action || "status"; // "on", "off", "toggle", "status"
          const entity = args.entity || args.light || "";

          if (action === "status" || !entity) {
            const states = await haGet("/states");
            const lights = states.filter((s: any) =>
              (s.entity_id as string).startsWith("light."),
            );
            const lines = lights.map((s: any) => {
              const name = s.attributes?.friendly_name || s.entity_id;
              const icon = s.state === "on" ? "💡" : "⚫";
              const brightness = s.attributes?.brightness
                ? ` (${Math.round((s.attributes.brightness / 255) * 100)}%)`
                : "";
              return `  ${icon} ${name}: ${s.state}${brightness}`;
            });
            return {
              ok: true,
              output: `Lights (${lights.length}):\n${lines.join("\n")}`,
            };
          }

          const entityId = entity.startsWith("light.")
            ? entity
            : `light.${entity}`;
          if (action === "on") {
            await callService("light", "turn_on", entityId, {
              brightness: args.brightness
                ? Math.round(args.brightness * 2.55)
                : undefined,
              color_temp: args.color_temp,
            });
            return { ok: true, output: `💡 ${entityId} turned ON` };
          }
          if (action === "off") {
            await callService("light", "turn_off", entityId);
            return { ok: true, output: `⚫ ${entityId} turned OFF` };
          }
          if (action === "toggle") {
            await callService("light", "toggle", entityId);
            return { ok: true, output: `🔄 ${entityId} toggled` };
          }
          return { ok: false, error: `Unknown action: ${action}` };
        }

        case "home.switch": {
          const entity = args.entity || "";
          const action = args.action || "toggle";
          if (!entity) {
            const states = await haGet("/states");
            const switches = states.filter((s: any) =>
              (s.entity_id as string).startsWith("switch."),
            );
            const lines = switches.map(
              (s: any) =>
                `  ${s.state === "on" ? "🟢" : "⚪"} ${s.attributes?.friendly_name || s.entity_id}: ${s.state}`,
            );
            return {
              ok: true,
              output: `Switches (${switches.length}):\n${lines.join("\n")}`,
            };
          }

          const entityId = entity.startsWith("switch.")
            ? entity
            : `switch.${entity}`;
          await callService(
            "switch",
            action === "on"
              ? "turn_on"
              : action === "off"
                ? "turn_off"
                : "toggle",
            entityId,
          );
          return { ok: true, output: `Switch ${entityId}: ${action}` };
        }

        case "home.climate": {
          const states = await haGet("/states");
          const climate = states.filter((s: any) =>
            (s.entity_id as string).startsWith("climate."),
          );
          if (climate.length === 0)
            return { ok: true, output: "No climate entities found" };
          const lines = climate.map((s: any) => {
            const name = s.attributes?.friendly_name || s.entity_id;
            const temp = s.attributes?.current_temperature;
            const target = s.attributes?.temperature;
            return `  🌡️ ${name}: ${s.state} ${temp ? `(current: ${temp}°)` : ""} ${target ? `(target: ${target}°)` : ""}`;
          });
          return { ok: true, output: `Climate:\n${lines.join("\n")}` };
        }

        case "home.scenes": {
          const action = args.action || "list";
          if (action === "list") {
            const states = await haGet("/states");
            const scenes = states.filter((s: any) =>
              (s.entity_id as string).startsWith("scene."),
            );
            const lines = scenes.map(
              (s: any) => `  🎬 ${s.attributes?.friendly_name || s.entity_id}`,
            );
            return {
              ok: true,
              output: `Scenes (${scenes.length}):\n${lines.join("\n")}`,
            };
          }
          if (action === "activate" && args.scene) {
            const entityId = args.scene.startsWith("scene.")
              ? args.scene
              : `scene.${args.scene}`;
            await callService("scene", "turn_on", entityId);
            return { ok: true, output: `🎬 Scene activated: ${entityId}` };
          }
          return {
            ok: false,
            error:
              "Usage: home.scenes {action: 'activate', scene: 'scene.movie_mode'}",
          };
        }

        case "home.automation": {
          const states = await haGet("/states");
          const autos = states.filter((s: any) =>
            (s.entity_id as string).startsWith("automation."),
          );
          const lines = autos.map(
            (s: any) =>
              `  ${s.state === "on" ? "✅" : "❌"} ${s.attributes?.friendly_name || s.entity_id}`,
          );
          return {
            ok: true,
            output: `Automations (${autos.length}):\n${lines.join("\n")}`,
          };
        }
      case "home.device_info": {
        const entity = args.entity || args.device || "";
                if (!entity) return { ok: false, error: "Missing 'entity' ID" };
                const haUrl = process.env.HOMEASSISTANT_URL || "http://localhost:8123";
                const haToken = process.env.HOMEASSISTANT_TOKEN || "";
                try {
                  const res = await fetch(\`\${haUrl}/api/states/\${entity}\`, { headers: { Authorization: \`Bearer \${haToken}\` } });
                  const data = await res.json();
                  return { ok: true, output: JSON.stringify(data, null, 2), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "home.history": {
        const entity = args.entity || ""; const r = await ha(`/history/period?filter_entity_id=${entity}&minimal_response`); return { ok: r.ok, output: r.output, tool }; HA API history
      }

      case "home.energy": {
        const r = await ha("/states"); const energy = Array.isArray(r.data) ? r.data.filter((s: any) => s.attributes?.device_class === "energy" || s.entity_id.includes("energy")) : []; return { ok: true, output: energy.map((e: any) => `${e.attributes?.friendly_name || e.entity_id}: ${e.state} ${e.attributes?.unit_of_measurement || ""}`).join("\n") || "No energy entities found", tool }; HA API energy dashboard
      }

      case "home.battery_status": {
        const haUrl = process.env.HOMEASSISTANT_URL || "http://localhost:8123";
                const haToken = process.env.HOMEASSISTANT_TOKEN || "";
                try {
                  const res = await fetch(\`\${haUrl}/api/states\`, { headers: { Authorization: \`Bearer \${haToken}\` } });
                  const states = await res.json() as any[];
                  const batteries = states.filter((s: any) => s.attributes?.device_class === "battery" || s.entity_id.includes("battery"));
                  const lines = batteries.map((b: any) => \`\${b.attributes?.friendly_name || b.entity_id}: \${b.state}%\`);
                  return { ok: true, output: lines.length ? lines.join("\n") : "No battery devices found", tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "home.door_lock": {
        const entity = args.entity || ""; const action = args.action || "lock"; if (!entity) return { ok: false, error: "Missing lock entity" }; const r = await ha(`/services/lock/${action}`, "POST", { entity_id: entity }); return { ok: r.ok, output: `Lock ${entity}: ${action}`, tool };
      }

      case "home.garage": {
        const entity = args.entity || ""; const action = args.action || "toggle"; if (!entity) return { ok: false, error: "Missing cover entity" }; const svc = action === "open" ? "open_cover" : action === "close" ? "close_cover" : "toggle"; const r = await ha(`/services/cover/${svc}`, "POST", { entity_id: entity }); return { ok: r.ok, output: `Garage ${entity}: ${action}`, tool };
      }

      case "home.vacuum": {
        const entity = args.entity || ""; const action = args.action || "start"; if (!entity) return { ok: false, error: "Missing vacuum entity" }; const r = await ha(`/services/vacuum/${action}`, "POST", { entity_id: entity }); return { ok: r.ok, output: `Vacuum ${entity}: ${action}`, tool };
      }

      case "home.irrigation": {
        const entity = args.entity || ""; const action = args.action || "turn_on"; if (!entity) return { ok: false, error: "Missing switch entity for irrigation" }; const r = await ha(`/services/switch/${action}`, "POST", { entity_id: entity }); return { ok: r.ok, output: `Irrigation ${entity}: ${action}`, tool };
      }

      case "home.alarm": {
        const entity = args.entity || ""; const action = args.action || "arm_home"; if (!entity) return { ok: false, error: "Missing alarm entity" }; const code = args.code || ""; const body: any = { entity_id: entity }; if (code) body.code = code; const r = await ha(`/services/alarm_control_panel/${action}`, "POST", body); return { ok: r.ok, output: `Alarm ${entity}: ${action}`, tool };
      }

      case "home.media_player": {
        const entity = args.entity || ""; const action = args.action || "media_play_pause"; if (!entity) return { ok: false, error: "Missing media_player entity" }; const r = await ha(`/services/media_player/${action}`, "POST", { entity_id: entity }); return { ok: r.ok, output: `Media player ${entity}: ${action}`, tool };
      }

      case "home.notification": {
        const message = args.message || ""; const title = args.title || "RedNode"; if (!message) return { ok: false, error: "Missing message" }; const r = await ha("/services/notify/notify", "POST", { message, title }); return { ok: r.ok, output: `Notification sent: ${title}`, tool }; HA notify service call
      }

      case "home.logbook": {
        const r = await ha("/logbook?period=1"); return { ok: r.ok, output: r.output, tool }; HA logbook API
      }



        default:
          return null;
      }
    } catch (e: any) {
      return { ok: false, error: e.message };
    }
  }
}

const agent = new HomeAgent();
await agent.connect();
await agent.serve();
