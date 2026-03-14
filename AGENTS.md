# AGENTS.md — WorkGraph v4

## What This Is

WorkGraph v4 is a **context graph and coordination daemon** for AI-native companies, written in Rust.

It is NOT a task tracker. It is NOT a knowledge base. It is the **nervous system** of a company where humans and AI agents collaborate — the single place where institutional knowledge lives, decisions are recorded, work is coordinated, and agents communicate.

## Core Architecture: Context Graph

WorkGraph stores what no other system captures — the connective tissue between systems.

### Three-Tier Data Model

**Tier 1 — Lives ONLY in WorkGraph (unique value, no other system has this)**
- `decision` — rationale, alternatives considered, who decided, when, outcome
- `pattern` — repeatable process, steps, exceptions, when to use it
- `lesson` — what we learned, from what experience, applies to what
- `policy` — rule, scope, enforcement, exceptions, who can override
- `relationship` — between any two entities, nature, context (not just "X works at Y" but "X is the decision-maker, prefers email, always ask about the dealer portal first")
- `strategic_note` — long-term context, market position, vision

**Tier 2 — Cached snapshot + external link (orient without API calls)**
- `org` — company identity, structure + links to external systems
- `team` — group of people/agents, responsibilities
- `person` — human profile + links to CRM/LinkedIn/email
- `agent` — AI agent, capabilities, location, connection info
- `client` — customer context + links to CRM/billing
- `project` — work container, status + links to Linear/GitHub

**Tier 3 — Pure external references (never stored locally)**
- Links to GitHub PRs, Shopify orders, Linear issues, emails, etc.
- Stored as `external_ref` fields on Tier 1/2 primitives

### Why This Matters
A new AI agent connects via MCP and calls `workgraph brief`. It immediately knows:
- What company it's in, who the clients are, who's on the team
- What patterns to follow, what lessons to remember
- What decisions were recently made that affect its work
- What's assigned to it

No reading 10 docs. No asking "who are our clients?" The graph IS the company brain.

## Storage Model

**Markdown-native, filesystem-first.**
- Each primitive is a markdown file with YAML frontmatter
- Directory-per-type layout: `decisions/`, `patterns/`, `agents/`, `clients/`, etc.
- Frontmatter = structured fields. Body = rich content, rationale, notes.
- Human-readable, git-friendly, Obsidian-compatible
- No database required for local-first mode

Example decision primitive:
```markdown
---
type: decision
id: rust-for-workgraph-v4
title: "Rust for WorkGraph v4"
status: decided
decided_by: pedro
decided_at: 2026-03-13
participants: [pedro, clawdious]
tags: [architecture, language-choice]
links:
  - provider: github
    kind: repo
    url: "https://github.com/Versatly/workgraph-v4"
  - provider: telegram
    kind: conversation
    url: "telegram:2026-03-13"
---

## Context
WorkGraph v3 (TypeScript) hit ceiling at 45K lines across 16 packages.

## Alternatives
1. Stay TypeScript — rejected: can't do single binary, GC pauses
2. Go — rejected: worse type system
3. Rust — chosen: single binary, zero-cost abstractions

## Rationale
Infrastructure-grade product needs infrastructure-grade language.

## Consequences
- ~3 month rewrite
- Must maintain workspace format compatibility during migration
```

## Ledger

Every mutation to the graph produces an immutable ledger entry with a hash chain:
```
{ ts, actor, op, primitive_type, primitive_id, fields_changed, hash, prev_hash }
```
The ledger is the audit trail and the event bus. Triggers subscribe to ledger events.

## Repo Structure

Rust workspace monorepo. `crates/` directory. `wg-` prefix for all crates.

### Layers (strict: lower never imports higher)

**Layer 0 — Foundation (zero domain knowledge)**
- `wg-types` — All type definitions (primitives, ledger, events, config, identity)
- `wg-error` — Error types with codes
- `wg-paths` — Path abstractions (workspace paths, store paths)
- `wg-fs` — Filesystem utilities (atomic write, temp files)
- `wg-encoding` — Serde helpers, markdown frontmatter parse/write, YAML
- `wg-clock` — Time abstraction (real clock + test/mock clock)

**Layer 1 — Kernel (domain truth, pure logic)**
- `wg-store` — Primitive CRUD (markdown ↔ struct), query engine, validation
- `wg-ledger` — Append-only ledger, hash chain, integrity verification
- `wg-registry` — Primitive type registry (built-in + user-defined types)
- `wg-thread` — Thread lifecycle + conversation model
- `wg-mission` — Mission orchestration, decomposition
- `wg-graph` — Wiki-link graph, traversal, hygiene
- `wg-policy` — Policy engine, gates, party-based auth
- `wg-orientation` — Status, brief, checkpoint, context lenses

**Layer 2 — Execution (dispatch, adapters, triggers)**
- `wg-dispatch` — Run lifecycle, orchestration
- `wg-adapter-api` — RuntimeAdapter trait (no implementations)
- `wg-adapter-cursor` — Cursor Background Agents
- `wg-adapter-claude` — Claude Code
- `wg-adapter-shell` — Generic shell subprocess
- `wg-adapter-webhook` — HTTP webhook
- `wg-trigger` — Trigger engine, event matching, NL instruction model
- `wg-autonomy` — Autonomy daemon, self-healing loops
- `wg-connector-api` — EventSource + Reconciler traits
- `wg-connector-github` — GitHub webhook + API reconciliation

