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

- WorkGraph is the durable context graph, trigger plane, and coordination substrate for AI-native organizations.
- WorkGraph is not a generic agent runtime, generic workflow builder, generic task tracker, or generic memory layer.
- The context graph is first-class and typed. Wiki-links are one edge source, not the graph definition.
- The ledger is both audit trail and durable event stream.
- Triggers are core infrastructure, even when only contract-level behavior is implemented.
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

## Surface Expectations

- `workgraph schema` is the authoritative machine-readable discovery surface for CLI and primitive contracts.
- `workgraph capabilities` should explain workflows and primitive contracts to entering agents.
- `workgraph status` should expose graph hygiene and evidence gaps, not only counts.
- `workgraph show` should render coordination primitives in a way that makes their contracts obvious to humans and agents.

## Out Of Scope For This Foundation Pass

- live trigger execution loops
- webhook ingress runtime
- remote MCP/API server implementation
- approval workflow execution
- ergonomic nested authoring flows beyond direct markdown editing

Those are later layers. The foundation pass exists to make those future layers disciplined rather than improvised.
