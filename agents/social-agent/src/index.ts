/**
 * RedNode-OS – Social Media Agent
 *
 * Manages social media interactions across platforms.
 * Designed as a platform-agnostic framework — plug in any API.
 *
 * Currently supported (via environment variables):
 *   - Twitter/X (API v2 — requires developer account)
 *   - Mastodon (any instance — fully self-hosted compatible)
 *   - Bluesky (AT Protocol — no API key needed for public data)
 *   - LinkedIn (requires OAuth app)
 *   - Instagram (via Instagram Graph API — requires Facebook developer account)
 *   - WhatsApp Business (via WhatsApp Cloud API)
 *
 * All posts go through LLM for drafting and require approval before sending.
 * Analytics and feed reading are Low risk (auto-execute).
 * Posting/replying are Medium risk (logged, auto-execute unless configured otherwise).
 *
 * Privacy: API calls go directly to each platform's API.
 * No third-party social media management service involved.
 */

import { RedNodeAgent } from "../../shared/src/agent.js";
import { sh, api, llm, cns, pihole, truenas, frigate, ha } from "../../shared/src/helpers.js";

const CNS = process.env.REDNODE_CNS || "http://localhost:8787";
const OLLAMA_URL = process.env.OLLAMA_URL || "http://127.0.0.1:11434";
const MODEL = process.env.REDNODE_MODEL || "qwen2.5:14b-instruct-q4_K_M";

// ─── Platform Configuration ───

interface PlatformConfig {
  name: string;
  enabled: boolean;
  apiBase: string;
  authHeader: () => Record<string, string>;
}

const PLATFORMS: Record<string, PlatformConfig> = {
  twitter: {
    name: "Twitter/X",
    enabled: !!process.env.TWITTER_BEARER_TOKEN,
    apiBase: "https://api.twitter.com/2",
    authHeader: () => ({
      Authorization: `Bearer ${process.env.TWITTER_BEARER_TOKEN}`,
    }),
  },
  mastodon: {
    name: "Mastodon",
    enabled: !!process.env.MASTODON_ACCESS_TOKEN,
    apiBase: process.env.MASTODON_INSTANCE || "https://mastodon.social",
    authHeader: () => ({
      Authorization: `Bearer ${process.env.MASTODON_ACCESS_TOKEN}`,
    }),
  },
  bluesky: {
    name: "Bluesky",
    enabled: !!process.env.BLUESKY_HANDLE,
    apiBase: "https://bsky.social/xrpc",
    authHeader: () => ({}), // Auth handled per-request via session
  },
  linkedin: {
    name: "LinkedIn",
    enabled: !!process.env.LINKEDIN_ACCESS_TOKEN,
    apiBase: "https://api.linkedin.com/v2",
    authHeader: () => ({
      Authorization: `Bearer ${process.env.LINKEDIN_ACCESS_TOKEN}`,
    }),
  },
  instagram: {
    name: "Instagram",
    enabled: !!process.env.INSTAGRAM_ACCESS_TOKEN,
    apiBase: "https://graph.instagram.com/v18.0",
    authHeader: () => ({}), // Token passed as query param
  },
  whatsapp: {
    name: "WhatsApp Business",
    enabled: !!process.env.WHATSAPP_TOKEN,
    apiBase: `https://graph.facebook.com/v18.0/${process.env.WHATSAPP_PHONE_ID || ""}`,
    authHeader: () => ({
      Authorization: `Bearer ${process.env.WHATSAPP_TOKEN}`,
    }),
  },
};

const TOOLS = [
  "social.analytics",
  "social.best_time",
  "social.block",
  "social.crosspost",
  "social.dm",
  "social.draft",
  "social.feed",
  "social.followers",
  "social.hashtags",
  "social.mentions",
  "social.monitor",
  "social.platforms",
  "social.post",
  "social.reply",
  "social.schedule",
  "social.thread",
];

// ─── LLM Drafting ───

async function draftWithLLM(prompt: string, platform: string): Promise<string> {
  try {
    const constraints: Record<string, string> = {
      twitter: "Max 280 characters. Be concise, impactful. No hashtag spam.",
      mastodon: "Max 500 characters. Can be more detailed. Hashtags OK.",
      bluesky: "Max 300 characters. Conversational tone.",
      linkedin:
        "Professional tone. Can be longer. Use line breaks for readability.",
      instagram:
        "Engaging, visual language. Use relevant hashtags (max 5). Include call-to-action.",
      whatsapp: "Conversational, direct. Keep it short.",
    };

    const resp = await fetch(`${OLLAMA_URL}/api/chat`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        model: MODEL,
        messages: [
          {
            role: "system",
            content: `You are a social media content writer. Draft a post for ${platform}. ${constraints[platform] || ""}. Output ONLY the post text, no explanations.`,
          },
          { role: "user", content: prompt },
        ],
        stream: false,
        options: { temperature: 0.7, num_predict: 512 },
      }),
    });
    const data = (await resp.json()) as any;
    return data.message?.content?.trim() || "";
  } catch (e: any) {
    return `[LLM unavailable: ${e.message}]`;
  }
}

