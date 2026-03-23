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
- `agent` may declare `parent_actor_id`
- `agent` may declare `root_actor_id`
- lineage may be `tracked` or `opaque`

Opaque lineage means WorkGraph preserves delegation meaning without forcing every descendant actor into the graph.

## Remote Access Principles

Remote MCP and API access are expected future surfaces, but they are not the product boundary. They are access paths into the durable contracts defined here.

Those future surfaces must preserve:

- the same typed primitive model
- the same graph semantics
- the same trigger semantics
- the same evidence and provenance requirements
- the same distinction between single-user and organizational modes

## Codebase Discipline

The repository must stay easy for autonomous contributors to extend safely.

That means:

- code contracts and docs stay synchronized
- public surfaces are machine-readable and agent-legible
- layering stays explicit
- graph semantics remain operational, not poetic
- trigger semantics remain durable, not ad hoc
- thread completion remains evidence-aware, not a loose status toggle
