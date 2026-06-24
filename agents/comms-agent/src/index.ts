import { RedNodeAgent } from "../../shared/src/agent.js";
import { sh, api, llm, cns, pihole, truenas, frigate, ha } from "../../shared/src/helpers.js";

const CNS = process.env.REDNODE_CNS || "http://localhost:8787";
const OLLAMA_URL = process.env.OLLAMA_URL || "http://127.0.0.1:11434";
const MODEL = process.env.REDNODE_MODEL || "qwen2.5:14b-instruct-q4_K_M";

// Email config — set via environment variables
const IMAP_HOST = process.env.IMAP_HOST || "";
const IMAP_PORT = parseInt(process.env.IMAP_PORT || "993");
const IMAP_USER = process.env.IMAP_USER || "";
const IMAP_PASS = process.env.IMAP_PASS || "";
const SMTP_HOST = process.env.SMTP_HOST || "";
const SMTP_PORT = parseInt(process.env.SMTP_PORT || "587");
const SMTP_USER = process.env.SMTP_USER || IMAP_USER;
const SMTP_PASS = process.env.SMTP_PASS || IMAP_PASS;

// CalDAV config
const CALDAV_URL = process.env.CALDAV_URL || "";
const CALDAV_USER = process.env.CALDAV_USER || "";
const CALDAV_PASS = process.env.CALDAV_PASS || "";

const TOOLS = [
  "calendar.availability",
  "calendar.conflicts",
  "calendar.create",
  "calendar.reschedule",
  "calendar.view",
  "contacts.add",
  "contacts.birthday_remind",
  "contacts.search",
  "email.archive",
  "email.auto_draft",
  "email.draft",
  "email.fetch",
  "email.rules",
  "email.search",
  "email.send",
  "email.summarize",
  "email.triage",
  "email.unsubscribe",
];

// ─── Email via IMAP ───

async function fetchEmails(
  limit: number = 10,
  folder: string = "INBOX",
): Promise<any[]> {
  if (!IMAP_HOST)
    return [
      {
        error:
          "IMAP not configured — set IMAP_HOST, IMAP_USER, IMAP_PASS environment variables",
      },
    ];

  const { ImapFlow } = await import("imapflow");

  const client = new ImapFlow({
    host: IMAP_HOST,
    port: IMAP_PORT,
    secure: true,
    auth: { user: IMAP_USER, pass: IMAP_PASS },
    logger: false,
  });

  const emails: any[] = [];

  try {
    await client.connect();
    const lock = await client.getMailboxLock(folder);

    try {
      const mailboxExists =
        typeof client.mailbox === "boolean"
          ? 0
          : Math.max(1, client.mailbox.exists - limit + 1);

      const messages = client.fetch(`${mailboxExists}:*`, {
        envelope: true,
        bodyStructure: true,
        source: { maxLength: 5000 }, // first 5KB of body
      });

      for await (const msg of messages) {
        const env = msg.envelope ?? {};
        emails.push({
          uid: msg.uid,
          date: env.date?.toISOString(),
          from: env.from?.[0]?.address || "unknown",
          from_name: env.from?.[0]?.name || "",
          subject: env.subject || "(no subject)",
          to: env.to?.map((t: any) => t.address).join(", ") || "",
          snippet: msg.source?.toString("utf-8")?.substring(0, 200) || "",
        });
      }
    } finally {
      lock.release();
    }

    await client.logout();
  } catch (e: any) {
    return [{ error: `IMAP error: ${e.message}` }];
  }

  return emails.reverse(); // newest first
}

// ─── Email Sending via SMTP ───

async function sendEmail(
  to: string,
  subject: string,
  body: string,
): Promise<{ ok: boolean; message: string }> {
  if (!SMTP_HOST)
    return {
      ok: false,
      message: "SMTP not configured — set SMTP_HOST, SMTP_USER, SMTP_PASS",
    };

  const nodemailer = await import("nodemailer");

  const transporter = nodemailer.createTransport({
    host: SMTP_HOST,
    port: SMTP_PORT,
    secure: SMTP_PORT === 465,
    auth: { user: SMTP_USER, pass: SMTP_PASS },
  });

  try {
    const info = await transporter.sendMail({
      from: SMTP_USER,
      to,
      subject,
      text: body,
    });
    return { ok: true, message: `Email sent: ${info.messageId}` };
  } catch (e: any) {
    return { ok: false, message: `SMTP error: ${e.message}` };
  }
}

// ─── LLM Summarization ───

