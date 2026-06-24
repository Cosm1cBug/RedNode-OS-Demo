import { RedNodeAgent } from "../../shared/src/agent.js";
import { sh, api, llm, cns, pihole, truenas, frigate, ha } from "../../shared/src/helpers.js";
import mqtt from "mqtt";

const FRIGATE_URL = process.env.FRIGATE_URL || "http://localhost:5000";
const MQTT_URL = process.env.MQTT_URL || "mqtt://127.0.0.1:1883";
const MQTT_USER = process.env.MQTT_USER || "rednode";
const MQTT_PASS = process.env.MQTT_PASS || "rednode-mqtt";
const CNS = process.env.REDNODE_CNS || "http://localhost:8787";

const TOOLS = [
  "cam.alert_config",
  "cam.anomaly",
  "cam.audio_detect",
  "cam.clip",
  "cam.events",
  "cam.face_identify",
  "cam.face_register",
  "cam.health_check",
  "cam.live_url",
  "cam.motion_zones",
  "cam.object_filter",
  "cam.person_detect",
  "cam.ptz_control",
  "cam.recording_export",
  "cam.recording_list",
  "cam.retain_event",
  "cam.review",
  "cam.search",
  "cam.snapshot",
  "cam.status",
  "cam.timelapse",
  "cam.vehicle_detect",
  "cam.zones",
  "presence.evaluate",
  "presence.history",
  "presence.status",
];

// ─── Frigate REST API ───

async function frigateGet(path: string): Promise<any> {
  const resp = await fetch(`${FRIGATE_URL}/api${path}`);
  if (!resp.ok)
    throw new Error(`Frigate API error: ${resp.status} ${resp.statusText}`);
  return resp.json();
}

// ─── Anomaly Detection ───

interface EventCounter {
  camera: string;
  label: string;
  hour: number;
  count: number;
}

const recentEvents: EventCounter[] = [];

function isAnomalous(camera: string, label: string): boolean {
  const hour = new Date().getHours();
  const isNight = hour >= 23 || hour < 6;

  // Person detection at night is always notable
  if (label === "person" && isNight) return true;

  // Count recent events for this camera+label
  const recent = recentEvents.filter(
    (e) =>
      e.camera === camera && e.label === label && Date.now() - e.hour < 3600000,
  ).length;

  // More than 10 events per hour for same camera+label is unusual
  return recent > 10;
}

// ─── MQTT Bridge — Frigate Events → RedNode Security Events ───

