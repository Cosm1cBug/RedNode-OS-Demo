# RedNode-OS — Obsidian and Knowledge Management

---

## What Is Obsidian?

Obsidian is a note-taking application that stores everything as **local Markdown files** with **bidirectional links** between notes. It's become massively popular because:

- **All data is local** — Markdown `.md` files on your disk, no cloud required
- **Links between notes** create a knowledge graph (like a personal Wikipedia)
- **Graph view** visualizes how your notes connect to each other
- **Plugins** extend it with kanban boards, calendars, dataview queries, templates
- **Zero lock-in** — your notes are plain text files, readable by anything

Example: you write a note about "pfSense Firewall Rules" and link it to "VLAN Configuration" and "Network Security" — Obsidian shows how all three topics connect.

---

## Can Obsidian Replace RedNode's Memory?

**No.** They serve fundamentally different purposes:

| | Obsidian | RedNode Memory |
|---|---|---|
| **For** | Humans reading and writing notes | Machines querying and reasoning over data |
| **Format** | Markdown files with wiki-links | PostgreSQL rows + Qdrant vectors + Kuzu graph |
| **Search** | Full-text search over file contents | Semantic vector search (meaning, not just keywords) |
| **Structure** | Free-form notes linked by `[[topic]]` | Typed propositions with confidence scores + entities + relationships |
| **Query** | Manual reading or Dataview plugin | SQL queries, vector similarity, graph traversal |
| **Speed** | Human reading speed | Millisecond programmatic queries |
| **Audit** | Git history (if configured) | SHA-256 hash-chained tamper-evident audit log |
| **Embedding** | None | Each proposition has a 768-dim vector embedding for semantic search |
| **Memory consolidation** | None (manual review) | Automatic JUDGE + CONSOLIDATE cycles |

---

## How Obsidian CAN Complement RedNode

The best approach is **bidirectional sync** — RedNode and Obsidian feeding each other:

### Direction 1: Obsidian → RedNode (Knowledge Ingestion)

Point RedNode's research agent at your Obsidian vault:

```bash
# In .env
OBSIDIAN_VAULT_PATH=/home/owner/Obsidian/MyVault

# RedNode will:
# 1. Scan all .md files in the vault
# 2. Extract propositions from each note
# 3. Embed them in Qdrant for semantic search
# 4. Build entity-relationship graph from [[links]]
#
# Then you can ask:
#   "What do my notes say about network security?"
#   → RedNode searches your Obsidian vault semantically
```

### Direction 2: RedNode → Obsidian (Knowledge Export)

RedNode can export its learned knowledge as Obsidian-compatible Markdown:

```bash
rednode intent "export knowledge base to Obsidian format"

# Creates in your vault:
# RedNode-Knowledge/
#   Entities/
#     pfSense.md          — everything RedNode knows about your pfSense
#     Pi-hole.md          — DNS stats, common blocks, anomalies
#     Camera-System.md    — camera inventory, common events
#   Daily-Reports/
#     2026-07-12.md       — morning briefing, events, health
#   Security/
#     CVE-History.md      — all CVEs found and actions taken
#     Threat-Intel.md     — IOCs blocked, sources synced
```

### Direction 3: Obsidian as the Human Interface to RedNode's Brain

Think of it as:
- **RedNode's PostgreSQL** = the fast machine memory (millisecond queries, structured data)
- **Obsidian vault** = the human-readable mirror (browse, edit, link, visualize)

You read and organize knowledge in Obsidian. RedNode processes it programmatically from PostgreSQL + Qdrant. Changes in either direction sync to the other.

---

## Should You Use Obsidian with RedNode?

**Yes, if you already use Obsidian** — it's a great way to:
- Browse what RedNode has learned in a human-friendly format
- Add notes that RedNode can query ("I moved the back garden camera to a new position")
- Review RedNode's daily reports and security findings
- Build a personal knowledge base that's both human and machine readable

**No need if you don't** — RedNode's built-in productivity agent (notes, tasks, journal) handles note-taking. Obsidian adds a richer UI and graph visualization, but it's optional.

---

## Implementation Status

| Feature | Status |
|---|---|
| Obsidian vault ingestion (→ RedNode) | 🟡 Planned — needs `kb.ingest` tool to scan vault directory |
| Knowledge export (RedNode →) | 🟡 Planned — needs `kb.export` to write Obsidian-format Markdown |
| Bidirectional sync | 🔴 Future — needs file watcher + change detection |
| Graph view from RedNode's knowledge graph | 🔴 Future — could export DOT/SVG or feed Obsidian's graph |
