/**
 * RedNode-OS – Signal Bot
 *
 * Chat with RedNode from Signal messenger.
 * Fully self-owned — uses signal-cli (self-hosted, no cloud).
 *
 * Setup:
 *   1. Install signal-cli: https://github.com/AsamK/signal-cli
 *      - NixOS: nix-env -i signal-cli
 *      - Docker: docker run -v signal-data:/home/.local/share/signal-cli asamk/signal-cli
 *
 *   2. Register a phone number for the bot:
 *      signal-cli -u +1YOURNUMBER register
 *      signal-cli -u +1YOURNUMBER verify CODE
 *
 *   3. Set environment variables:
 *      SIGNAL_CLI_PATH=/usr/bin/signal-cli
 *      SIGNAL_BOT_NUMBER=+1YOURNUMBER
 *      SIGNAL_OWNER_NUMBER=+1YOURPERSONALNUMBER
 *
 *   4. Start: pnpm --filter @rednode/signal-bot dev
 *
 * Architecture:
 *   Signal message → signal-cli JSON RPC → this bot → CNS /intent → response → signal-cli → Signal reply
 *   Everything runs locally. No data touches any cloud. E2EE from your phone to your RedNode server.
 */

import { exec, spawn, ChildProcess } from "child_process";
import { promisify } from "util";
const execAsync = promisify(exec);

const CNS = process.env.REDNODE_CNS || "http://localhost:8787";
const SIGNAL_CLI = process.env.SIGNAL_CLI_PATH || "signal-cli";
const BOT_NUMBER = process.env.SIGNAL_BOT_NUMBER || "";
const OWNER_NUMBER = process.env.SIGNAL_OWNER_NUMBER || "";
const POLL_INTERVAL = parseInt(process.env.SIGNAL_POLL_INTERVAL || "3000"); // ms

if (!BOT_NUMBER) {
  console.error(
    "[signal-bot] ❌ SIGNAL_BOT_NUMBER not set. See README for setup instructions.",
  );
  console.error("  export SIGNAL_BOT_NUMBER=+1234567890");
  process.exit(1);
}

if (!OWNER_NUMBER) {
  console.warn(
    "[signal-bot] ⚠️ SIGNAL_OWNER_NUMBER not set — bot will respond to ALL messages",
  );
}

console.log(
  `[signal-bot] Starting — bot: ${BOT_NUMBER}, owner: ${OWNER_NUMBER || "(any)"}`,
);

// ─── Signal CLI Interface ───

async function sendSignalMessage(
  recipient: string,
  message: string,
): Promise<void> {
  try {
    // Truncate long messages (Signal has a 4096 char limit)
    const truncated =
      message.length > 3500
        ? message.substring(0, 3500) + "\n\n... (truncated)"
        : message;

    await execAsync(
      `${SIGNAL_CLI} -u ${BOT_NUMBER} send -m ${JSON.stringify(truncated)} ${recipient}`,
      { timeout: 15000 },
    );
  } catch (e: any) {
    console.error(`[signal-bot] Failed to send message: ${e.message}`);
  }
}

async function receiveMessages(): Promise<any[]> {
  try {
    const { stdout } = await execAsync(
      `${SIGNAL_CLI} -u ${BOT_NUMBER} receive --json --timeout 1`,
      { timeout: 10000 },
    );

    if (!stdout.trim()) return [];

    // signal-cli outputs one JSON object per line
    const messages: any[] = [];
    for (const line of stdout.trim().split("\n")) {
      try {
        const msg = JSON.parse(line);
        if (msg.envelope?.dataMessage?.message) {
          messages.push({
            sender: msg.envelope.source || msg.envelope.sourceNumber,
            text: msg.envelope.dataMessage.message,
            timestamp: msg.envelope.timestamp,
          });
        }
      } catch {}
    }
    return messages;
  } catch {
    return [];
  }
}

// ─── Intent Processing ───