function startMqttBridge() {
  const mqttClient = mqtt.connect(MQTT_URL, {
    username: MQTT_USER,
    password: MQTT_PASS,
    reconnectPeriod: 5000,
  });

  mqttClient.on("connect", () => {
    console.log(
      "[surveillance-agent] MQTT connected — subscribing to frigate/events",
    );
    mqttClient.subscribe("frigate/events", (err) => {
      if (err) console.error("[surveillance-agent] MQTT subscribe error:", err);
    });
    // Also subscribe to individual camera availability
    mqttClient.subscribe("frigate/+/available", (err) => {
      if (err) console.error("[surveillance-agent] MQTT subscribe error:", err);
    });
  });

  mqttClient.on("message", async (topic, message) => {
    try {
      if (topic === "frigate/events") {
        const event = JSON.parse(message.toString());
        if (event.type === "new" && event.after) {
          const { camera, label, score, id, has_snapshot, has_clip } =
            event.after;
          const zone = event.after.entered_zones?.[0] || "unknown";

          console.log(
            `[surveillance] ${label} detected on ${camera} (zone: ${zone}, score: ${(score * 100).toFixed(0)}%)`,
          );

          // Track for anomaly detection
          recentEvents.push({ camera, label, hour: Date.now(), count: 1 });
          // Keep only last 1000
          if (recentEvents.length > 1000) recentEvents.shift();

          // Determine severity
          const anomalous = isAnomalous(camera, label);
          const severity = anomalous
            ? "CRITICAL"
            : label === "person"
              ? "MEDIUM"
              : "LOW";

          // Report to CNS as security event
          if (severity !== "LOW") {
            const summary = anomalous
              ? `⚠️ ANOMALOUS: ${label} detected on ${camera} (zone: ${zone}) at unusual time`
              : `${label} detected on ${camera} (zone: ${zone}, confidence: ${(score * 100).toFixed(0)}%)`;

            await fetch(`${CNS}/security/events`, {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                severity,
                source: `surveillance-agent/${camera}`,
                summary,
                raw: {
                  frigate_event_id: id,
                  camera,
                  label,
                  score,
                  zone,
                  has_snapshot,
                  has_clip,
                  snapshot_url: has_snapshot
                    ? `${FRIGATE_URL}/api/events/${id}/snapshot.jpg`
                    : null,
                  clip_url: has_clip
                    ? `${FRIGATE_URL}/api/events/${id}/clip.mp4`
                    : null,
                },
              }),
            });
          }
        }
      }

      // Camera availability
      if (topic.startsWith("frigate/") && topic.endsWith("/available")) {
        const camera = topic.split("/")[1];
        const available = message.toString();
        if (available === "offline") {
          console.warn(`[surveillance] Camera OFFLINE: ${camera}`);
          await fetch(`${CNS}/security/events`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              severity: "HIGH",
              source: `surveillance-agent/${camera}`,
              summary: `Camera '${camera}' went OFFLINE — possible tampering, power loss, or network issue`,
              raw: { camera, status: "offline" },
            }),
          });
        }
      }
    } catch (e: any) {
      console.error(
        "[surveillance-agent] MQTT message handler error:",
        e.message,
      );
    }
  });

  mqttClient.on("error", (err) => {
    console.error("[surveillance-agent] MQTT error:", err.message);
  });

  mqttClient.on("reconnect", () => {
    console.log("[surveillance-agent] MQTT reconnecting...");
  });
}

// ─── Agent ───

class SurveillanceAgent extends RedNodeAgent {
  constructor() {
    super("surveillance", TOOLS);
  }