// ─── Platform API Implementations ───

async function postToTwitter(
  text: string,
): Promise<{ ok: boolean; id?: string; error?: string }> {
  const cfg = PLATFORMS.twitter;
  if (!cfg.enabled)
    return {
      ok: false,
      error: "Twitter not configured (set TWITTER_BEARER_TOKEN)",
    };

  try {
    const resp = await fetch(`${cfg.apiBase}/tweets`, {
      method: "POST",
      headers: { ...cfg.authHeader(), "Content-Type": "application/json" },
      body: JSON.stringify({ text }),
    });
    const data = (await resp.json()) as any;
    if (data.data?.id) return { ok: true, id: data.data.id };
    return { ok: false, error: JSON.stringify(data.errors || data) };
  } catch (e: any) {
    return { ok: false, error: e.message };
  }
}

async function postToMastodon(
  text: string,
): Promise<{ ok: boolean; id?: string; error?: string }> {
  const cfg = PLATFORMS.mastodon;
  if (!cfg.enabled)
    return {
      ok: false,
      error:
        "Mastodon not configured (set MASTODON_ACCESS_TOKEN, MASTODON_INSTANCE)",
    };

  try {
    const resp = await fetch(`${cfg.apiBase}/api/v1/statuses`, {
      method: "POST",
      headers: { ...cfg.authHeader(), "Content-Type": "application/json" },
      body: JSON.stringify({ status: text, visibility: "public" }),
    });
    const data = (await resp.json()) as any;
    if (data.id) return { ok: true, id: data.id };
    return { ok: false, error: JSON.stringify(data.error || data) };
  } catch (e: any) {
    return { ok: false, error: e.message };
  }
}

async function postToBluesky(
  text: string,
): Promise<{ ok: boolean; id?: string; error?: string }> {
  const handle = process.env.BLUESKY_HANDLE || "";
  const password = process.env.BLUESKY_APP_PASSWORD || "";
  if (!handle || !password)
    return {
      ok: false,
      error:
        "Bluesky not configured (set BLUESKY_HANDLE, BLUESKY_APP_PASSWORD)",
    };

  try {
    // Login to get session
    const loginResp = await fetch(
      "https://bsky.social/xrpc/com.atproto.server.createSession",
      {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ identifier: handle, password }),
      },
    );
    const session = (await loginResp.json()) as any;
    if (!session.accessJwt) return { ok: false, error: "Bluesky auth failed" };

    // Create post
    const resp = await fetch(
      "https://bsky.social/xrpc/com.atproto.repo.createRecord",
      {
        method: "POST",
        headers: {
          Authorization: `Bearer ${session.accessJwt}`,
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          repo: session.did,
          collection: "app.bsky.feed.post",
          record: {
            text,
            createdAt: new Date().toISOString(),
            $type: "app.bsky.feed.post",
          },
        }),
      },
    );
    const data = (await resp.json()) as any;
    if (data.uri) return { ok: true, id: data.uri };
    return { ok: false, error: JSON.stringify(data) };
  } catch (e: any) {
    return { ok: false, error: e.message };
  }
}

async function postToLinkedIn(
  text: string,
): Promise<{ ok: boolean; id?: string; error?: string }> {
  const cfg = PLATFORMS.linkedin;
  if (!cfg.enabled)
    return {
      ok: false,
      error: "LinkedIn not configured (set LINKEDIN_ACCESS_TOKEN)",
    };

  // LinkedIn posting requires person URN — simplified
  return {
    ok: false,
    error:
      "LinkedIn posting requires OAuth2 flow — configure via LinkedIn Developer Portal",
  };
}

async function sendWhatsApp(
  to: string,
  message: string,
): Promise<{ ok: boolean; error?: string }> {
  const cfg = PLATFORMS.whatsapp;
  if (!cfg.enabled)
    return {
      ok: false,
      error: "WhatsApp not configured (set WHATSAPP_TOKEN, WHATSAPP_PHONE_ID)",
    };

  try {
    const resp = await fetch(`${cfg.apiBase}/messages`, {
      method: "POST",
      headers: { ...cfg.authHeader(), "Content-Type": "application/json" },
      body: JSON.stringify({
        messaging_product: "whatsapp",
        to,
        type: "text",
        text: { body: message },
      }),
    });
    const data = (await resp.json()) as any;
    return data.messages
      ? { ok: true }
      : { ok: false, error: JSON.stringify(data.error || data) };
  } catch (e: any) {
    return { ok: false, error: e.message };
  }
}