async function processIntent(text: string): Promise<string> {
  try {
    const resp = await fetch(`${CNS}/intent`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ intent: text, session_id: "signal" }),
    });
    const data = (await resp.json()) as any;

    if (!data.ok) return "❌ Failed to process intent.";

    // Format results for Signal message
    const parts: string[] = [`🧠 RedNode — "${text}"\n`];

    // Plan
    if (data.plan?.length > 0) {
      parts.push(`📋 Plan (${data.plan.length} steps):`);
      for (const step of data.plan) {
        parts.push(`  → ${step.tool} [${step.risk}]`);
      }
      parts.push("");
    }

    // Results
    for (const r of data.results || []) {
      const icon =
        r.status === "executed"
          ? "✅"
          : r.status === "needs_approval"
            ? "⏳"
            : "❌";
      parts.push(`${icon} ${r.tool}: ${r.status}`);

      const output = r.result?.output || r.result?.result?.output;
      if (output) {
        // Keep output concise for Signal
        const short = String(output)
          .substring(0, 500)
          .replace(/\n{3,}/g, "\n\n");
        parts.push(short);
        parts.push("");
      }
    }

    return parts.join("\n").trim();
  } catch (e: any) {
    return `❌ Error: ${e.message}`;
  }
}

// ─── Quick Commands ───

const QUICK_COMMANDS: Record<string, string> = {
  "/status": "show system status and sentience drives",
  "/health": "show system health",
  "/cameras": "show camera status and recent person detections",
  "/security": "show recent security events",
  "/nas": "check TrueNAS pool health",
  "/pihole": "show pihole DNS stats",
  "/goodnight": "run workflow goodnight",
  "/morning": "run workflow morning",
  "/focus": "run workflow focus",
  "/tasks": "show my tasks",
  "/help": "HELP",
};

function getHelpText(): string {
  return `🧠 RedNode-OS Signal Bot\n\nCommands:\n${Object.entries(
    QUICK_COMMANDS,
  )
    .filter(([_, v]) => v !== "HELP")
    .map(([cmd, desc]) => `  ${cmd} — ${desc}`)
    .join(
      "\n",
    )}\n\nOr just type any intent naturally:\n  "check if any cameras detected people today"\n  "create a note about the server migration"\n  "what's my disk usage?"`;
}

// ─── Main Loop ───

async function main() {
  // Verify signal-cli works
  try {
    await execAsync(`${SIGNAL_CLI} --version`, { timeout: 5000 });
    console.log("[signal-bot] signal-cli found ✅");
  } catch {
    console.error(`[signal-bot] ❌ signal-cli not found at: ${SIGNAL_CLI}`);
    console.error("  Install: https://github.com/AsamK/signal-cli");
    console.error("  NixOS: nix-env -i signal-cli");
    process.exit(1);
  }

  console.log("[signal-bot] 📱 Listening for Signal messages...");
  console.log(`[signal-bot] Send a message to ${BOT_NUMBER} from Signal`);

  // Poll for messages
  setInterval(async () => {
    const messages = await receiveMessages();

    for (const msg of messages) {
      // Security: only respond to owner if configured
      if (OWNER_NUMBER && msg.sender !== OWNER_NUMBER) {
        console.log(
          `[signal-bot] Ignoring message from non-owner: ${msg.sender}`,
        );
        await sendSignalMessage(
          msg.sender,
          "⛔ Unauthorized. This RedNode bot only responds to its owner.",
        );
        continue;
      }

      const text = msg.text.trim();
      console.log(`[signal-bot] 📩 ${msg.sender}: "${text}"`);

      // Quick commands
      const quickCmd = QUICK_COMMANDS[text.toLowerCase()];
      if (quickCmd === "HELP") {
        await sendSignalMessage(msg.sender, getHelpText());
        continue;
      }

      const intentText = quickCmd || text;

      // Process through CNS
      const response = await processIntent(intentText);
      console.log(`[signal-bot] 📤 Response: ${response.substring(0, 100)}...`);
      await sendSignalMessage(msg.sender, response);
    }
  }, POLL_INTERVAL);
}

