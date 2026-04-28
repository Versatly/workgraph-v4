# Context Graph Specification

## Purpose

The WorkGraph context graph is the durable semantic substrate that connects organizational knowledge, coordination state, and event-aware operational context.

It is not merely a graph extracted from text. It is a typed graph built from durable primitives and durable relationship sources.

## Node Classes

Every persisted primitive is a graph node.

Important node classes include:

- Tier 1 knowledge primitives: `decision`, `pattern`, `lesson`, `policy`, `relationship`, `strategic_note`
- Tier 2 context primitives: `org`, `team`, `person`, `agent`, `client`, `project`
- Coordination primitives: `thread`, `mission`, `run`, `trigger`, `trigger_receipt`, `checkpoint`

External systems remain authoritative unless WorkGraph has an explicit reason to cache or model their state locally.

## Edge Kinds

The graph uses semantic edge kinds. Wiki-links are only one source.

- `reference` — loose or content-derived reference edges
- `relationship` — explicit semantic relationships emitted from relationship primitives
- `assignment` — actor-to-thread, actor-to-run, or equivalent ownership/assignment edges
- `containment` — mission-to-thread, thread-to-run, or mission-to-run structure
- `evidence` — evidence support edges from threads to supporting records
- `trigger` — trigger-rule edges to relevant targets, subjects, or action targets

Agent lineage references such as `parent_actor_id` and `root_actor_id` are part of the durable
assignment/lineage surface and should appear in graph-derived orientation outputs when present.
Runtime sessions, spawned workers, and other ephemeral execution details should appear as linked
context or external references when needed, not as automatic first-class graph nodes by default.

## Edge Provenance

Every edge records provenance so the system can distinguish soft references from stronger structured facts.

- `wiki_link`
- `field`
- `relationship_primitive`
- `evidence_record`
- `trigger_rule`

Edge provenance is part of the contract, not a debugging nicety.

## Company-Context Reference Semantics

Company-context primitives such as `org`, `team`, `person`, `agent`, `client`, and `project`
should expose durable references through canonical frontmatter fields, not only through prose.

- registry-declared reference fields are the source of truth for structured graph edges
- wiki-links remain supported in body text and string-valued frontmatter, but they are secondary
  provenance and should not replace typed company-context fields
- new generated references should prefer explicit `type/id` form to avoid ambiguous bare ids as the
  workspace grows
- status and browse surfaces should distinguish field-derived references from wiki-link mentions so
  agents can tell accountability and membership facts apart from loose mentions

## Graph Hygiene

Graph hygiene is a first-class operational output, not just a debugging helper.

Status surfaces should expose at least:

- broken structured references (with edge kind + provenance)
- orphan nodes (nodes with no inbound typed edges)
- evidence contract gaps on coordination threads

## Authority Rules

### What belongs in the graph

- durable organizational knowledge
- durable coordination state
- durable policy and decision constraints
- durable evidence and durable receipts of delegation
- durable work receipts such as runs and checkpoints

### What does not belong in the graph by default

- every raw external object
- transient runtime internals from other tools
- every session, subagent, or process emitted by an external runtime
- duplicated sources of truth when a compact external reference is enough

## Events And The Graph

Ledger events and normalized ingested events are graph-adjacent coordination facts.

They are not themselves the graph, but they are durable signals that:

- triggers subscribe to
- orientation surfaces summarize
- future automation planes react to

The graph holds state. The ledger records state change. Trigger receipts preserve durable event-to-plan outcomes. All three matter.

Durable primitive mutation paths should emit ledger events consistently so status, trigger
evaluation, trigger receipt persistence, and auditability observe the same state transitions.

Coordination-family writes should flow through explicit domain mutation services above the
audited store layer. Those services own operation-specific semantics, policy checks, audited
writes, and trigger hook integration so the graph, ledger, and trigger receipt plane observe one
coherent mutation contract per primitive family instead of scattered persistence helpers.

## Query Expectations

A useful WorkGraph query should be able to answer not only “what links to this?” but also:

- who owns this work?
- what mission or run contains it?
- what evidence supports completion?
- what durable relationship makes this relevant?
- what trigger rule reacts to this kind of change?
- what trigger receipts and pending planned actions were produced by recent events?

If the graph cannot answer those questions, the semantics are not yet first-class enough.
