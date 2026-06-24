import { RedNodeAgent } from "../../shared/src/agent.js";
import { sh, api, llm, cns, pihole, truenas, frigate, ha } from "../../shared/src/helpers.js";

const JELLYFIN_URL = process.env.JELLYFIN_URL || "http://localhost:8096";
const JELLYFIN_API_KEY = process.env.JELLYFIN_API_KEY || "";

const TOOLS = [
  "media.library",
  "media.pause",
  "media.play",
  "media.playing",
  "media.recent",
  "media.search",
  "media.sessions",
  "music.lyrics",
  "music.playlist",
  "music.scan",
  "photo.duplicate",
  "photo.export",
  "photo.faces",
  "photo.ingest",
  "photo.resize",
  "photo.search",
  "photo.stats",
  "photo.tag",
  "video.convert",
  "video.info",
  "video.thumbnail",
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
      case "photo.ingest": {
        return { ok: true, output: "Media playback requires Jellyfin session — use the web interface or mobile app", tool }; file scan + metadata extraction
      }

      case "photo.search": {
        const dir = args.dir || process.env.PHOTO_WATCH_DIR || "/var/lib/rednode/photos"; const r = await sh(`find ${dir} -type f \( -name "*.jpg" -o -name "*.png" -o -name "*.heic" -o -name "*.raw" \) 2>/dev/null | wc -l`); return { ok: r.ok, output: `Found ${r.output.trim()} photos in ${dir}`, tool }; PostgreSQL + Qdrant search
      }

      case "photo.tag": {
        const query = args.query || ""; if (!query) return { ok: false, error: "Missing search query" }; const r = await cns(`/memory/query?type=photo&filter=${encodeURIComponent(query)}`); return { ok: r.ok, output: r.output, tool }; CLIP tagging (if available)
      }

      case "photo.stats": {
        return { ok: true, output: "Photo auto-tagging requires CLIP model — configure in .env (PHOTO_AUTO_TAG=true)", tool }; PostgreSQL photo stats
      }

      case "photo.faces": {
        const dir = process.env.PHOTO_LIBRARY_DIR || "/var/lib/rednode/photo-library"; const r = await sh(`du -sh ${dir} 2>/dev/null && find ${dir} -type f 2>/dev/null | wc -l`); return { ok: r.ok, output: `Photo library: ${r.output}`, tool }; face detection pipeline
      }

      case "photo.duplicate": {
        return { ok: true, output: "Face detection requires InsightFace or dlib — configure FACE_DETECTION_MODEL in .env", tool }; perceptual hash comparison
      }

      case "photo.resize": {
        const dir = args.dir || process.env.PHOTO_LIBRARY_DIR || "/var/lib/rednode/photo-library"; const r = await sh(`find ${dir} -type f -name "*.jpg" -exec md5sum {} \; 2>/dev/null | sort | uniq -w32 -d | head -20`, 30000); return { ok: r.ok, output: r.output || "No duplicates found (or directory empty)", tool }; ImageMagick batch resize
      }

      case "photo.export": {
        const dir = args.dir || ""; const size = args.size || "1920x1080"; if (!dir) return { ok: false, error: "Missing directory" }; const r = await sh(`find ${dir} -name "*.jpg" -exec convert {} -resize ${size} {} \; 2>&1 | tail -5 || echo "ImageMagick not installed"`, 60000); return { ok: r.ok, output: r.output, tool }; zip album export
      }

      case "music.scan": {
        const album = args.album || args.dir || ""; if (!album) return { ok: false, error: "Missing album/directory" }; const r = await sh(`zip -r /tmp/album-export.zip "${album}" 2>&1 | tail -3`, 60000); return { ok: r.ok, output: r.output, tool }; file scan + metadata extraction
      }

      case "music.playlist": {
        const dir = args.dir || "/var/lib/rednode/music"; const r = await sh(`find ${dir} -type f \( -name "*.mp3" -o -name "*.flac" -o -name "*.ogg" -o -name "*.m4a" \) 2>/dev/null | wc -l`); return { ok: r.ok, output: `Found ${r.output.trim()} music files in ${dir}`, tool }; playlist management
      }

      case "music.lyrics": {
        return { ok: true, output: "Playlist management through Jellyfin or Navidrome API — configure in .env", tool }; lyrics API lookup
      }

      case "video.info": {
        const file = args.file || args.path || "";
                if (!file) return { ok: false, error: "Missing 'file' path" };
                try {
                  const { execSync } = await import("child_process");
                  const out = execSync(\`ffprobe -v quiet -print_format json -show_format -show_streams "\${file}" 2>/dev/null || echo 'ffprobe not available'\`, { encoding: "utf-8", timeout: 10000 });
                  return { ok: true, output: out.trim(), tool };
                } catch (e: any) { return { ok: false, error: e.message }; }
      }

      case "video.thumbnail": {
        const file = args.file || ""; const time = args.time || "00:00:05"; if (!file) return { ok: false, error: "Missing video file" }; const out = args.output || "/tmp/thumbnail.jpg"; const r = await sh(`ffmpeg -ss ${time} -i "${file}" -vframes 1 -q:v 2 "${out}" 2>&1 && echo "Thumbnail: ${out}"`); return { ok: r.ok, output: r.output, tool }; ffmpeg thumbnail extraction
      }

      case "video.convert": {
        const file = args.file || ""; const format = args.format || "mp4"; if (!file) return { ok: false, error: "Missing video file" }; const out = file.replace(/\.[^.]+$/, `.${format}`); const r = await sh(`ffmpeg -i "${file}" -c:v libx264 -c:a aac "${out}" 2>&1 | tail -5`, 300000); return { ok: r.ok, output: r.output, tool }; ffmpeg conversion (medium risk)
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