async function summarizeWithLLM(
  text: string,
  instruction: string,
): Promise<string> {
  try {
    const resp = await fetch(`${OLLAMA_URL}/api/chat`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        model: MODEL,
        messages: [
          {
            role: "system",
            content: "You are a concise assistant. Be brief and clear.",
          },
          { role: "user", content: `${instruction}\n\n${text}` },
        ],
        stream: false,
        options: { temperature: 0.3, num_predict: 512 },
      }),
    });
    const data = (await resp.json()) as any;
    return data.message?.content || "No summary generated";
  } catch (e: any) {
    return `LLM unavailable: ${e.message}`;
  }
}

// ─── CalDAV Calendar ───

async function fetchCalendarEvents(days: number = 7): Promise<any[]> {
  if (!CALDAV_URL)
    return [
      {
        error:
          "CalDAV not configured — set CALDAV_URL, CALDAV_USER, CALDAV_PASS",
      },
    ];

  try {
    const { DAVClient } = await import("tsdav");

    const client = new DAVClient({
      serverUrl: CALDAV_URL,
      credentials: { username: CALDAV_USER, password: CALDAV_PASS },
      authMethod: "Basic",
      defaultAccountType: "caldav",
    });

    await client.login();
    const calendars = await client.fetchCalendars();

    if (calendars.length === 0) return [{ info: "No calendars found" }];

    const now = new Date();
    const future = new Date(now.getTime() + days * 24 * 60 * 60 * 1000);

    const events: any[] = [];
    for (const cal of calendars) {
      const calEvents = await client.fetchCalendarObjects({
        calendar: cal,
        timeRange: {
          start: now.toISOString(),
          end: future.toISOString(),
        },
      });

      for (const ev of calEvents) {
        // Parse basic event info from iCal data
        const data = ev.data || "";
        const summary = data.match(/SUMMARY:(.*)/)?.[1]?.trim() || "Untitled";
        const dtstart = data.match(/DTSTART[^:]*:(.*)/)?.[1]?.trim() || "";
        const dtend = data.match(/DTEND[^:]*:(.*)/)?.[1]?.trim() || "";
        const location = data.match(/LOCATION:(.*)/)?.[1]?.trim() || "";

        events.push({
          summary,
          start: dtstart,
          end: dtend,
          location,
          calendar: cal.displayName,
        });
      }
    }

    return events.sort((a, b) => (a.start || "").localeCompare(b.start || ""));
  } catch (e: any) {
    return [{ error: `CalDAV error: ${e.message}` }];
  }
}

// ─── Agent ───

class CommsAgent extends RedNodeAgent {
  constructor() {
    super("comms", TOOLS);
  }