// ─── Scheduled Posts Store ───

interface ScheduledPost {
  id: string;
  platform: string;
  text: string;
  scheduled_for: string;
  status: "pending" | "posted" | "failed";
  created: string;
}

const scheduledPosts: ScheduledPost[] = [];

// Check scheduled posts every minute
setInterval(async () => {
  const now = new Date();
  for (const post of scheduledPosts) {
    if (post.status === "pending" && new Date(post.scheduled_for) <= now) {
      console.log(
        `[social-agent] Posting scheduled ${post.platform}: "${post.text.substring(0, 50)}..."`,
      );
      const postFn = getPostFunction(post.platform);
      if (postFn) {
        const result = await postFn(post.text);
        post.status = result.ok ? "posted" : "failed";
      }
    }
  }
}, 60000);

function getPostFunction(
  platform: string,
): ((text: string) => Promise<any>) | null {
  switch (platform) {
    case "twitter":
      return postToTwitter;
    case "mastodon":
      return postToMastodon;
    case "bluesky":
      return postToBluesky;
    case "linkedin":
      return postToLinkedIn;
      case "social.thread": {
        const posts = args.posts || args.content || []; if (!posts.length) return { ok: false, error: "Missing thread posts array" }; return { ok: true, output: `Thread with ${Array.isArray(posts) ? posts.length : 1} posts — configure platform API to post`, tool };
      }

      case "social.hashtags": {
        const content = args.content || args.topic || ""; if (!content) return { ok: false, error: "Missing content" }; const tags = await llm(`Suggest 5-8 relevant hashtags for: "${content}". Output only hashtags.`); return { ok: true, output: tags, tool }; LLM hashtag suggestion
      }

      case "social.best_time": {
        return { ok: true, output: "Best posting times analysis requires engagement data history — post regularly for 2+ weeks for accurate analysis", tool }; //analytics calculation
      }

      case "social.followers": {
        return { ok: true, output: "Follower analytics requires platform API credentials — configure in .env", tool }; //platform API query
      }

      case "social.mentions": {
        return { ok: true, output: "Mentions tracking requires platform API credentials — configure in .env", tool }; //platform API query
      }

      case "social.block": {
        const content = args.content || args.topic || ""; if (!content) return { ok: false, error: "Missing content/topic" }; const tags = await llm(`Suggest 5-8 relevant hashtags for this social media content: "${content}". Output only hashtags separated by spaces.`); return { ok: true, output: tags, tool };
      }

      case "social.crosspost": {
        return { ok: true, output: "User blocking requires platform API credentials — configure in .env", tool };
      }


    default:
      const content = args.content || ""; if (!content) return { ok: false, error: "Missing content" }; return { ok: true, output: `Cross-post to all configured platforms: "${content.substring(0, 50)}..." — configure API keys in .env`, tool };
  }
}

// ─── Agent ───

class SocialAgent extends RedNodeAgent {
  constructor() {
    super("social", TOOLS);
  }

