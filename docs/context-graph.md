# Context Graph Specification

## Purpose

The WorkGraph context graph is the durable semantic substrate that connects organizational knowledge, coordination state, and event-aware operational context.

It is not merely a graph extracted from text. It is a typed graph built from durable primitives and durable relationship sources.

## Node Classes

Every persisted primitive is a graph node.

Important node classes include:

- Tier 1 knowledge primitives: `decision`, `pattern`, `lesson`, `policy`, `relationship`, `strategic_note`
- Tier 2 context primitives: `org`, `team`, `person`, `agent`, `client`, `project`
- Coordination primitives: `thread`, `mission`, `run`, `trigger`, `checkpoint`

External systems remain authoritative unless WorkGraph has an explicit reason to cache or model their state locally.

## Edge Kinds

The graph uses semantic edge kinds. Wiki-links are only one source.

- `reference` — loose or content-derived reference edges
- `relationship` — explicit semantic relationships emitted from relationship primitives
- `assignment` — actor-to-thread, actor-to-run, or equivalent ownership/assignment edges
- `containment` — mission-to-thread, thread-to-run, or mission-to-run structure
- `evidence` — evidence support edges from threads to supporting records
- `trigger` — trigger-rule edges to relevant targets or action targets

Agent lineage references such as `parent_actor_id` and `root_actor_id` are part of the durable
assignment/lineage surface and should appear in graph-derived orientation outputs when present.

## Edge Provenance

Every edge records provenance so the system can distinguish soft references from stronger structured facts.

- `wiki_link`
- `field`
- `relationship_primitive`
- `evidence_record`
- `trigger_rule`

Edge provenance is part of the contract, not a debugging nicety.

## Authority Rules

### What belongs in the graph

- durable organizational knowledge
- durable coordination state
- durable policy and decision constraints
- durable evidence and durable receipts of delegation

### What does not belong in the graph by default

- every raw external object
- transient runtime internals from other tools
- duplicated sources of truth when a compact external reference is enough

## Events And The Graph

Ledger events are graph-adjacent coordination facts.

They are not themselves the graph, but they are durable signals that:

- triggers subscribe to
- orientation surfaces summarize
- future automation planes react to

The graph holds state. The ledger records state change. Both matter.

Durable primitive mutation paths should emit ledger events consistently so status, trigger
evaluation, and auditability observe the same state transitions.

## Query Expectations

A useful WorkGraph query should be able to answer not only “what links to this?” but also:

- who owns this work?
- what mission or run contains it?
- what evidence supports completion?
- what durable relationship makes this relevant?
- what trigger rule reacts to this kind of change?

If the graph cannot answer those questions, the semantics are not yet first-class enough.
