import { RedNodeAgent } from "../../shared/src/agent.js";
import { sh, api, llm, cns, pihole, truenas, frigate, ha } from "../../shared/src/helpers.js";
import * as fs from "fs";
import * as path from "path";

const CNS = process.env.REDNODE_CNS || "http://localhost:8787";
const NOTES_DIR = process.env.NOTES_DIR || "/var/lib/rednode/notes";
const TASKS_FILE = process.env.TASKS_FILE || "/var/lib/rednode/tasks.json";

const TOOLS = [
  "bookmarks.save",
  "bookmarks.search",
  "habits.streak",
  "habits.track",
  "journal.entry",
  "journal.search",
  "notes.create",
  "notes.export",
  "notes.link",
  "notes.list",
  "notes.read",
  "notes.search",
  "notes.tag",
  "pomodoro.start",
  "pomodoro.status",
  "tasks.complete",
  "tasks.create",
  "tasks.delete",
  "tasks.due",
  "tasks.list",
  "tasks.priority",
  "tasks.project",
  "tasks.recurring",
];

// ─── Notes — Markdown files + RAG ingest ───

function ensureDir(dir: string) {
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
}

function slugify(text: string): string {
  return text
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/(^-|-$)/g, "")
    .substring(0, 60);
}

async function ingestToRAG(source: string, content: string) {
  try {
    await fetch(`${CNS}/memory/ingest`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ source, content }),
    });
  } catch {}
}

// ─── Tasks — JSON file ───

interface Task {
  id: string;
  title: string;
  priority: "low" | "medium" | "high" | "urgent";
  created: string;
  due?: string;
  completed: boolean;
  completed_at?: string;
}

function loadTasks(): Task[] {
  try {
    if (fs.existsSync(TASKS_FILE))
      return JSON.parse(fs.readFileSync(TASKS_FILE, "utf-8"));
  } catch {}
  return [];
}

function saveTasks(tasks: Task[]) {
  ensureDir(path.dirname(TASKS_FILE));
  fs.writeFileSync(TASKS_FILE, JSON.stringify(tasks, null, 2));
}

// ─── Bookmarks ───

const BOOKMARKS_FILE =
  process.env.BOOKMARKS_FILE || "/var/lib/rednode/bookmarks.json";

interface Bookmark {
  id: string;
  url: string;
  title: string;
  summary: string;
  tags: string[];
  created: string;
}

function loadBookmarks(): Bookmark[] {
  try {
    if (fs.existsSync(BOOKMARKS_FILE))
      return JSON.parse(fs.readFileSync(BOOKMARKS_FILE, "utf-8"));
  } catch {}
  return [];
}

function saveBookmarks(bm: Bookmark[]) {
  ensureDir(path.dirname(BOOKMARKS_FILE));
  fs.writeFileSync(BOOKMARKS_FILE, JSON.stringify(bm, null, 2));
}

// ─── Agent ───

class ProductivityAgent extends RedNodeAgent {
  constructor() {
    super("productivity", TOOLS);
  }

