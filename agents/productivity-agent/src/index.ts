import { RedNodeAgent } from "../../shared/src/agent.js";
import * as fs from "fs";
import * as path from "path";

const CNS = process.env.REDNODE_CNS || "http://localhost:8787";
const NOTES_DIR = process.env.NOTES_DIR || "/var/lib/rednode/notes";
const TASKS_FILE = process.env.TASKS_FILE || "/var/lib/rednode/tasks.json";

const TOOLS = [
  "notes.create",
  "notes.search",
  "notes.list",
  "notes.read",
  "tasks.create",
  "tasks.list",
  "tasks.complete",
  "tasks.delete",
  "bookmarks.save",
  "bookmarks.search",
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
