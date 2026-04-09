# Operating Model

## Actor And Delegation Semantics

- `person` and `agent` are the primary tracked actor primitives
- `ActorId` is the stable logical actor identity used across ledger, runs, and thread activity
- tracked actors are durable accountability boundaries, not every runtime boundary
- subactors may exist below the tracked boundary
- lineage may be fully `tracked` or intentionally `opaque`

Delegation should preserve meaning even when every subactor is not first-class in the graph.

In practice:

- a tracked actor is something WorkGraph can assign work to, attribute work to,
  and reason about across many threads or runs
- a tracked actor should usually reflect a durable organizational identity or
  role, not a single tool session identifier
- runtime sessions, spawned workers, and internal subagents are execution
  details by default, not automatically first-class actors
- those descendants may remain opaque unless they need independent policy,
  assignment, repeated graph visibility, or durable handoff semantics

This keeps WorkGraph focused on durable coordination while allowing runtimes to
use their own internal orchestration models.

## Primitive Semantics

### Mission

A mission is a coordinated objective. It may require many threads and many runs.

Mission is not synonymous with task.

Mission lifecycle is explicit and durable:

- `draft` — mission shell exists
- `planned` — milestones are declared and milestone threads are auto-created
- `approved` — plan is accepted and ready to start
- `active` — execution in progress
- `validating` — completion readiness is being checked
- `completed` / `cancelled` — terminal outcomes

Mission planning is evidence-bearing by structure: milestones carry durable
`thread_id` bindings so completion and progress are evaluated over concrete
thread state rather than inferred from narrative text.

### Thread

A thread is a scoped coordination workstream around a concrete problem or slice of work.

Threads carry:

- lifecycle status
- assigned actor
- parent mission
- exit criteria
- evidence items
- planned update actions
- planned completion actions
- conversation log

### Run

A run is one bounded execution attempt or work session on behalf of a thread.

Runs are not limited to software execution. A run may represent:

- an agent pass
- a human work session
- an automation job
- a review step
- an approval pass
- an external tool-mediated action that matters to coordination

A run is the durable receipt that some actor attempted concrete work on a
thread. It records:

- which thread the work belonged to
- which tracked actor was responsible
- which tracked executor performed it when that differs from the responsible actor
- when the attempt started and ended
- what the outcome was
- whether it was delegated from another run

Run rules:

- one run belongs to exactly one thread
- one run may optionally reference a mission
- one run may optionally reference a parent run
- logical actor and concrete executor may differ
- `started_at` and `ended_at` are durable lifecycle timestamps

Execution-specific details such as sessions, spawned subagents, job ids, or
adapter event ids may be linked as metadata or external references without
forcing every runtime descendant to become a first-class actor.

`parent_run_id` preserves durable delegation meaning without requiring
WorkGraph to mirror an orchestrator's full internal execution tree.

By default, WorkGraph should capture the coordination receipt, summary, and
evidence-bearing outputs of a run rather than every raw runtime log line.

### Trigger

A trigger is a durable rule that matches an event pattern and yields one or more action plans.

This foundation pass supports event source contracts for:

- `ledger`
- `webhook`
- `internal`

Concrete matching is implemented for ledger events in this pass. The other sources are part of the schema contract but not yet live runtime surfaces.

Kernel and CLI mutation paths append durable ledger entries for persisted coordination changes so trigger evaluation can observe real thread, mission, run, trigger, and checkpoint state transitions.
Those mutation paths should flow through primitive-family domain mutation services that own lifecycle semantics, policy checks, audited writes, and future trigger hooks rather than composing store writes ad hoc at call sites.

### Checkpoint

A checkpoint is a durable saved working context that helps future humans or agents resume work without reconstructing local state from scratch.

## Thread Completion Contract

A thread is complete only when every required exit criterion is satisfied by recorded evidence.

That means:

- exit criteria are explicit
- evidence references the criteria it satisfies
- completion is validated, not assumed

Planned update actions and completion actions are durable follow-up intentions. They are not automatically executed in this foundation pass.

## Surface Model

The CLI (`wg-cli`) is the primary agent interface. An agent on any machine with shell access runs `workgraph brief --json` and is immediately oriented.

MCP (`wg-mcp`) and API (`wg-api`) are secondary surfaces for cloud contexts. Both call the same kernel operations — neither owns business logic.

Agent-facing CLI expectations:

- structured JSON envelope on every command (`--json`)
- `workgraph brief` for orientation (what is this workspace, what's here, what happened recently)
- `workgraph capabilities` for self-discovery (what commands exist, what they accept)
- `workgraph schema [type]` for field definitions
- idempotent creates, `--dry-run` on writes, stdin for pipelines
- actionable error messages with fix suggestions

Coordination commands now include:

- `workgraph claim <thread-id>` — claim and activate a thread
- `workgraph complete <thread-id>` — complete a thread with evidence validation
- `workgraph checkpoint --working-on ... --focus ...` — persist working context
- `workgraph ledger [--last N]` — inspect recent immutable ledger entries

`workgraph status` also surfaces graph hygiene (`graph_issues`, `orphan_nodes`)
and thread evidence gaps in both human and JSON output.

## Delegation And Handoff

Durable delegation should preserve:

- who requested work
- what thread or run the work belongs to
- what actor lineage produced the result
- what evidence came back
- what follow-up actions remain

Agent lineage fields such as `parent_actor_id` and `root_actor_id` are durable graph-visible coordination facts, even when descendant execution remains operationally opaque.

Future transport, MCP, API, and trigger layers must preserve these semantics instead of collapsing them into generic task execution.
