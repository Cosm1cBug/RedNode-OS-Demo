import { RedNodeAgent } from "../../shared/src/agent.js";

const JELLYFIN_URL = process.env.JELLYFIN_URL || "http://localhost:8096";
const JELLYFIN_API_KEY = process.env.JELLYFIN_API_KEY || "";

const TOOLS = [
  "media.search",
  "media.library",
  "media.playing",
  "media.play",
  "media.pause",
  "media.recent",
  "media.sessions",
];

async function jellyfinGet(path: string): Promise<any> {
  const resp = await fetch(`${JELLYFIN_URL}${path}`, {
    headers: {
      "X-Emby-Authorization": `MediaBrowser Token="${JELLYFIN_API_KEY}"`,
    },
  });
  if (!resp.ok) throw new Error(`Jellyfin API: ${resp.status}`);
  return resp.json();
}

async function jellyfinPost(path: string, body?: any): Promise<any> {
  const resp = await fetch(`${JELLYFIN_URL}${path}`, {
    method: "POST",
    headers: {
      "X-Emby-Authorization": `MediaBrowser Token="${JELLYFIN_API_KEY}"`,
      "Content-Type": "application/json",
    },
    body: body ? JSON.stringify(body) : undefined,
  });
  return resp.ok ? resp.json().catch(() => ({})) : { error: `${resp.status}` };
}

class MediaAgent extends RedNodeAgent {
  constructor() {
    super("media", TOOLS);
  }

  async handleTool(tool: string, args: any): Promise<any> {
    if (!JELLYFIN_API_KEY) {
      return {
        ok: false,
        error:
          "Jellyfin not configured. Set JELLYFIN_URL and JELLYFIN_API_KEY.",
      };
    }

    try {
      switch (tool) {
        case "media.search": {
          const query = args.query || args.q || "";
          if (!query) return { ok: false, error: "Missing 'query'" };
          const data = await jellyfinGet(
            `/Items?searchTerm=${encodeURIComponent(query)}&Limit=10&Recursive=true&api_key=${JELLYFIN_API_KEY}`,
          );
          const items = (data.Items || []).map(
            (i: any) =>
              `${i.Type}: ${i.Name}${i.ProductionYear ? ` (${i.ProductionYear})` : ""}${i.SeriesName ? ` — ${i.SeriesName}` : ""}`,
          );
          return {
            ok: true,
            output:
              items.length > 0
                ? items.join("\n")
                : `No results for: "${query}"`,
            count: items.length,
          };
        }

        case "media.library": {
          const data = await jellyfinGet(
            `/Library/VirtualFolders?api_key=${JELLYFIN_API_KEY}`,
          );
          const libs = (data || []).map(
            (l: any) =>
              `📁 ${l.Name} (${l.CollectionType || "mixed"}) — ${l.Locations?.join(", ") || "?"}`,
          );
          return {
            ok: true,
            output: libs.join("\n") || "No libraries configured",
          };
        }

        case "media.recent": {
          const data = await jellyfinGet(
            `/Items/Latest?Limit=10&api_key=${JELLYFIN_API_KEY}`,
          );
          const items = (data || []).map(
            (i: any) =>
              `${i.Type === "Episode" ? "📺" : i.Type === "Movie" ? "🎬" : "🎵"} ${i.Name}${i.SeriesName ? ` (${i.SeriesName})` : ""}`,
          );
          return {
            ok: true,
            output:
              items.length > 0
                ? `Recently added:\n${items.join("\n")}`
                : "No recent items",
          };
        }

        case "media.sessions": {
          const data = await jellyfinGet(
            `/Sessions?api_key=${JELLYFIN_API_KEY}`,
          );
          const active = (data || []).filter((s: any) => s.NowPlayingItem);
          if (active.length === 0)
            return { ok: true, output: "Nothing currently playing" };
          const lines = active.map((s: any) => {
            const item = s.NowPlayingItem;
            return `▶️ ${item.Name}${item.SeriesName ? ` (${item.SeriesName})` : ""} — on ${s.DeviceName || "unknown device"}`;
          });
          return { ok: true, output: lines.join("\n") };
        }

        case "media.playing": {
          // Same as sessions but friendlier name
          return this.handleTool("media.sessions", args);
        }

        case "media.play":
        case "media.pause": {
          // Play/pause requires a session ID and item ID — complex UPnP/DLNA
          return {
            ok: true,
            output: `Media playback control: use the Jellyfin app or web UI at ${JELLYFIN_URL}. Remote play/pause requires session management.`,
          };
        }

        default:
          return null;
      }
    } catch (e: any) {
      return { ok: false, error: e.message };
    }
  }
}

const agent = new MediaAgent();
await agent.connect();
await agent.serve();
