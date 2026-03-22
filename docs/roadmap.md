# Roadmap

## Phase 1 — Foundation Lock

Goal:

- make the product boundary explicit
- lock down graph, actor, thread, run, trigger, and checkpoint definitions
- make canonical docs and code contracts agree

Success looks like:

- no fuzzy “context graph” language left in the repo
- durable coordination contracts in `wg-types`
- machine-readable discovery surfaces aware of those contracts

## Phase 2 — Kernel Hardening

Goal:

- make the kernel enforce the semantics, not merely describe them

Scope:

- evidence-aware thread completion
- mission and run persistence
- typed graph edge emission
- orientation and CLI surfaces for graph issues and evidence gaps

## Phase 3 — Trigger Plane

Goal:

- move from trigger schema to a fuller trigger/event core

Scope:

- richer event matching
- durable trigger subscriptions
- policy-aware action planning
- ingress from internal and external event sources

## Phase 4 — Remote Access Surfaces

Goal:

- expose the same durable contracts over remote interfaces

Scope:

- remote MCP
- API surfaces
- scoped auth and service accounts
- access boundaries that differ cleanly between single-user and org mode

## Phase 5 — Org-Grade Governance

Goal:

- add operational safety for multi-user and multi-agent organizations

Scope:

- approvals
- stronger policy enforcement
- durable operational guardrails
- richer audit workflows

## Phase 6 — Federation

Goal:

- support distributed coordination across multiple workspaces

Scope:

- cross-workspace signaling
- distributed delegation
- federated graph and trigger semantics

## Anti-Goal

Do not skip semantic closure in order to chase transport, adapters, or runtime novelty. The foundation is the leverage.
