# WorkGraph v4

WorkGraph is the durable system of record for agentic work.

It combines a typed context graph, immutable ledger, and evidence-bearing coordination layer so humans and AI tools can share durable context, accountability, and handoff state across the same workspace.

Execution tools can do work, but they do not naturally preserve the durable organizational facts that make work reusable and governable over time: who owned it, what happened, why it mattered, what evidence supports completion, what policy applied, and what should happen next.

WorkGraph is not trying to replace Cursor, ChatGPT, Claude, OpenHands, OpenClaw, or other execution tools. It is the shared organizational system those tools should be able to consult and act through.

In this repo, "context graph" does not mean a fuzzy wiki-link memory map. It means a typed graph built from durable primitives and provenanced edges. Likewise, the current trigger substrate turns durable events into planned follow-up actions; live execution loops are a later layer.

## Why It Exists

WorkGraph exists for teams and operators whose work already spans humans, coding agents, chats, automations, and handoffs. Its job is to make that work:

- accountable - clear actor, ownership, and lineage
- resumable - future humans and agents inherit thread, run, and checkpoint state
- auditable - immutable ledger-backed history of durable writes
- evidence-backed - completion is validated against recorded criteria and evidence
- governable - policies and triggers act on durable coordination facts, not only chat logs

## Start Here

The canonical definition lives in `docs/`, not in scattered comments or one-off prompts:

- `docs/foundation.md` — what WorkGraph is, what it is not, and the product boundary
- `docs/context-graph.md` — first-class node, edge, provenance, and event semantics
- `docs/operating-model.md` — actor, mission, thread, run, trigger, checkpoint, and evidence semantics
- `docs/roadmap.md` — the disciplined execution order from foundation lock through federation
- `AGENTS.md` — contributor operating contract for future humans and agents

## What Exists In The Repo Today

The current workspace encodes that durable foundation rather than only describing it:

- markdown-native primitive storage with YAML frontmatter
- immutable ledger entries with hash-chain verification
- audited kernel writes for threads, missions, runs, triggers, checkpoints, and CLI-created primitives
- first-class thread, mission, run, trigger, checkpoint, and actor-lineage contracts in `wg-types`
- evidence-bearing thread persistence in `wg-thread`
- mission and run persistence in `wg-mission` and `wg-dispatch`, including mission planning/approval/validation states, milestone thread auto-creation, and run start/end timestamps
- typed graph edges in `wg-graph`, including assignment, containment, evidence, trigger, reference, actor-lineage, and trigger-receipt edges derived from durable coordination state
- orientation and CLI surfaces that expose evidence gaps, graph issues, coordination contracts, and full primitive discovery metadata
- trigger evaluation over normalized ledger, internal, and webhook event envelopes with durable `trigger_receipt` primitives for replay-safe planned follow-up actions

This turn still does not implement live trigger execution loops or webhook ingress runtime, but it now includes a **minimal hosted HTTP adapter** for CLI-based remote access and a **minimal MCP stdio adapter** for cloud/tool-hosted agents. These surfaces remain intentionally thin wrappers over the same CLI/kernel contracts rather than independent business-logic layers, and remote credentials are now **actor-bound** and **scope-bound** (`read`, `operate`, `admin`) so remote writes stay durably attributable instead of acting as generic bearer sessions.

CLI creation paths evaluate persisted policy primitives before writing. Trigger evaluation now records replay-safe `trigger_receipt` primitives and policy-aware planned action outcomes rather than auto-executed effects.

## Product Boundary

WorkGraph is:

- the durable system of record for organizational context and agentic work
- the coordination layer for missions, threads, runs, checkpoints, and evidence-backed handoffs
- the typed graph and immutable ledger that future humans and agents inherit
- the trigger substrate that evaluates durable events into planned actions

WorkGraph is not:

- a generic agent runtime
- a generic workflow builder
- a generic task tracker
- a generic memory database
- "just" an MCP server or API wrapper

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

## Tonight Agent Onboarding Flow

For one operator bootstrapping OpenClaw, Hermes, or another shell-capable agent:

```bash
workgraph onboard \
  --person-id person:pedro \
  --person-title "Pedro" \
  --org-title "Versatly" \
  --project-title "WorkGraph" \
  --agent agent:pedro-openclaw=openclaw \
  --agent agent:pedro-hermes=hermes

workgraph invite create \
  --label openclaw \
  --actor-id agent:pedro-openclaw \
  --server http://127.0.0.1:8787

workgraph invite create \
  --label hermes \
  --actor-id agent:pedro-hermes \
  --server http://127.0.0.1:8787

workgraph serve --listen 127.0.0.1:8787
```

Give each invited agent only its generated `workgraph connect ...` command. The hosted
server accepts all active invite credentials in `.workgraph/credentials.yaml`, binds each
request to the credential's actor, and enforces the credential's `read`, `operate`, or
`admin` scope. Revoke an invite with:

```bash
workgraph invite revoke openclaw
```

Agents should start with:

```bash
workgraph connect --server <server> --token <invite-token> --actor-id <actor-id>
workgraph brief --json
workgraph capabilities --json
```

This is a developer-hosted coordination surface, not org-grade credential management yet:
tokens are bearer credentials, stored only as hashes on the host, and rotation is performed
by revoking and recreating an invite.

## Quality Gate

```bash
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

See `CONTRIBUTING.md` for workflow rules and `AGENTS.md` for the contributor operating contract.
