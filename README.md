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

## What Exists In The Repo Today

The current workspace encodes the durable foundation rather than only describing it:

- markdown-native primitive storage with YAML frontmatter
- immutable ledger entries with hash-chain verification
- first-class thread, mission, run, trigger, checkpoint, and actor-lineage contracts in `wg-types`
- evidence-bearing thread persistence in `wg-thread`
- mission and run persistence in `wg-mission` and `wg-dispatch`
- typed graph edges in `wg-graph`, including assignment, containment, evidence, trigger, and reference edges
- orientation and CLI surfaces that expose evidence gaps, graph issues, and coordination contracts

This turn does not implement live trigger execution loops, webhook ingress, remote MCP, or API runtime surfaces yet. It establishes the durable contracts those surfaces must honor.

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
./target/release/workgraph brief --lens workspace
./target/release/workgraph status
./target/release/workgraph capabilities
./target/release/workgraph schema
```

## Quality Gate

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

See `CONTRIBUTING.md` for workflow rules and `AGENTS.md` for the contributor operating contract.