main().catch((e) => {
  console.error("[signal-bot] Fatal:", e);
  process.exit(1);
});

// ─── NATS Tool Handlers (called by CNS/coordinator) ───
// These allow other agents and pipelines to send messages via Signal

import { connect, StringCodec, JSONCodec } from "nats";

async function startNatsHandler() {
  try {
    const nc = await connect({ servers: process.env.NATS_URL || "nats://localhost:4222" });
    const jc = JSONCodec();

    const sub = nc.subscribe("rednode.tool.signal-bot.>");
    for await (const msg of sub) {
      try {
        const data = jc.decode(msg.data) as any;
        const tool = data.tool || "";
        const args = data.args || {};
        let result: any = { ok: false, error: "Unknown tool" };

        switch (tool) {
          case "signal.send": {
            const recipient = args.to || OWNER_NUMBER;
            const message = args.message || args.body || "";
            if (!message) { result = { ok: false, error: "Missing 'message'" }; break; }
            await sendSignalMessage(recipient, message);
            result = { ok: true, output: `Sent to ${recipient}` };
            break;
          }
          case "signal.send_image": {
            const recipient = args.to || OWNER_NUMBER;
            const image = args.image || args.file || "";
            const caption = args.caption || "";
            if (!image) { result = { ok: false, error: "Missing 'image' path" }; break; }
            try {
              await execAsync(`${SIGNAL_CLI} -u ${BOT_NUMBER} send -m ${JSON.stringify(caption)} -a ${image} ${recipient}`);
              result = { ok: true, output: `Image sent to ${recipient}` };
            } catch (e: any) { result = { ok: false, error: e.message }; }
            break;
          }
          case "signal.send_file": {
            const recipient = args.to || OWNER_NUMBER;
            const file = args.file || args.path || "";
            if (!file) { result = { ok: false, error: "Missing 'file' path" }; break; }
            try {
              await execAsync(`${SIGNAL_CLI} -u ${BOT_NUMBER} send -m "File" -a ${file} ${recipient}`);
              result = { ok: true, output: `File sent to ${recipient}` };
            } catch (e: any) { result = { ok: false, error: e.message }; }
            break;
          }
          case "signal.group_list": {
            try {
              const { stdout } = await execAsync(`${SIGNAL_CLI} -u ${BOT_NUMBER} listGroups -d 2>/dev/null`);
              result = { ok: true, output: stdout.trim() || "No groups" };
            } catch (e: any) { result = { ok: true, output: "signal-cli not configured for groups" }; }
            break;
          }
          case "signal.group_send": {
            const group = args.group || args.group_id || "";
            const message = args.message || "";
            if (!group || !message) { result = { ok: false, error: "Missing 'group' and 'message'" }; break; }
            try {
              await execAsync(`${SIGNAL_CLI} -u ${BOT_NUMBER} send -g ${group} -m ${JSON.stringify(message)}`);
              result = { ok: true, output: `Sent to group ${group}` };
            } catch (e: any) { result = { ok: false, error: e.message }; }
            break;
          }
          case "signal.contacts": {
            try {
              const { stdout } = await execAsync(`${SIGNAL_CLI} -u ${BOT_NUMBER} listContacts 2>/dev/null`);
              result = { ok: true, output: stdout.trim() || "No contacts" };
            } catch (e: any) { result = { ok: true, output: "signal-cli not configured" }; }
            break;
          }
        }

        if (msg.reply) {
          msg.respond(jc.encode(result));
        }
      } catch (e) {
        console.error("[signal-bot] NATS handler error:", e);
      }
    }
  } catch (e) {
    console.error("[signal-bot] NATS connection failed (Signal bot runs standalone):", e);
  }
}

// Start NATS handler in background (non-blocking)
startNatsHandler();
