# Operating Model

## Actor And Delegation Semantics

`actor` is the umbrella coordination concept.

An actor is any durable identity that WorkGraph can:

- assign work to
- attribute work to
- reason about across many threads, runs, and ledger events

The tracked actor primitives in this foundation pass are:

- `person`
- `agent`

`ActorId` is the stable logical actor identity used across ledger, runs, thread
activity, assignment, and lineage.

Tracked actors are durable accountability boundaries, not every runtime
boundary. Subactors may exist below the tracked boundary, and lineage may be
fully `tracked` or intentionally `opaque`.

Delegation should preserve meaning even when every subactor is not first-class
in the graph.

### Actor Contract v1

Actor Contract v1 exists to keep WorkGraph centered on durable accountability
rather than vendor-specific runtime internals.

An identity should be represented as a tracked actor only when it is durable
enough that WorkGraph needs to reason about it repeatedly over time.

Tracked actors should usually have some combination of:

- independent assignment or ownership of work
- repeated appearance across many threads or runs
- durable policy or approval relevance
- durable graph visibility beyond one session
- durable handoff or evidence accountability

Execution details are not automatically actors. By default, these remain
runtime/surface context, metadata, or external references:

- chat surfaces such as plain Claude chat or plain ChatGPT chat
- IDE surfaces such as Cursor or editor tabs
- runtime sessions
- spawned workers
- internal subagents
- workflow execution ids
- temporary background jobs

### Person As Actor

A `person` is a human actor.

Use `person` when:

- a human is the durable accountable party
- the human is directly performing or steering the work
- an AI surface is being used as a tool or assistant rather than as an
  independently delegated agent

Examples:

- Pedro brainstorming in Claude chat
- Pedro drafting an email with ChatGPT
- Pedro reviewing a report in an editor with AI assistance

In those cases, the actor is usually still the person. The AI product appears
as source or surface context, not automatically as a tracked agent actor.

### Agent As Actor

An `agent` is a non-human actor subtype representing a durable delegated
machine participant.

Use `agent` when the system is acting more like a durable delegate than a
one-off assistant surface.

Typical signals:

- it can take delegated work and execute with some autonomy
- it persists across many tasks or runs
- it has its own tools, permissions, or policies
- humans repeatedly reason about it as a distinct participant
- it produces durable handoffs, checkpoints, or evidence under its own identity

Examples:

- Claude Code working autonomously in a repository
- Hermes running scheduled or gateway-mediated tasks
- OpenClaw acting as a persistent personal or team agent
- a recurring internal research or review agent with its own role and policy

### Surface Is Not Actor

A surface, runtime, or tool is not automatically an actor.

Examples:

- `claude-chat`
- `chatgpt-chat`
- `cursor`
- `claude-code`
- `claude-cowork`
- `openclaw`
- `hermes`

These may appear as:

- run `source`
- external references
- runtime metadata

but they are not first-class actors unless WorkGraph intentionally tracks a
durable `agent` identity that uses them.

### Human-Led Versus Agent-Led Work

The same product may appear in different roles.

Human-led assistant use:

- actor is usually the `person`
- the AI product is a surface or tool

Delegated agentic use:

- actor may be a tracked `agent`
- the runtime or product remains surface context for the run

This keeps WorkGraph neutral about vendor branding while still distinguishing
between:

- a human using an AI surface
- a durable delegated agent acting on behalf of a person, team, or org

### Lineage Rules

In practice:

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

#### Run Contract v1

Run Contract v1 exists to keep runs useful across general work, agent work, and
adapter-mediated workflows without turning WorkGraph into a runtime log sink.

Minimum durable fields:

- `id` — stable run identifier
- `title` — human-readable label for the attempt
- `actor_id` — durable tracked actor responsible for the run
- `thread_id` — the thread this run belongs to
- `status` — lifecycle state of the run

Operational lifecycle fields:

- `started_at`
- `ended_at`
- `summary`
- `mission_id`
- `parent_run_id`
- `executor_id`

Optional integration and SDK fields:

- `kind` — broad run classification such as `agent_pass`, `review`, `approval`,
  `call`, or `automation_job`
- `source` — which surface created or observed the run receipt, such as
  `manual`, `sdk`, `cursor`, `calendar_adapter`, or `salesforce_adapter`
- `external_refs` — authoritative links back to external records such as a tool
  session, workflow execution, meeting, ticket, CRM record, or support case

SDKs and adapters should prefer emitting normalized run receipts over dumping
raw logs. A useful adapter-created run should say:

- who was responsible
- what thread the work belonged to
- what kind of bounded attempt happened
- when it started and ended, when known
- what authoritative external record can be followed for more detail

Adapters should create or update runs only for activities that are bounded,
attributable, and coordination-relevant. Not every click, note edit, email, or
background event deserves a run.

Recommended promotion rule:

- keep sessions, spawned workers, and internal subagents as opaque execution
  detail by default
- promote them into tracked actors only when they need independent assignment,
  policy, repeated graph visibility, or durable handoff accountability

This lets WorkGraph preserve durable delegation meaning while staying neutral
about the internal orchestration model of tools like Cursor, Claude Code, or
OpenClaw.

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
