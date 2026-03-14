# WorkGraph v4

**The context graph and coordination daemon for AI-native companies.**

WorkGraph is the missing layer between agent runtimes (Cursor, Claude Code, Codex, OpenClaw) and company operations (projects, clients, decisions, policies). It stores what no other system captures — the institutional knowledge, decision traces, and connective tissue that makes autonomous agents productive.

## What It Does

- **Company context graph** — Org structure, clients, projects, decisions, patterns, lessons, policies — queryable by any agent via MCP
- **Thread coordination** — Agents and humans collaborate in threaded conversations with structured metadata
- **Trigger engine** — Events from any source (webhooks, email, cron) trigger agent actions with natural language instructions
- **Runtime-agnostic dispatch** — Same coordination layer works with Cursor, Claude Code, Codex, OpenClaw, or human workers
- **Local-first, markdown-native** — Every primitive is a markdown file. Human-readable, git-friendly, Obsidian-compatible.

## Architecture

```
┌──────────────────────────────────────────┐
│          CONTEXT GRAPH (truth)           │
│  decisions · patterns · lessons · people │
│  agents · clients · projects · policies  │
├──────────────────────────────────────────┤
│          COORDINATION (work)             │
│  threads · runs · missions · triggers    │
├──────────────────────────────────────────┤
│          SURFACES (access)               │
│  CLI · MCP · REST · gRPC · SSE          │
├──────────────────────────────────────────┤
│          EXECUTION (dispatch)            │
│  Cursor · Claude · Codex · Shell · HTTP │
└──────────────────────────────────────────┘
```

## Quick Start

```bash
# Initialize a workspace
workgraph init

# Add company context
workgraph create org --title "Versatly Holdings" --field mission="Deploy autonomous AI employees"
workgraph create client --title "Hale Pet Door" --field contact="Justin Dukes"
workgraph create agent --title "Clawdious" --field capabilities="coding,research,orchestration"

# Query
workgraph status
workgraph query client
workgraph brief --actor clawdious
```

## Building

```bash
cargo build --release
```

## License

Apache-2.0 OR MIT