**Layer 3 — Transport (network, sync)**
- `wg-transport` — Event transport: outbox, inbox, dead-letter
- `wg-federation` — Cross-workspace federation
- `wg-net` — Tailscale peer discovery, networking
- `wg-signal` — Agent-to-agent signaling

**Layer 4 — Surface (CLI, API, MCP)**
- `wg-cli` — CLI commands + output formatters
- `wg-mcp` — MCP server (stdio + HTTP)
- `wg-api` — REST (axum) + gRPC (tonic) + SSE + webhook gateway
- `wg-projections` — Read-model projections for operator UX
- `wg-markdown` — Markdown projection writer (Obsidian compat)

**Layer 5 — Integration (optional)**
- `wg-obsidian` — Obsidian vault sync
- `wg-otel` — OpenTelemetry instrumentation

**Binary**
- `bins/workgraph/` — Main binary, wires everything

### Dependency Rules
- Layer N can only import from Layer 0..N-1
- `wg-types` has ZERO dependencies except serde/chrono
- Kernel crates never import adapter/transport/surface crates
- Adapters depend on `wg-adapter-api`, not on each other

## What To Build NOW (Phase 0)

### Priority 1: Perfect Scaffolding
- Cargo workspace with ALL crates stubbed (Cargo.toml + lib.rs with doc comment)
- Workspace-level dependency management in root Cargo.toml
- `rust-toolchain.toml` (edition 2024, MSRV 1.85)
- `.cargo/config.toml` with build profiles
- `deny.toml` for cargo-deny (license/advisory checking)
- `clippy.toml` with strict lints
- `.github/workflows/ci.yml` — cargo test + clippy + fmt + deny
- `.github/workflows/release.yml` — cross-compile for Linux/macOS/Windows
- README.md with architecture overview
- CONTRIBUTING.md

### Priority 2: Foundation Crates (implement fully)
- `wg-types` — All type definitions:
  - Primitive types (PrimitiveType, FieldDefinition, Registry)
  - Ledger types (LedgerEntry, LedgerOp)
  - Identity types (ActorId, WorkspaceId, NodeId)
  - Config types (WorkgraphConfig)
  - Three-tier model types (ExternalRef, CachedSnapshot)
  - Company context types (Org, Team, Person, Agent, Client, Project)
  - Higher-order types (Decision, Pattern, Lesson, Policy, Relationship)
- `wg-error` — Error enum with thiserror, error codes
- `wg-paths` — WorkspacePath, StorePath, LedgerPath
- `wg-fs` — atomic_write, ensure_dir, temp_dir
- `wg-encoding` — parse_frontmatter, write_frontmatter, serde helpers
- `wg-clock` — Clock trait + RealClock + MockClock

### Priority 3: Core Kernel (implement fully)
- `wg-store` — read_primitive, write_primitive, list_primitives, query
- `wg-ledger` — append, read_from_cursor, verify_chain
- `wg-registry` — register_type, get_type, list_types, built-in type registration

### Priority 4: CLI Skeleton
- `wg-cli` — clap-based CLI with subcommands:
  - `workgraph init` — initialize a new workspace
  - `workgraph status` — show workspace overview
  - `workgraph query <type> [--filter field=value]` — query primitives
  - `workgraph create <type> --title "..." [--field key=value]` — create primitive
  - `workgraph show <type>/<id>` — show single primitive
- `bins/workgraph/` — main.rs that wires wg-cli

### What NOT To Build Yet
- Adapters (just stub the trait in wg-adapter-api)
- Triggers (just stub in wg-trigger)
- MCP/API servers (just stub)
- Federation/transport (just stub)
- Thread conversation model (just stub wg-thread with lifecycle only)

## Key Dependencies

```toml
[workspace.dependencies]
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
serde_yaml = "0.9"
clap = { version = "4", features = ["derive", "env"] }
thiserror = "2"
anyhow = "1"
chrono = { version = "0.4", features = ["serde"] }
sha2 = "0.10"
tracing = "0.1"
tracing-subscriber = "0.3"
uuid = { version = "1", features = ["v4", "serde"] }
pulldown-cmark = "0.12"
tempfile = "3"
walkdir = "2"
glob = "0.3"
```

## Quality Standards

- `cargo clippy -- -D warnings` must pass
- `cargo fmt --check` must pass
- Every public function has a doc comment
- Every crate has a module-level doc comment explaining what it owns
- Tests for all core logic (store read/write, ledger append/verify, registry)
- Use `#[cfg(test)]` modules, not separate test files, for unit tests
- Integration tests in `tests/` at workspace root
- Surface crates must be split by responsibility before they become large:
  - `wg-cli` should prefer `args.rs`, `app.rs`, `commands/`, `output/`, and `util/`
  - `wg-mcp` should prefer `server/`, `resources/`, `tools/`, and transport-specific modules
  - `wg-api` should prefer route modules, service modules, and serialization modules
- Do not allow command or surface crates to collapse into a single large file once multiple commands or renderers exist

## Design Principles

1. **Small crates, clear boundaries** — if a crate is growing past 1000 lines, consider splitting
2. **Traits for extension points** — adapters, connectors, storage backends
3. **No magic** — explicit wiring in main.rs, no hidden initialization
4. **Errors are values** — thiserror for library crates, anyhow for binary
5. **Test with real files** — tempdir-based tests, not mocks, for store/ledger
6. **Markdown is the API** — if a human can't read the workspace in a text editor, something is wrong
7. **Surface crates mirror agent experience** — CLI, MCP, and API should evolve toward shared workspace loading, orientation, and output semantics rather than bespoke one-off logic
8. **Split early, not late** — once a file owns parsing + orchestration + rendering + helpers, it is already too large
