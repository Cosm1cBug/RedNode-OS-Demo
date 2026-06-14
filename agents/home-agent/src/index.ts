import { RedNodeAgent } from "../../shared/src/agent.js";

const HA_URL = process.env.HOME_ASSISTANT_URL || "http://localhost:8123";
const HA_TOKEN = process.env.HOME_ASSISTANT_TOKEN || "";

const TOOLS = [
  "home.lights", "home.switch", "home.status",
  "home.climate", "home.scenes", "home.entities",
  "home.automation",
];

async function haGet(path: string): Promise<any> {
  const resp = await fetch(`${HA_URL}/api${path}`, {
    headers: { Authorization: `Bearer ${HA_TOKEN}`, "Content-Type": "application/json" },
  });
  if (!resp.ok) throw new Error(`Home Assistant API: ${resp.status}`);
  return resp.json();
}

async function haPost(path: string, body: any): Promise<any> {
  const resp = await fetch(`${HA_URL}/api${path}`, {
    method: "POST",
    headers: { Authorization: `Bearer ${HA_TOKEN}`, "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  return resp.ok ? resp.json().catch(() => ({})) : { error: `${resp.status}` };
}

async function callService(domain: string, service: string, entityId: string, data: any = {}): Promise<any> {
  return haPost(`/services/${domain}/${service}`, { entity_id: entityId, ...data });
}

class HomeAgent extends RedNodeAgent {
  constructor() { super("home", TOOLS); }

  async handleTool(tool: string, args: any): Promise<any> {
    if (!HA_TOKEN) {
      return { ok: false, error: "Home Assistant not configured. Set HOME_ASSISTANT_URL and HOME_ASSISTANT_TOKEN." };
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
          return { ok: true, output: `Home Assistant — ${states.length} entities:\n${lines.join("\n")}` };
        }

        case "home.entities": {
          const domain = args.domain || args.type || "light";
          const states = await haGet("/states");
          const filtered = states.filter((s: any) => (s.entity_id as string).startsWith(`${domain}.`));
          const lines = filtered.map((s: any) =>
            `  ${s.attributes?.friendly_name || s.entity_id}: ${s.state}`
          );
          return { ok: true, output: `${domain} entities (${filtered.length}):\n${lines.join("\n")}` };
        }

        case "home.lights": {
          const action = args.action || "status"; // "on", "off", "toggle", "status"
          const entity = args.entity || args.light || "";

          if (action === "status" || !entity) {
            const states = await haGet("/states");
            const lights = states.filter((s: any) => (s.entity_id as string).startsWith("light."));
            const lines = lights.map((s: any) => {
              const name = s.attributes?.friendly_name || s.entity_id;
              const icon = s.state === "on" ? "💡" : "⚫";
              const brightness = s.attributes?.brightness
                ? ` (${Math.round(s.attributes.brightness / 255 * 100)}%)`
                : "";
              return `  ${icon} ${name}: ${s.state}${brightness}`;
            });
            return { ok: true, output: `Lights (${lights.length}):\n${lines.join("\n")}` };
          }

          const entityId = entity.startsWith("light.") ? entity : `light.${entity}`;
          if (action === "on") {
            await callService("light", "turn_on", entityId, {
              brightness: args.brightness ? Math.round(args.brightness * 2.55) : undefined,
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
            const switches = states.filter((s: any) => (s.entity_id as string).startsWith("switch."));
            const lines = switches.map((s: any) =>
              `  ${s.state === "on" ? "🟢" : "⚪"} ${s.attributes?.friendly_name || s.entity_id}: ${s.state}`
            );
            return { ok: true, output: `Switches (${switches.length}):\n${lines.join("\n")}` };
          }

          const entityId = entity.startsWith("switch.") ? entity : `switch.${entity}`;
          await callService("switch", action === "on" ? "turn_on" : action === "off" ? "turn_off" : "toggle", entityId);
          return { ok: true, output: `Switch ${entityId}: ${action}` };
        }

        case "home.climate": {
          const states = await haGet("/states");
          const climate = states.filter((s: any) => (s.entity_id as string).startsWith("climate."));
          if (climate.length === 0) return { ok: true, output: "No climate entities found" };
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
            const scenes = states.filter((s: any) => (s.entity_id as string).startsWith("scene."));
            const lines = scenes.map((s: any) =>
              `  🎬 ${s.attributes?.friendly_name || s.entity_id}`
            );
            return { ok: true, output: `Scenes (${scenes.length}):\n${lines.join("\n")}` };
          }
          if (action === "activate" && args.scene) {
            const entityId = args.scene.startsWith("scene.") ? args.scene : `scene.${args.scene}`;
            await callService("scene", "turn_on", entityId);
            return { ok: true, output: `🎬 Scene activated: ${entityId}` };
          }
          return { ok: false, error: "Usage: home.scenes {action: 'activate', scene: 'scene.movie_mode'}" };
        }

        case "home.automation": {
          const states = await haGet("/states");
          const autos = states.filter((s: any) => (s.entity_id as string).startsWith("automation."));
          const lines = autos.map((s: any) =>
            `  ${s.state === "on" ? "✅" : "❌"} ${s.attributes?.friendly_name || s.entity_id}`
          );
          return { ok: true, output: `Automations (${autos.length}):\n${lines.join("\n")}` };
        }

        default: return null;
      }
    } catch (e: any) {
      return { ok: false, error: e.message };
    }
  }
}

const agent = new HomeAgent();
await agent.connect();
await agent.serve();
