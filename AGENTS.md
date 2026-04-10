# AGENTS.md — WorkGraph v4

This file is the contributor operating contract for future humans and agents. It is not the full product spec.

## Canonical Docs

Read these before changing semantics:

1. `docs/foundation.md`
2. `docs/context-graph.md`
3. `docs/operating-model.md`
4. `docs/roadmap.md`

If code and docs disagree, stop and reconcile them instead of guessing.

## Non-Negotiable Invariants

- WorkGraph is the durable system of record for agentic work: a typed context graph, immutable ledger, and evidence-bearing coordination layer for AI-native organizations.
- WorkGraph is not a generic agent runtime, generic workflow builder, generic task tracker, or generic memory layer.
- The context graph is first-class and typed. Wiki-links are one edge source, not the graph definition.
- The ledger is both audit trail and durable event stream.
- Triggers are core infrastructure, even when the current phase only yields durable planned follow-up actions rather than live execution.
- Threads are evidence-bearing coordination units, not chat logs or loose tasks.
- Missions coordinate related work. Runs capture one execution instance. Triggers yield planned follow-up actions.
- The actor model must scale to hundreds or thousands of actors while allowing opaque subactor lineages.
- Single-user and organizational modes are first-class product distinctions, not a cosmetic toggle.

## Implementation Priorities

Preserve this order when expanding the system:

1. Definition closure and documentation sync
2. Kernel hardening around graph, thread, mission, run, and trigger semantics
3. Trigger and event plane expansion
4. Remote MCP/API surfaces
5. Org-grade governance and approvals
6. Federation and distributed coordination

Do not jump ahead into flashy transport or runtime work while the semantic layer is still ambiguous.

## Repository Discipline

- Respect crate layering. Lower layers never import higher layers.
- Prefer small, factored modules over large “everything files.”
- Keep machine-readable outputs and human-readable outputs aligned to the same typed models.
- When you add or change primitive semantics, update both the Rust contracts and the canonical docs in the same turn.
- Avoid fuzzy naming. If a term matters to coordination, define it precisely.
- Treat graph edges, thread completion, lineage, and trigger semantics as durable contracts, not incidental implementation details.

## Surface Architecture Decision: CLI-first, MCP as Cloud Adapter

**Decided 2026-04-03 by Pedro + Clawdious.**

The CLI (`wg-cli`) is the **default interface** for all agents with shell access. The MCP server (`wg-mcp`) is the adapter for cloud/OAuth-only contexts (ChatGPT, cloud-hosted agents without shell).

**Rationale:**
- Agents with shell access already know how to exec binaries. Zero setup, zero auth.
- MCP adds overhead (server process, HTTP, connection lifecycle) unnecessary when you have a shell.
- Cloud agents can't exec binaries — MCP is their only path in.
- Same graph, different door. Both surfaces call the same kernel operations.

**Rules:**
1. `wg-cli` is the reference surface — every feature lands here first.
2. `wg-mcp` is a thin translation layer wrapping the same workspace ops.
3. Neither surface contains business logic — both are I/O adapters over the kernel.
4. MCP must never implement features unavailable via CLI.
5. Agent onboarding leads with CLI, mentions MCP as cloud alternative.

**Agent-friendly CLI requirements** (Cursor research, Eric Zakariasson March 2026):
- `--json` envelope on every command (`schema_version`, `success`, `result`, `next_actions`, `error`, `fix`)
- `--help` with examples on every subcommand
- Idempotent creates, `--dry-run` on writes
- Stdin support for pipelines
- Actionable errors with fix suggestions
- Predictable command structure
- Exit codes: 0 success, 1 error, 2 usage

## Surface Expectations

- `workgraph schema` is the authoritative machine-readable discovery surface for CLI and primitive contracts.
- `workgraph capabilities` should explain workflows and primitive contracts to entering agents.
- `workgraph status` should expose graph hygiene and evidence gaps, not only counts.
- `workgraph show` should render coordination primitives in a way that makes their contracts obvious to humans and agents.

## Out Of Scope For This Phase

- live trigger execution loops
- webhook ingress HTTP runtime
- remote MCP/API server implementation
- approval workflow execution
- ergonomic nested authoring flows beyond direct markdown editing

Those are later layers. The current trigger-plane expansion exists to make those future layers disciplined rather than improvised.
