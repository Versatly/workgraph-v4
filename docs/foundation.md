# WorkGraph Foundation

## Identity

WorkGraph is the durable context graph, trigger plane, and coordination substrate for AI-native organizations.

It exists to hold the organizational semantics that execution tools do not naturally preserve:

- decisions and why they were made
- patterns, policies, lessons, and relationships
- active coordination state across missions, threads, runs, triggers, and checkpoints
- durable actor lineage and delegation context
- durable event history and planned downstream actions

## Product Boundary

WorkGraph is:

- the organizational semantic layer
- the durable coordination layer
- the durable trigger evaluation substrate
- the graph and ledger that heterogeneous agents can consult and act through

WorkGraph is not:

- a generic agent runtime
- a general workflow automation builder
- a generic project manager
- a general-purpose memory database
- a replacement for execution tools such as Cursor, ChatGPT, Claude, OpenHands, OpenClaw, or shell runtimes

Those tools may execute work. WorkGraph exists to give that work context, continuity, governance, and durable handoff semantics.

## Strategic Position

The moat is not transport or hosting. The moat is governed organizational semantics with consequences.

Specifically:

- typed organizational primitives
- typed graph edges with provenance
- immutable ledger-backed history
- evidence-aware coordination
- policy-gated mutation surfaces for durable writes
- explicit domain mutation services per primitive family so semantic checks,
  policy, audited writes, and future hooks share one contract
- trigger evaluation over durable events
- cross-agent continuity through missions, threads, runs, checkpoints, and action plans

## Deployment Modes

### Single-user mode

Single-user mode is optimized for:

- one operator
- local-first storage
- lightweight auth
- low-governance usage
- local MCP and optional remote access

### Organizational mode

Organizational mode is optimized for:

- many humans and many agents
- scoped credentials and approvals
- stronger policy enforcement
- remote MCP/API access
- durable auditability and trigger isolation

These are different product shapes, not different skins over the same assumptions.

## Actor Model

Tracked actors are represented by stable logical `ActorId` values and usually materialized through `person` and `agent` primitives.

The system must assume:

- hundreds or thousands of actors
- delegation chains below the tracked boundary
- subactors that matter operationally without always needing first-class nodes

The durable contract is:

- `ActorId` remains the stable logical identity
- tracked actors are the durable accountability boundary, not every runtime boundary
- `agent` may declare `parent_actor_id`
- `agent` may declare `root_actor_id`
- lineage may be `tracked` or `opaque`

Opaque lineage means WorkGraph preserves delegation meaning without forcing every descendant actor into the graph.

That distinction matters because operational runtimes often create short-lived sessions,
spawned workers, and internal subagents. Those runtime descendants are not first-class
actors by default. WorkGraph should preserve them as execution context or external
references unless they become durably meaningful organizational participants.

In practice:

- a tracked actor is something WorkGraph can assign work to, attribute work to, and
  reason about across many threads or runs
- a runtime session is an execution detail, not automatically a new actor
- a spawned subagent may remain operationally opaque unless it needs independent
  identity, policy, assignment, or repeated graph visibility

This keeps the actor layer stable even when the underlying execution tools, session
boundaries, or internal orchestration patterns change.

## Surface Architecture: CLI-first

The CLI is the **primary interface** for all agents with shell access. It is the reference surface — every feature lands here first.

MCP and API are **secondary surfaces** for cloud-hosted agents that cannot exec a binary (ChatGPT plugins, OAuth-gated services, cloud-hosted agents without shell). They are thin translation layers over the same kernel operations the CLI uses. They must never implement features unavailable via CLI.

This was decided because:

- agents with shell access already know how to exec binaries — zero setup, zero auth
- MCP adds overhead (server process, HTTP, connection lifecycle) unnecessary when you have a shell
- cloud agents can't exec binaries — MCP is their only path in

All access surfaces (CLI, MCP, API) must preserve:

- the same typed primitive model
- the same graph semantics
- the same trigger semantics
- the same evidence and provenance requirements
- the same distinction between single-user and organizational modes

## Mutation Surface Contract

Durable writes are split into two layers on purpose:

- storage primitives validate structure, write markdown, and append ledger entries
- domain mutation services own primitive-family semantics, policy evaluation,
  audited persistence orchestration, and future hook points

`wg-store` remains the low-level audited persistence layer. Coordination and
other semantic families should expose explicit mutation services above it so
every create/update path flows through one durable contract instead of
re-implementing policy or audit behavior ad hoc.

## Codebase Discipline

The repository must stay easy for autonomous contributors to extend safely.

That means:

- code contracts and docs stay synchronized
- public surfaces are machine-readable and agent-legible
- layering stays explicit
- graph semantics remain operational, not poetic
- trigger semantics remain durable, not ad hoc
- thread completion remains evidence-aware, not a loose status toggle