  async handleTool(tool: string, args: any): Promise<any> {
    try {
      switch (tool) {
        case "email.fetch": {
          const limit = args.limit || 10;
          const folder = args.folder || "INBOX";
          const emails = await fetchEmails(limit, folder);
          const lines = emails.map((e: any) =>
            e.error
              ? e.error
              : `${e.date?.substring(0, 16) || "?"} | ${e.from_name || e.from} | ${e.subject}`,
          );
          return {
            ok: true,
            output: lines.join("\n"),
            emails,
            count: emails.length,
          };
        }

        case "email.summarize": {
          const limit = args.limit || 5;
          const emails = await fetchEmails(limit);
          if (emails.length === 0 || emails[0]?.error) {
            return { ok: true, output: emails[0]?.error || "No emails found" };
          }

          const emailText = emails
            .map(
              (e: any) =>
                `From: ${e.from_name || e.from}\nSubject: ${e.subject}\nSnippet: ${e.snippet}`,
            )
            .join("\n---\n");

          const summary = await summarizeWithLLM(
            emailText,
            `Summarize these ${emails.length} emails in 2-3 sentences. Highlight anything urgent or actionable.`,
          );

          return { ok: true, output: summary, email_count: emails.length };
        }

        case "email.draft": {
          const to = args.to || "";
          const about = args.about || args.subject || "";
          const context = args.context || "";
          if (!about)
            return {
              ok: false,
              error: "Missing 'about' or 'subject' argument",
            };

          const draft = await summarizeWithLLM(
            context,
            `Draft a professional email about: ${about}${to ? ` to ${to}` : ""}. Keep it concise. Include subject line.`,
          );

          return {
            ok: true,
            output: `Draft:\n\n${draft}\n\n(Use email.send to send this)`,
            draft,
          };
        }

        case "email.send": {
          const to = args.to;
          const subject = args.subject;
          const body = args.body;
          if (!to || !subject || !body) {
            return {
              ok: false,
              error: "Missing required arguments: to, subject, body",
            };
          }
          const result = await sendEmail(to, subject, body);
          return { ok: result.ok, output: result.message };
        }

        case "email.rules": {
          return {
            ok: true,
            output:
              "Email rules management: configure IMAP filters directly in your email provider. RedNode can auto-label via email.fetch + LLM classification — coming in Phase 5b.",
          };
        }

        case "calendar.view": {
          const days = args.days || 7;
          const events = await fetchCalendarEvents(days);
          if (events.length === 0 || events[0]?.error) {
            return {
              ok: true,
              output: events[0]?.error || "No upcoming events",
            };
          }
          const lines = events.map(
            (e: any) =>
              `${e.start || "?"} | ${e.summary}${e.location ? ` @ ${e.location}` : ""}`,
          );
          return {
            ok: true,
            output: `Upcoming events (next ${days} days):\n${lines.join("\n")}`,
            events,
          };
        }

        case "calendar.create": {
          return {
            ok: true,
            output:
              "Calendar event creation via CalDAV — requires building iCal VEVENT. Coming in Phase 5b. For now, use your calendar app directly.",
          };
        }

        case "calendar.conflicts": {
          const events = await fetchCalendarEvents(7);
          if (events.length < 2)
            return {
              ok: true,
              output: "Not enough events to check for conflicts",
            };
          // Simple overlap detection
          const conflicts: string[] = [];
          for (let i = 0; i < events.length - 1; i++) {
            for (let j = i + 1; j < events.length; j++) {
              if (events[i].start && events[j].start && events[i].end) {
                if (events[j].start < events[i].end) {
                  conflicts.push(
                    `Conflict: "${events[i].summary}" overlaps with "${events[j].summary}"`,
                  );
                }
              }
            }
          }
          return {
            ok: true,
            output:
              conflicts.length > 0
                ? conflicts.join("\n")
                : "No scheduling conflicts found ✅",
            conflicts,
          };
        }

        case "contacts.search": {
          return {
            ok: true,
            output: "Contact search via CardDAV — coming in Phase 5b",
          };
        }

        case "notifications.digest": {
          // Pull recent security events + audit log + email summary
          const parts: string[] = [];

          // Security events
          try {
            const secResp = await fetch(`${CNS}/security/events`);
            const secData = (await secResp.json()) as any;
            const unacked = (secData.events || []).filter(
              (e: any) => !e.acknowledged,
            );
            if (unacked.length > 0) {
              parts.push(`🛡️ ${unacked.length} unacknowledged security events`);
              for (const e of unacked.slice(0, 3)) {
                parts.push(`  [${e.severity}] ${e.summary}`);
              }
            }
          } catch {}

          // Approvals
          try {
            const appResp = await fetch(`${CNS}/approvals`);
            const appData = (await appResp.json()) as any;
            const pending = appData.approvals || [];
            if (pending.length > 0) {
              parts.push(`⏳ ${pending.length} pending approvals`);
            }
          } catch {}

          // Email summary
          try {
            const emails = await fetchEmails(5);
            if (emails.length > 0 && !emails[0]?.error) {
              parts.push(`📧 ${emails.length} recent emails`);
              for (const e of emails.slice(0, 3)) {
                parts.push(`  ${e.from_name || e.from}: ${e.subject}`);
              }
            }
          } catch {}

          // Calendar
          try {
            const events = await fetchCalendarEvents(1);
            if (events.length > 0 && !events[0]?.error) {
              parts.push(`📅 ${events.length} events today`);
              for (const e of events.slice(0, 3)) {
                parts.push(
                  `  ${e.start?.substring(9, 14) || "?"} ${e.summary}`,
                );
              }
            }
          } catch {}

          if (parts.length === 0) {
            parts.push("All clear — no notifications");
          }

          return {
            ok: true,
            output: `📋 Notification Digest:\n\n${parts.join("\n")}`,
          };
        }
      case "email.triage": {
        const r = await cns("/intent", { method: "POST", body: { intent: "fetch and classify emails by urgency", session: "email-triage" } }); return { ok: r.ok, output: r.output || "Email triage requires IMAP connection — configure EMAIL_IMAP_* in .env", tool }; IMAP fetch + LLM classification
      }

      case "email.auto_draft": {
        const r = await cns("/intent", { method: "POST", body: { intent: "fetch emails and classify by urgency", session: "comms-triage" } }); return { ok: r.ok, output: r.output, tool };
      }

      case "email.search": {
        const query = args.query || args.q || ""; if (!query) return { ok: false, error: "Missing search query" }; return { ok: true, output: `Email search for "${query}" requires IMAP connection — configure EMAIL_IMAP_* in .env`, tool }; IMAP SEARCH command
      }

      case "email.archive": {
        const subject = args.subject || ""; if (!subject) return { ok: false, error: "Missing email subject to draft reply for" }; const draft = await llm(`Draft a professional reply to an email with subject: "${subject}". Context: ${args.context || "none"}`); return { ok: true, output: draft, tool };
      }

      case "email.unsubscribe": {
        return { ok: true, output: "Email archive requires IMAP MOVE — configure EMAIL_IMAP_* in .env", tool };
      }

      case "calendar.reschedule": {
        return { ok: true, output: "Email unsubscribe detection requires IMAP connection — configure EMAIL_IMAP_* in .env", tool };
      }

      case "calendar.availability": {
        return { ok: true, output: "Calendar reschedule requires CalDAV connection — configure CALDAV_URL in .env", tool }; CalDAV free/busy query
      }

      case "contacts.add": {
        return { ok: true, output: "Calendar availability requires CalDAV free-busy query — configure CALDAV_URL in .env", tool }; CardDAV vCard creation
      }

      case "contacts.birthday_remind": {
        return { ok: true, output: "Contact add requires CardDAV connection — configure CARDDAV_URL in .env", tool }; CardDAV birthday scan
      }



        default:
          return { ok: true, output: "Birthday reminders require CardDAV BDAY scan — configure CARDDAV_URL in .env", tool };
      }
    } catch (e: any) {
      console.error(`[comms-agent] ${tool} failed:`, e.message);
      return { ok: false, error: e.message };
    }
  }
}

