# WorkGraph v4

WorkGraph is the durable context graph, trigger plane, and coordination substrate for AI-native organizations.

It is not trying to replace Cursor, ChatGPT, Claude, OpenHands, OpenClaw, or other execution tools. It is the shared organizational system those tools should be able to consult and act through.

## Start Here

The canonical definition lives in `docs/`, not in scattered comments or one-off prompts:

- `docs/foundation.md` — what WorkGraph is, what it is not, and the product boundary
- `docs/context-graph.md` — first-class node, edge, provenance, and event semantics
- `docs/operating-model.md` — actor, mission, thread, run, trigger, checkpoint, and evidence semantics
- `docs/roadmap.md` — the disciplined execution order from foundation lock through federation
- `AGENTS.md` — contributor operating contract for future humans and agents

## MVP Scope

The MVP for WorkGraph is the disciplined completion of roadmap phases 1-4:

- durable semantic contracts for knowledge, coordination, graph, and ledger primitives
- a CLI-first operator/agent surface with machine-readable discovery and write workflows
- evidence-aware coordination over threads, missions, runs, triggers, and checkpoints
- trigger evaluation that yields durable action plans rather than generic automation glue
- thin remote MCP/API adapters over the same kernel operations used by the CLI

The MVP does **not** include:

- live autonomous trigger execution loops
- webhook ingress runtime
- approval workflow execution
- federation / cross-workspace distributed coordination

Those remain post-MVP roadmap work.

## What Exists In The Repo Today

The current workspace encodes the durable MVP foundation rather than only describing it:

- markdown-native primitive storage with YAML frontmatter
- immutable ledger entries with hash-chain verification
- audited kernel writes for threads, missions, runs, triggers, checkpoints, and CLI-created primitives
- first-class thread, mission, run, trigger, checkpoint, and actor-lineage contracts in `wg-types`
- evidence-bearing thread persistence in `wg-thread`
- mission and run persistence in `wg-mission` and `wg-dispatch`
- typed graph edges in `wg-graph`, including assignment, containment, evidence, trigger, reference, and actor-lineage edges derived from agent metadata
- orientation and CLI surfaces that expose evidence gaps, graph issues, coordination contracts, and primitive discovery metadata
- agent-friendly CLI contract features including JSON envelopes, `--dry-run`, idempotent create behavior, stdin body input, examples in help output, and actionable errors
- first-class CLI workflows for thread, mission, run, trigger, and checkpoint mutations
- thin remote adapters in `wg-mcp` and `wg-api` that translate remote requests into the same CLI/kernel-backed operations

Live trigger execution loops, webhook ingress, approval execution, and federation are intentionally still outside the MVP boundary.

CLI creation paths evaluate persisted policy primitives before writing. Trigger action plans remain durable planned follow-up actions rather than auto-executed effects.

## Product Boundary

WorkGraph is:

- the durable record of organizational context
- the coordination system for missions, threads, runs, checkpoints, and handoffs
- the trigger substrate that evaluates durable events into planned actions
- the graph and ledger that future agents inherit

WorkGraph is not:

- a generic agent runtime
- a generic workflow builder
- a generic task tracker
- a generic memory database
- “just” an MCP server

## Repository Shape

```text
Layer 0  Foundation  -> wg-types, wg-error, wg-paths, wg-fs, wg-encoding, wg-clock
Layer 1  Kernel      -> wg-store, wg-ledger, wg-registry, wg-thread, wg-mission, wg-graph, wg-policy, wg-orientation
Layer 2  Execution   -> wg-dispatch, adapters, triggers, connectors
Layer 3  Transport   -> transport, federation, networking, signaling
Layer 4  Surface     -> CLI, MCP, API, projections, markdown views
Layer 5  Integration -> optional integrations
```

Lower layers may not depend on higher layers.

## Quick Start

```bash
cargo build --release
./target/release/workgraph init
./target/release/workgraph --json brief --lens workspace
./target/release/workgraph --json status
./target/release/workgraph --json capabilities
./target/release/workgraph --json schema
./target/release/workgraph --json create org --title Versatly --field summary="AI-native company"
printf 'Mission objective' | ./target/release/workgraph --json create mission --title "Launch mission" --stdin-body
```

## Quality Gate

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

See `CONTRIBUTING.md` for workflow rules and `AGENTS.md` for the contributor operating contract.