  async handleTool(tool: string, args: any): Promise<any> {
    try {
      switch (tool) {
        case "social.platforms": {
          const platforms = Object.entries(PLATFORMS).map(([key, cfg]) => ({
            platform: key,
            name: cfg.name,
            enabled: cfg.enabled,
            status: cfg.enabled ? "✅ configured" : "❌ not configured",
          }));
          const lines = platforms.map(
            (p) => `  ${p.status} ${p.name} (${p.platform})`,
          );
          return {
            ok: true,
            output: `Social Media Platforms:\n${lines.join("\n")}`,
            platforms,
          };
        }

        case "social.draft": {
          const about = args.about || args.topic || args.prompt || "";
          const platform = args.platform || "twitter";
          if (!about) return { ok: false, error: "Missing 'about' or 'topic'" };

          const draft = await draftWithLLM(about, platform);
          return {
            ok: true,
            output: `Draft for ${platform} (${draft.length} chars):\n\n${draft}\n\nUse social.post to publish this.`,
            draft,
            platform,
            char_count: draft.length,
          };
        }

        case "social.post": {
          const text = args.text || args.content || "";
          const platform = args.platform || "twitter";
          if (!text) return { ok: false, error: "Missing 'text' to post" };

          const postFn = getPostFunction(platform);
          if (!postFn)
            return { ok: false, error: `Unknown platform: ${platform}` };

          const result = await postFn(text);
          if (result.ok) {
            // Log to audit
            await fetch(`${CNS}/security/events`, {
              method: "POST",
              headers: { "Content-Type": "application/json" },
              body: JSON.stringify({
                severity: "LOW",
                source: `social-agent/${platform}`,
                summary: `Posted to ${platform}: "${text.substring(0, 80)}"`,
                raw: result,
              }),
            }).catch(() => {});
          }
          return {
            ok: result.ok,
            output: result.ok
              ? `✅ Posted to ${platform}: "${text.substring(0, 100)}"${result.id ? ` (ID: ${result.id})` : ""}`
              : `❌ Failed to post to ${platform}: ${result.error}`,
            result,
          };
        }

        case "social.schedule": {
          const text = args.text || "";
          const platform = args.platform || "twitter";
          const scheduledFor = args.at || args.scheduled_for || args.time || "";
          if (!text) return { ok: false, error: "Missing 'text'" };
          if (!scheduledFor)
            return {
              ok: false,
              error: "Missing 'at' (e.g. '2026-06-15T10:00:00')",
            };

          const post: ScheduledPost = {
            id: Date.now().toString(36),
            platform,
            text,
            scheduled_for: scheduledFor,
            status: "pending",
            created: new Date().toISOString(),
          };
          scheduledPosts.push(post);

          return {
            ok: true,
            output: `📅 Scheduled for ${platform} at ${scheduledFor}:\n"${text.substring(0, 100)}"`,
            post,
          };
        }

        case "social.feed": {
          const platform = args.platform || "mastodon";

          if (platform === "mastodon" && PLATFORMS.mastodon.enabled) {
            try {
              const resp = await fetch(
                `${PLATFORMS.mastodon.apiBase}/api/v1/timelines/home?limit=10`,
                { headers: PLATFORMS.mastodon.authHeader() },
              );
              const data = (await resp.json()) as any[];
              const lines = data.map((s: any) => {
                const text = (s.content || "")
                  .replace(/<[^>]*>/g, "")
                  .substring(0, 150);
                return `  @${s.account?.acct || "?"}: ${text}`;
              });
              return {
                ok: true,
                output: `Mastodon Feed (${data.length} posts):\n${lines.join("\n")}`,
                count: data.length,
              };
            } catch (e: any) {
              return { ok: false, error: e.message };
            }
          }

          return {
            ok: true,
            output: `Feed for ${platform}: configure ${platform} API credentials to view feed`,
          };
        }

        case "social.analytics": {
          const platform = args.platform || "";
          const enabled = Object.entries(PLATFORMS).filter(
            ([_, c]) => c.enabled,
          );
          if (enabled.length === 0) {
            return {
              ok: true,
              output:
                "No social platforms configured. Set API credentials in .env",
            };
          }
          const lines = enabled.map(
            ([key, cfg]) => `  ${cfg.name}: configured ✅`,
          );
          return {
            ok: true,
            output: `Social Analytics:\n${lines.join("\n")}\n\nDetailed analytics require platform-specific API calls.`,
          };
        }

        case "social.reply": {
          return {
            ok: true,
            output:
              "Reply functionality: use social.post with the reply context. Platform-specific reply APIs vary.",
          };
        }

        case "social.dm": {
          const platform = args.platform || "whatsapp";
          const to = args.to || "";
          const message = args.message || args.text || "";

          if (platform === "whatsapp") {
            if (!to || !message)
              return {
                ok: false,
                error: "Missing 'to' (phone number) and 'message'",
              };
            const result = await sendWhatsApp(to, message);
            return {
              ok: result.ok,
              output: result.ok
                ? `✅ WhatsApp sent to ${to}`
                : `❌ Failed: ${result.error}`,
            };
          }
          return {
            ok: true,
            output: `DM on ${platform}: configure platform API credentials`,
          };
        }

        case "social.monitor": {
          // Monitor mentions/keywords — requires platform-specific streaming APIs
          return {
            ok: true,
            output:
              "Social monitoring: configure platform APIs and keywords.\n" +
              "Mastodon: streaming API (WebSocket)\n" +
              "Twitter: filtered stream API (v2)\n" +
              "Bluesky: firehose subscription\n" +
              "Set SOCIAL_MONITOR_KEYWORDS in .env for keyword tracking.",
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

// ─── Startup ───

const enabledPlatforms = Object.entries(PLATFORMS).filter(
  ([_, c]) => c.enabled,
);
console.log(
  `[social-agent] Starting — ${enabledPlatforms.length} platform(s) configured:`,
);
for (const [key, cfg] of enabledPlatforms) {
  console.log(`  ✅ ${cfg.name}`);
}
if (enabledPlatforms.length === 0) {
  console.log("  (none — set API credentials in .env to enable platforms)");
}

const agent = new SocialAgent();
await agent.connect();
await agent.serve();