const agent = new CommsAgent();
await agent.connect();

// ─── Calendar Awareness — Proactive Reminders ───
// Checks upcoming calendar events every 5 minutes.
// If an event starts within 30 minutes, sends a security event as a reminder.

const REMINDER_MINUTES = parseInt(
  process.env.CALENDAR_REMINDER_MINUTES || "30",
);
const REMINDER_INTERVAL = 300000; // 5 minutes
const remindedEvents = new Set<string>(); // prevent duplicate reminders

async function checkUpcomingEvents() {
  if (!CALDAV_URL) return; // Calendar not configured

  try {
    const events = await fetchCalendarEvents(1); // today only
    if (!events || events.length === 0 || events[0]?.error) return;

    const now = new Date();

    for (const event of events) {
      if (!event.start) continue;

      // Parse event start time
      let eventTime: Date;
      try {
        // CalDAV returns various formats: 20260615T100000Z, 2026-06-15T10:00:00
        const cleaned = event.start
          .replace(
            /(\d{4})(\d{2})(\d{2})T(\d{2})(\d{2})(\d{2})/,
            "$1-$2-$3T$4:$5:$6",
          )
          .replace(/Z$/, "+00:00");
        eventTime = new Date(cleaned);
        if (isNaN(eventTime.getTime())) continue;
      } catch {
        continue;
      }

      const minutesUntil = (eventTime.getTime() - now.getTime()) / 60000;
      const eventKey = `${event.summary}-${event.start}`;

      // Reminder: event starts within REMINDER_MINUTES and hasn't been reminded
      if (
        minutesUntil > 0 &&
        minutesUntil <= REMINDER_MINUTES &&
        !remindedEvents.has(eventKey)
      ) {
        remindedEvents.add(eventKey);

        const summary = `📅 Upcoming: "${event.summary}" in ${Math.round(minutesUntil)} minutes${event.location ? ` @ ${event.location}` : ""}`;
        console.log(`[comms-agent] ${summary}`);

        // Send as security event (shows in dashboard + Signal bot)
        try {
          await fetch(`${CNS}/security/events`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              severity: minutesUntil <= 10 ? "MEDIUM" : "LOW",
              source: "calendar-reminder",
              summary,
              raw: {
                event: event.summary,
                start: event.start,
                location: event.location,
                minutes_until: Math.round(minutesUntil),
              },
            }),
          });
        } catch {}

        // Emit to event bus for real-time dashboard
        try {
          await fetch(`${CNS}/intent`, {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              intent: `calendar reminder: ${event.summary} starts in ${Math.round(minutesUntil)} minutes`,
              session_id: "calendar-awareness",
            }),
          });
        } catch {}
      }
    }

    // Clean old reminded events (older than 2 hours)
    if (remindedEvents.size > 100) {
      remindedEvents.clear();
    }
  } catch (e: any) {
    // Silent fail — calendar might not be configured
  }
}

// Start calendar awareness loop
setTimeout(checkUpcomingEvents, 15000); // first check after 15s
setInterval(checkUpcomingEvents, REMINDER_INTERVAL);
console.log(
  `[comms-agent] Calendar awareness active — reminders ${REMINDER_MINUTES}min before events`,
);

await agent.serve();