  async handleTool(tool: string, args: any): Promise<any> {
    try {
      switch (tool) {
        case "cam.status": {
          const stats = await frigateGet("/stats");
          const cameras = Object.entries(stats.cameras || {}).map(
            ([name, data]: [string, any]) => ({
              name,
              fps: data.camera_fps,
              detection_fps: data.detection_fps,
              pid: data.pid,
            }),
          );
          const lines = cameras.map(
            (c) =>
              `${c.name}: ${c.fps > 0 ? "✅ Online" : "❌ Offline"} | ${c.fps} fps | Detection: ${c.detection_fps} fps`,
          );
          return {
            ok: true,
            output: lines.join("\n") || "No cameras configured",
            cameras,
          };
        }

        case "cam.events": {
          const limit = args.limit || 20;
          const label = args.label || "";
          const camera = args.camera || "";
          let url = `/events?limit=${limit}`;
          if (label) url += `&label=${label}`;
          if (camera) url += `&camera=${camera}`;
          const events = await frigateGet(url);
          const lines = (events || []).map((e: any) => {
            const time = new Date(e.start_time * 1000).toLocaleString();
            return `${time} | ${e.camera} | ${e.label} (${(e.top_score * 100).toFixed(0)}%) | Zone: ${e.zones?.join(", ") || "—"} | ${e.has_clip ? "📹 clip" : ""} ${e.has_snapshot ? "📸 snap" : ""}`;
          });
          return {
            ok: true,
            output: lines.join("\n") || "No events found",
            events,
          };
        }

        case "cam.snapshot": {
          const camera = args.camera;
          if (!camera) return { ok: false, error: "Missing 'camera' argument" };
          const url = `${FRIGATE_URL}/api/${camera}/latest.jpg`;
          return { ok: true, output: `Snapshot URL: ${url}`, url };
        }

        case "cam.clip": {
          const eventId = args.event_id || args.id;
          if (!eventId)
            return { ok: false, error: "Missing 'event_id' argument" };
          const clipUrl = `${FRIGATE_URL}/api/events/${eventId}/clip.mp4`;
          const snapUrl = `${FRIGATE_URL}/api/events/${eventId}/snapshot.jpg`;
          return {
            ok: true,
            output: `Clip: ${clipUrl}\nSnapshot: ${snapUrl}`,
            clip_url: clipUrl,
            snapshot_url: snapUrl,
          };
        }

        case "cam.search": {
          const label = args.label || "person";
          const camera = args.camera || "";
          const after = args.after || "";
          const before = args.before || "";
          let url = `/events?label=${label}&limit=20`;
          if (camera) url += `&camera=${camera}`;
          if (after) url += `&after=${new Date(after).getTime() / 1000}`;
          if (before) url += `&before=${new Date(before).getTime() / 1000}`;
          const events = await frigateGet(url);
          const lines = (events || []).map((e: any) => {
            const time = new Date(e.start_time * 1000).toLocaleString();
            return `${time} | ${e.camera} | ${e.label} (${(e.top_score * 100).toFixed(0)}%) | Zones: ${e.zones?.join(", ") || "—"}`;
          });
          return {
            ok: true,
            output: lines.join("\n") || `No ${label} events found`,
            count: events?.length || 0,
            events,
          };
        }

        case "cam.zones": {
          const config = await frigateGet("/config");
          const zones: any[] = [];
          for (const [camName, camConfig] of Object.entries(
            config.cameras || {},
          )) {
            const camZones = (camConfig as any).zones || {};
            for (const [zoneName, zoneConfig] of Object.entries(camZones)) {
              zones.push({
                camera: camName,
                zone: zoneName,
                objects: (zoneConfig as any).objects || [],
              });
            }
          }
          const lines = zones.map(
            (z) =>
              `${z.camera} → ${z.zone}: tracking [${z.objects.join(", ")}]`,
          );
          return {
            ok: true,
            output: lines.join("\n") || "No zones configured",
            zones,
          };
        }

        case "cam.person_detect": {
          // Quick shortcut: recent person detections across all cameras
          const events = await frigateGet("/events?label=person&limit=10");
          const lines = (events || []).map((e: any) => {
            const time = new Date(e.start_time * 1000).toLocaleString();
            return `${time} | ${e.camera} | confidence: ${(e.top_score * 100).toFixed(0)}% | ${e.has_snapshot ? `📸 ${FRIGATE_URL}/api/events/${e.id}/snapshot.jpg` : ""}`;
          });
          return {
            ok: true,
            output: lines.join("\n") || "No recent person detections",
            count: events?.length || 0,
          };
        }

        case "cam.anomaly": {
          const anomalies = recentEvents.filter((e) =>
            isAnomalous(e.camera, e.label),
          );
          if (anomalies.length === 0) {
            return {
              ok: true,
              output: "No anomalous camera activity detected ✅",
              anomalies: [],
            };
          }
          const lines = anomalies.map(
            (a) => `${a.camera}: ${a.label} — unusual pattern`,
          );
          return {
            ok: true,
            output: `${anomalies.length} anomalies:\n${lines.join("\n")}`,
            anomalies,
          };
        }

        case "cam.retain_event": {
          const eventId = args.event_id || args.id;
          if (!eventId)
            return { ok: false, error: "Missing 'event_id' argument" };
          await fetch(`${FRIGATE_URL}/api/events/${eventId}/retain`, {
            method: "POST",
          });
          return {
            ok: true,
            output: `Event ${eventId} marked for permanent retention`,
          };
        }

        case "cam.review": {
          // Frigate v0.17+ AI-generated review summaries
          try {
            const reviews = await frigateGet("/reviews?limit=5");
            const lines = (reviews || []).map((r: any) => {
              const time = new Date(r.start_time * 1000).toLocaleString();
              return `${time} | ${r.camera} | ${r.severity} | ${r.summary || "no summary"}`;
            });
            return {
              ok: true,
              output:
                lines.join("\n") ||
                "No reviews available (requires Frigate 0.17+)",
              reviews,
            };
          } catch {
            return {
              ok: true,
              output: "Review summaries not available — requires Frigate 0.17+",
            };
          }
        }
        case "presence.evaluate": {
          const camR = await frigate("/events?label=person&limit=5"); const netR = await sh("ip neigh show | grep -v FAILED | wc -l"); const people = Array.isArray(camR.data) ? camR.data.length : 0; const devices = parseInt(netR.output) || 0; const occupied = people > 0 || devices > 3; return { ok: true, output: `Presence: ${occupied ? "OCCUPIED" : "EMPTY"} (${people} people detected, ${devices} network devices)`, tool, occupied, people, devices }; combines camera + network data
        }

        case "presence.status": {
          const r = await cns("/presence/status"); return { ok: r.ok, output: r.output || "Presence tracking active", tool }; presence state machine
        }

        case "presence.history": {
          const r = await cns("/presence/history"); return { ok: r.ok, output: r.output || "No presence history yet", tool }; presence timeline from DB
        }

        case "cam.live_url": {
          const cam = args.camera || args.name || "";
                  const frigateUrl = process.env.FRIGATE_URL || "http://localhost:5000";
                  return { ok: true, output: \`RTSP: rtsp://\${cam}:554/stream1\nHTTP: \${frigateUrl}/api/\${cam}/latest.jpg\`, tool };
          }

        case "cam.recording_list": {
          const cam = args.camera || ""; const r = await frigate(cam ? `/recordings/${cam}` : "/recordings"); return { ok: r.ok, output: r.output, tool }; Frigate API recordings
        }

        case "cam.recording_export": {
          const cam = args.camera || ""; const r = await frigate(cam ? `/recordings/${cam}` : "/recordings"); return { ok: r.ok, output: r.output, tool };
        }

        case "cam.motion_zones": {
          const cam = args.camera || ""; const start = args.start || ""; const end = args.end || ""; if (!cam) return { ok: false, error: "Missing camera" }; const r = await frigate(`/${cam}/recordings/export?start=${start}&end=${end}`); return { ok: r.ok, output: r.output, tool };
        }

      case "cam.object_filter": {
        const r = await frigate("/config"); return { ok: r.ok, output: "Motion zones: " + r.output, tool };
      }

      case "cam.timelapse": {
        const r = await frigate("/config"); return { ok: r.ok, output: "Object filters: " + r.output, tool };
      }

      case "cam.face_register": {
        const cam = args.camera || ""; if (!cam) return { ok: false, error: "Missing camera" }; const r = await sh(`ffmpeg -framerate 30 -pattern_type glob -i "/media/frigate/recordings/${cam}/*.mp4" -c:v libx264 /tmp/timelapse-${cam}.mp4 2>&1 || echo "ffmpeg timelapse requires recordings directory"`, 120000); return { ok: r.ok, output: r.output, tool };
      }

      case "cam.face_identify": {
        return { ok: true, output: "Face registration requires CompreFace or InsightFace integration — configure FACE_API_URL in .env", tool }; local face matching
      }

      case "cam.vehicle_detect": {
        const r = await frigate("/events?label=car&limit=10"); return { ok: r.ok, output: r.output, tool }; Frigate event filter
      }

      case "cam.audio_detect": {
        const r = await frigate("/events?limit=10"); return { ok: r.ok, output: r.output, tool }; Frigate audio events
      }

      case "cam.ptz_control": {
        const r = await frigate("/events?label=person&limit=5"); return { ok: r.ok, output: "Face identification requires CompreFace integration. Recent person events:\n" + r.output, tool };
      }

      case "cam.health_check": {
        const frigateUrl = process.env.FRIGATE_URL || "http://localhost:5000";
                try {
                  const res = await fetch(\`\${frigateUrl}/api/stats\`);
                  const data = await res.json() as any;
                  const cams = Object.entries(data.cameras || {}).map(([n, c]: [string, any]) => \`\${n}: \${c.camera_fps > 0 ? "✅ online" : "❌ offline"} (fps: \${c.camera_fps})\`);
                  return { ok: true, output: \`Camera Health:\n\${cams.join("\n")}\`, tool };
                } catch (e: any) { return { ok: true, output: "Frigate not reachable", tool }; }
      }



        default:
          return null;
      }
    } catch (e: any) {
      console.error(`[surveillance-agent] ${tool} failed:`, e.message);
      return { ok: false, error: e.message };
    }
  }
}

// ─── Start ───

const agent = new SurveillanceAgent();
await agent.connect();

// Start MQTT bridge for real-time Frigate events
startMqttBridge();

await agent.serve();