  async handleTool(tool: string, args: any): Promise<any> {
    try {
      switch (tool) {
        case "notes.create": {
          const title = args.title || `note-${Date.now()}`;
          const content = args.content || "";
          if (!content) return { ok: false, error: "Missing 'content'" };

          ensureDir(NOTES_DIR);
          const filename = `${slugify(title)}.md`;
          const filepath = path.join(NOTES_DIR, filename);
          const md = `# ${title}\n\n*Created: ${new Date().toISOString()}*\n\n${content}\n`;
          fs.writeFileSync(filepath, md);

          // Ingest into RAG for semantic search
          await ingestToRAG(`note/${filename}`, `${title}: ${content}`);

          return {
            ok: true,
            output: `Note saved: ${filename}`,
            path: filepath,
          };
        }

        case "notes.list": {
          ensureDir(NOTES_DIR);
          const files = fs
            .readdirSync(NOTES_DIR)
            .filter((f) => f.endsWith(".md"))
            .sort();
          if (files.length === 0)
            return {
              ok: true,
              output: "No notes yet. Create one with notes.create",
            };
          const lines = files.map((f) => {
            const stat = fs.statSync(path.join(NOTES_DIR, f));
            return `  ${f} (${(stat.size / 1024).toFixed(1)} KB, ${stat.mtime.toLocaleDateString()})`;
          });
          return {
            ok: true,
            output: `${files.length} notes:\n${lines.join("\n")}`,
            count: files.length,
          };
        }

        case "notes.read": {
          const name = args.name || args.filename || "";
          if (!name) return { ok: false, error: "Missing 'name' argument" };
          const filepath = path.join(
            NOTES_DIR,
            name.endsWith(".md") ? name : `${name}.md`,
          );
          if (!fs.existsSync(filepath))
            return { ok: false, error: `Note not found: ${name}` };
          const content = fs.readFileSync(filepath, "utf-8");
          return { ok: true, output: content.substring(0, 3000) };
        }

        case "notes.search": {
          const query = args.query || args.q || "";
          if (!query) return { ok: false, error: "Missing 'query'" };
          // Use RAG for semantic search
          const resp = await fetch(
            `${CNS}/memory/query?q=${encodeURIComponent(query)}&limit=5`,
          );
          const data = (await resp.json()) as any;
          const noteResults = (data.results || []).filter(
            (r: any) =>
              r.source?.startsWith("note/") ||
              r.content?.includes(query.toLowerCase()),
          );
          if (noteResults.length === 0)
            return { ok: true, output: `No notes matching: "${query}"` };
          const lines = noteResults.map(
            (r: any, i: number) =>
              `[${i + 1}] ${r.source} (${(r.score * 100).toFixed(0)}%): ${r.content.substring(0, 150)}`,
          );
          return { ok: true, output: lines.join("\n"), results: noteResults };
        }

        case "tasks.create": {
          const title = args.title || args.task || "";
          if (!title) return { ok: false, error: "Missing 'title'" };
          const tasks = loadTasks();
          const task: Task = {
            id: Date.now().toString(36),
            title,
            priority: args.priority || "medium",
            created: new Date().toISOString(),
            due: args.due,
            completed: false,
          };
          tasks.push(task);
          saveTasks(tasks);
          return {
            ok: true,
            output: `Task created: [${task.id}] ${title} (${task.priority})`,
            task,
          };
        }

        case "tasks.list": {
          const tasks = loadTasks();
          const showCompleted = args.completed === true;
          const filtered = showCompleted
            ? tasks
            : tasks.filter((t) => !t.completed);
          if (filtered.length === 0)
            return { ok: true, output: "No tasks. All done! 🎉" };
          const lines = filtered.map((t) => {
            const icon = t.completed
              ? "✅"
              : t.priority === "urgent"
                ? "🔴"
                : t.priority === "high"
                  ? "🟠"
                  : t.priority === "medium"
                    ? "🟡"
                    : "🟢";
            const due = t.due ? ` (due: ${t.due})` : "";
            return `  ${icon} [${t.id}] ${t.title}${due}`;
          });
          return {
            ok: true,
            output: `${filtered.length} tasks:\n${lines.join("\n")}`,
            count: filtered.length,
          };
        }

        case "tasks.complete": {
          const id = args.id;
          if (!id) return { ok: false, error: "Missing 'id'" };
          const tasks = loadTasks();
          const task = tasks.find((t) => t.id === id);
          if (!task) return { ok: false, error: `Task not found: ${id}` };
          task.completed = true;
          task.completed_at = new Date().toISOString();
          saveTasks(tasks);
          return { ok: true, output: `✅ Completed: ${task.title}` };
        }

        case "tasks.delete": {
          const id = args.id;
          if (!id) return { ok: false, error: "Missing 'id'" };
          let tasks = loadTasks();
          const before = tasks.length;
          tasks = tasks.filter((t) => t.id !== id);
          saveTasks(tasks);
          return {
            ok: true,
            output:
              before !== tasks.length
                ? `Deleted task: ${id}`
                : `Task not found: ${id}`,
          };
        }

        case "bookmarks.save": {
          const url = args.url;
          if (!url) return { ok: false, error: "Missing 'url'" };
          const title = args.title || url;
          const summary = args.summary || "";
          const tags = args.tags || [];
          const bms = loadBookmarks();
          const bm: Bookmark = {
            id: Date.now().toString(36),
            url,
            title,
            summary,
            tags,
            created: new Date().toISOString(),
          };
          bms.push(bm);
          saveBookmarks(bms);
          await ingestToRAG(
            `bookmark/${bm.id}`,
            `${title}: ${url} — ${summary}`,
          );
          return { ok: true, output: `Bookmark saved: ${title}`, bookmark: bm };
        }

        case "bookmarks.search": {
          const query = args.query || args.q || "";
          if (!query) return { ok: false, error: "Missing 'query'" };
          const bms = loadBookmarks();
          const q = query.toLowerCase();
          const matches = bms.filter(
            (b) =>
              b.title.toLowerCase().includes(q) ||
              b.url.toLowerCase().includes(q) ||
              b.summary.toLowerCase().includes(q) ||
              b.tags.some((t) => t.toLowerCase().includes(q)),
          );
          if (matches.length === 0)
            return { ok: true, output: `No bookmarks matching: "${query}"` };
          const lines = matches.map((b) => `  ${b.title}\n    ${b.url}`);
          return { ok: true, output: lines.join("\n"), count: matches.length };
        }
      case "notes.tag": {
        const id = args.id || ""; const tag = args.tag || ""; if (!id || !tag) return { ok: false, error: "Missing note id and tag" }; const r = await cns("/memory/store", { method: "POST", body: { type: "note_tag", key: `${id}-${tag}`, value: { note_id: id, tag } } }); return { ok: r.ok, output: `Tag "${tag}" added to note ${id}`, tool }; PostgreSQL tag management
      }

      case "notes.export": {
        const r = await cns("/memory/query?type=note&limit=100"); return { ok: r.ok, output: r.output || "No notes to export", tool }; PostgreSQL → markdown/PDF
      }

      case "notes.link": {
        const from = args.from || ""; const to = args.to || ""; if (!from || !to) return { ok: false, error: "Missing from and to note IDs" }; const r = await cns("/memory/store", { method: "POST", body: { type: "note_link", key: `${from}-${to}`, value: { from, to } } }); return { ok: r.ok, output: `Notes linked: ${from} → ${to}`, tool }; PostgreSQL note linking
      }

      case "tasks.priority": {
        const id = args.id || ""; const priority = args.priority || "medium"; if (!id) return { ok: false, error: "Missing task ID" }; const r = await cns("/memory/store", { method: "POST", body: { type: "task_update", key: id, value: { priority } } }); return { ok: r.ok, output: `Task ${id} priority set to ${priority}`, tool }; PostgreSQL priority update
      }

      case "tasks.due": {
        const id = args.id || ""; const due = args.due || args.date || ""; if (!id || !due) return { ok: false, error: "Missing task ID and due date" }; const r = await cns("/memory/store", { method: "POST", body: { type: "task_update", key: id, value: { due_date: due } } }); return { ok: r.ok, output: `Task ${id} due date set to ${due}`, tool }; PostgreSQL due date update
      }

      case "tasks.recurring": {
        const r = await cns("/memory/query?type=note"); return { ok: r.ok, output: r.output, tool };
      }

      case "tasks.project": {
        const name = args.name || ""; const schedule = args.schedule || args.cron || ""; if (!name || !schedule) return { ok: false, error: "Missing task name and schedule" }; const r = await cns("/memory/store", { method: "POST", body: { type: "recurring_task", key: name, value: { name, schedule, created: new Date().toISOString() } } }); return { ok: r.ok, output: `Recurring task "${name}" created: ${schedule}`, tool }; PostgreSQL project grouping
      }

      case "habits.track": {
        const habit = args.habit || args.name || ""; if (!habit) return { ok: false, error: "Missing habit name" }; const r = await cns("/memory/store", { method: "POST", body: { type: "habit_log", key: `${habit}-${new Date().toISOString().split("T")[0]}`, value: { habit, date: new Date().toISOString(), completed: true } } }); return { ok: r.ok, output: `✅ Habit "${habit}" logged for today`, tool }; PostgreSQL habit log
      }

      case "habits.streak": {
        const habit = args.habit || args.name || ""; if (!habit) return { ok: false, error: "Missing habit name" }; const r = await cns(`/memory/query?type=habit_log&filter=${habit}`); return { ok: r.ok, output: r.output, tool }; PostgreSQL streak calculation
      }

      case "pomodoro.start": {
        const minutes = args.minutes || 25; return { ok: true, output: `🍅 Pomodoro started: ${minutes} minutes. Focus!`, tool, started: new Date().toISOString(), duration_minutes: minutes }; timer management
      }

      case "pomodoro.status": {
        return { ok: true, output: "Pomodoro timer status — check your dashboard at http://localhost:3000", tool }; timer status
      }

      case "journal.entry": {
        const entry = args.entry || args.text || args.content || ""; if (!entry) return { ok: false, error: "Missing journal entry text" }; const date = new Date().toISOString().split("T")[0]; const r = await cns("/memory/store", { method: "POST", body: { type: "journal", key: `journal-${date}`, value: { date, entry, created: new Date().toISOString() } } }); return { ok: r.ok, output: `📝 Journal entry saved for ${date}`, tool }; PostgreSQL journal insert
      }

      case "journal.search": {
        const query = args.query || args.q || ""; if (!query) return { ok: false, error: "Missing search query" }; const r = await cns(`/memory/query?type=journal&filter=${encodeURIComponent(query)}`); return { ok: r.ok, output: r.output, tool }; PostgreSQL journal search
      }



        default:
          return null;
      }
    } catch (e: any) {
      return { ok: false, error: e.message };
    }
  }
}

const agent = new ProductivityAgent();
await agent.connect();
await agent.serve();
