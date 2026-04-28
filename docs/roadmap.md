# Roadmap

## Phase 1 - Foundation Lock

Goal:

- make the product boundary explicit
- lock down graph, actor, thread, run, trigger, and checkpoint definitions
- make canonical docs and code contracts agree
- make the CLI fully agent-friendly as the primary interface surface

Success looks like:

- no hand-wavy "context graph" language left in the repo; the term always means a typed graph with durable primitives, semantic edges, and provenance
- durable coordination contracts in `wg-types`
- machine-readable discovery surfaces aware of those contracts
- CLI passes agent-friendly audit: `--json` envelopes, `brief`, `capabilities`, `schema`, examples in `--help`, idempotent creates, `--dry-run`, stdin support, actionable errors

## Phase 2 - Kernel Hardening

Goal:

- make the kernel enforce the semantics, not merely describe them

Scope:

- evidence-aware thread completion
- mission and run persistence
- typed graph edge emission
- orientation and CLI surfaces for graph issues and evidence gaps

## Phase 3 - Trigger Plane

Goal:

- move from trigger schema to a fuller trigger/event core

Scope:

- richer event matching
- durable trigger subscriptions
- policy-aware action planning
- ingress from internal and external event sources
- durable trigger receipts and replay-safe deduplication
- CLI-first trigger validation, replay, ingest, and status surfaces

## Phase 4 — Remote Access Surfaces (MCP as Cloud Adapter)

Goal:

- expose the same durable contracts over remote interfaces for agents without shell access

The CLI is the primary interface (decided Phase 1). MCP and API are secondary surfaces for cloud-hosted agents, ChatGPT plugins, OAuth-gated services, and contexts where an agent cannot exec a binary.

MCP and API must be thin translation layers over the same kernel operations the CLI uses. They must never implement features unavailable via CLI.

Scope:

- remote MCP server (`wg-mcp`) wrapping kernel ops
- HTTP API (`wg-api`) for REST consumers
- scoped auth and service accounts
- access boundaries that differ cleanly between single-user and org mode

Current delivered floor in this repo:

- first-run `workgraph onboard` for creating the operator actor, optional initial
  agents, and seed work primitives
- actor-bound hosted invite credentials via `workgraph invite create|list|revoke`
- `workgraph serve` over all active hosted invite credentials for one workspace
- actor-bound MCP stdio sessions for `workgraph mcp serve`
- coarse remote access scopes (`read`, `operate`, `admin`) enforced before remote command execution
- `workgraph connect` validation that a hosted credential matches the requested actor identity

## Phase 5 - Org-Grade Governance

Goal:

- add operational safety for multi-user and multi-agent organizations

Scope:

- approvals
- stronger policy enforcement
- durable operational guardrails
- richer audit workflows

## Phase 6 - Federation

Goal:

- support distributed coordination across multiple workspaces

Scope:

- cross-workspace signaling
- distributed delegation
- federated graph and trigger semantics

## Anti-Goal

Do not skip semantic closure in order to chase transport, adapters, or runtime novelty. The foundation is the leverage.
