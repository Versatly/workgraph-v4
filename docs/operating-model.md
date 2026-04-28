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

For durable actor and company-context authoring, the CLI schema is part of the
contract:

- `workgraph schema person` describes the canonical frontmatter for durable
  human actor records
- `workgraph schema agent` describes the canonical frontmatter for durable
  machine actor records
- `workgraph actor register` is the guided CLI for creating those actor
  records, while `workgraph create` remains the generic primitive authoring
  surface
- typed references such as `person/pedro`, `team/platform`, and
  `project/dealer-portal` are the preferred durable link format for new
  structured metadata and generated navigation output

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

### Surface / Runtime Contract v1

Surface / Runtime Contract v1 exists to separate:

- who did the work (`actor`)
- where the work was carried out (`surface` / `runtime`)
- how WorkGraph received the durable receipt (`integration path`)

Definitions:

- **surface** — the human-facing or agent-facing interface through which work was
  conducted, such as `claude-chat`, `chatgpt-chat`, `cursor`, `claude-code`,
  `claude-cowork`, `openclaw`, or `hermes`
- **runtime** — the execution environment commonly associated with a durable
  actor or run, such as a CLI tool, desktop app, gateway, VM, or hosted service
- **integration path** — how WorkGraph is reached from that environment, such as
  `cli`, `mcp`, `api`, or adapter-mediated receipt emission

Rules:

- surfaces and runtimes are execution context, not automatically tracked actors
- a durable `agent` actor may use one or more surfaces over time
- the same surface may be used in different modes, including human-led assistant
  use and delegated agentic execution
- WorkGraph should preserve surface and runtime context compactly without
  mirroring every internal session tree or transport detail

How to record this in the current foundation pass:

- use run `source` for the compact label of the surface or receipt source that
  created or observed the run
- use run `external_refs` for authoritative links to external conversations,
  tasks, workflows, tickets, sessions, or records
- use actor `runtime` for the common or default runtime associated with a
  durable tracked agent actor

Current integration-path guidance:

- use **CLI** as the reference path when the acting system has reliable shell
  access to `workgraph`
- use **MCP** when the acting system cannot reliably exec the CLI but can call
  remote or local tools through the protocol
- use **API** for remote programmatic contexts that are not best served by CLI
  or MCP
- use **adapter-emitted receipts** when WorkGraph is observing or importing
  meaningful work from another system rather than being called directly

Illustrative examples:

- Pedro using Claude chat interactively:
  - actor = `person/pedro`
  - run `source` = `claude-chat`
  - Claude is a surface, not automatically an actor
- Claude Code doing autonomous repository work:
  - actor = `agent/pedro-claude-code`
  - actor `runtime` may be `claude-code`
  - run `source` = `claude-code`
  - integration path may be `cli`
- Claude Cowork producing a recurring report:
  - actor may remain `person/pedro` when Pedro is directly steering it, or may
    become `agent/pedro-cowork` if it behaves as a durable delegated executor
  - run `source` = `claude-cowork`
  - integration path may be `mcp`
- ChatGPT app or deep research workflow:
  - actor may remain the person unless a durable delegated machine actor is
    intentionally modeled
  - run `source` = `chatgpt-chat` or another compact ChatGPT-related surface label
  - integration path may be `mcp` or `api`

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

## Modeling Guide v1

This guide translates the actor, surface/runtime, and run contracts into
concrete modeling choices.

### Scenario 1 - Human using Claude chat for brainstorming

Situation:

- Pedro opens Claude chat
- Pedro asks for help thinking through messaging
- Pedro remains the clear responsible worker

Recommended modeling:

- actor = `person/pedro`
- run `kind` = `research` or `drafting`, if the work session matters enough to
  record as a run
- run `source` = `claude-chat`
- Claude chat is a surface, not automatically a tracked `agent`

### Scenario 2 - Human using ChatGPT to draft an email

Situation:

- Pedro asks ChatGPT to improve a customer email
- Pedro reviews and sends it himself

Recommended modeling:

- actor = `person/pedro`
- run `source` = `chatgpt-chat`
- optional evidence or external reference = link to the draft or exported chat
- do not create an `agent` actor unless the system is durably delegated beyond
  one interactive chat

### Scenario 3 - Claude Code doing autonomous repository work

Situation:

- Pedro delegates implementation work to Claude Code
- Claude Code inspects files, runs commands, edits code, and reports back

Recommended modeling:

- actor = `agent/pedro-claude-code`
- actor `runtime` = `claude-code`
- run `source` = `claude-code`
- integration path = `cli` when the local environment can invoke `workgraph`
- subagents, sessions, and internal worktree details remain opaque unless they
  need repeated durable visibility

### Scenario 4 - Cursor assistant in human-led mode

Situation:

- Pedro is in Cursor
- Pedro remains actively steering the work
- AI suggestions help, but Pedro is the durable accountable worker

Recommended modeling:

- actor = `person/pedro`
- run `source` = `cursor`
- Cursor is the surface
- do not automatically create `agent/pedro-cursor`

### Scenario 5 - Cursor used as a durable delegated agent

Situation:

- Pedro relies on a recurring Cursor-based agent workflow
- the system repeatedly performs work under a stable delegated identity

Recommended modeling:

- actor = `agent/pedro-cursor`
- actor `runtime` = `cursor`
- run `source` = `cursor`
- integration path may be `cli` or `mcp` depending on how that environment
  reaches WorkGraph

The difference from Scenario 4 is not the brand name. The difference is whether
the system behaves as a durable delegated actor.

### Scenario 6 - Claude Cowork recurring task

Situation:

- Cowork prepares a recurring report from files and external tools
- in some cases Pedro actively steers each run
- in other cases Cowork behaves more like an ongoing delegate

Recommended modeling:

- if Pedro remains the direct responsible worker:
  - actor = `person/pedro`
  - run `source` = `claude-cowork`
- if Cowork behaves as a recurring delegated executor:
  - actor = `agent/pedro-cowork`
  - actor `runtime` = `claude-cowork`
  - run `source` = `claude-cowork`
- integration path may be `mcp`

### Scenario 7 - Hermes scheduled task

Situation:

- Hermes runs a scheduled weekly digest or recurring operational task
- it has stable identity, repeated behavior, and durable outputs

Recommended modeling:

- actor = `agent/pedro-hermes` or another durable Hermes-backed agent identity
- actor `runtime` = `hermes`
- run `kind` = `automation_job`
- run `source` = `hermes`
- integration path may be `cli`, `mcp`, `api`, or adapter-mediated receipt,
  depending on deployment

### Scenario 8 - OpenClaw with spawned subagents

Situation:

- OpenClaw handles work through its own session and subagent model
- internal descendants may appear during execution

Recommended modeling:

- top-level durable actor = `agent/pedro-openclaw` or another intentionally
  tracked OpenClaw actor
- run `source` = `openclaw`
- spawned descendants remain opaque by default
- only promote a subagent/session descendant into a tracked actor when it needs
  independent assignment, policy, repeated graph visibility, or durable handoff
  accountability

### Scenario 9 - Adapter observes a CRM workflow

Situation:

- no human or agent is directly calling WorkGraph
- a CRM adapter sees a meaningful event such as proposal sent or renewal review
  completed

Recommended modeling:

- create or update a run only if the event is bounded, attributable, and
  coordination-relevant
- actor may remain a tracked person, team-owned agent, or another durable actor
- run `source` = `salesforce_adapter`, `hubspot_adapter`, or similar compact
  label
- use `external_refs` to point to the authoritative CRM record
- do not mirror the entire CRM object graph into WorkGraph

### Scenario 10 - Human plus agent handoff

Situation:

- Pedro scopes work in chat
- a durable coding agent executes implementation
- Pedro validates the result

Recommended modeling:

- Run 1:
  - actor = `person/pedro`
  - run `source` = `claude-chat` or `chatgpt-chat`
  - purpose = scoping, drafting, or planning
- Run 2:
  - actor = `agent/pedro-claude-code`
  - run `source` = `claude-code`
  - parent linkage or thread continuity captures the delegation
- Run 3:
  - actor = `person/pedro`
  - run `source` = `cursor` or another review surface
  - purpose = validation or approval

This pattern is often better than collapsing all activity into one ambiguous run.

### Decision Checklist

When deciding how to model a new case, ask:

1. Who is the durable accountable identity for this work?
2. Is the AI product acting as a tool/surface or as a durable delegated actor?
3. What compact surface label best describes where the work happened?
4. What is the natural integration path: `cli`, `mcp`, `api`, or adapter
   receipt?
5. Which external references should be preserved instead of copied into
   WorkGraph?
6. Are any sessions or subagents durable enough to promote into tracked actors,
   or should they remain opaque?

### Trigger

A trigger is a durable rule that matches an event pattern and yields one or more action plans.

This foundation pass supports event source contracts for:

- `ledger`
- `webhook`
- `internal`

Phase 3 expands the substrate from ledger-only evaluation into a normalized event
plane. Ledger, internal, and externally ingested webhook-shaped events can all be
converted into the same event envelope, evaluated against active triggers, and
recorded as durable `trigger_receipt` primitives.

Those receipts are important because WorkGraph still does not auto-execute trigger
effects in this pass. Instead it durably records:

- which trigger matched
- which normalized event produced the match
- which planned follow-up actions were emitted
- which of those actions were suppressed by policy
- what replay-safe deduplication key prevents duplicate receipts for the same
  trigger/event pair

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

Planned update actions, completion actions, and trigger-emitted action plans are
durable follow-up intentions. They are not automatically executed in this
foundation pass.

## Surface Model

The CLI (`wg-cli`) is the primary agent interface. An agent on any machine with shell access runs `workgraph brief --json` and is immediately oriented.

MCP (`wg-mcp`) and API (`wg-api`) are secondary surfaces for cloud contexts. Both call the same kernel operations — neither owns business logic.

The current remote-access pass now includes a minimal hosted/dev shape for those
secondary surfaces:

- `workgraph onboard --person-id ... --person-title ... [--agent ...]` initializes a workspace and registers the operator plus first durable agent actors
- `workgraph invite create --label ... --actor-id ... --server ... [--access-scope ...]` creates an actor-bound hosted credential and prints the invited agent's connect command
- `workgraph serve --listen ...` starts a hosted HTTP adapter over one workspace using all active invite credentials
- `workgraph connect --server ... --token ... --actor-id ...` points a local CLI profile at that hosted workspace and verifies that the remote credential is bound to the requested actor
- `workgraph whoami` shows the active local or hosted actor identity
- `workgraph actor register|list|show` provides first-class actor registration and inspection
- `workgraph mcp serve --actor-id ... [--access-scope ...]` starts the MCP stdio adapter for tool-hosted/cloud agents with the same actor/scope contract

Those surfaces are intentionally thin and local/developer-oriented in this pass:

- one workspace per served process
- many active actor-bound hosted credentials per served process, stored as hashed bearer tokens in `.workgraph/credentials.yaml`
- bearer-token auth only on the hosted HTTP adapter
- three coarse remote access scopes: `read`, `operate`, and `admin`
- no org-grade service-account rotation or approval workflows yet
- no separate business logic path outside the CLI/kernel contracts

Remote governance contract in this pass:

- every hosted HTTP credential is bound to exactly one tracked actor
- hosted HTTP health and execution endpoints both require a valid bearer credential
- every MCP stdio session is bound to exactly one tracked actor
- `read` scope allows orientation and inspection commands only
- `operate` scope allows coordination writes such as claim, complete, checkpoint, and run lifecycle mutations
- `admin` scope is required for broad create flows, actor registration, and trigger administration
- remote callers may not impersonate a different actor than the credential/session binding
- `workgraph connect` validates the remote actor binding before persisting the hosted profile locally

Agent-facing CLI expectations:

- structured JSON envelope on every command (`--json`)
- `workgraph brief` for orientation (what is this workspace, what's here, what happened recently)
- `workgraph capabilities` for self-discovery (what commands exist, what they accept)
- `workgraph schema [type]` for field definitions
- idempotent creates, `--dry-run` on writes, stdin for pipelines
- actionable error messages with fix suggestions

Coordination commands now include:

- `workgraph onboard --person-id ... --person-title ... [--agent agent:id=runtime]` — bootstrap the local operator identity, optional seed work, and first durable agent actors
- `workgraph invite create|list|revoke ...` — manage actor-bound hosted invite credentials for agents such as OpenClaw and Hermes
- `workgraph connect --server ... --token ... --actor-id ...` — bind a CLI profile to a hosted workspace after validating the actor-bound remote credential
- `workgraph whoami` — show the effective actor and hosted/local execution mode
- `workgraph serve --listen ...` — expose one workspace over the hosted HTTP adapter using all active invite credentials
- `workgraph actor register --type ... --id ... --title ...` — register a durable person or agent actor
- `workgraph actor list|show ...` — inspect durable actor registrations
- `workgraph claim <thread-id>` — claim and activate a thread
- `workgraph complete <thread-id>` — complete a thread with evidence validation
- `workgraph run create --title ... --thread-id ...` — create a bounded run receipt for one thread
- `workgraph run start|complete|fail|cancel <run-id>` — transition run lifecycle state
- `workgraph checkpoint --working-on ... --focus ...` — persist working context
- `workgraph ledger [--last N]` — inspect recent immutable ledger entries
- `workgraph trigger validate <trigger-id>` — validate one persisted trigger
- `workgraph trigger replay [--last N]` — replay recent ledger entries through the trigger plane
- `workgraph trigger ingest --source ... --event-id ... --event-name ...` — ingest a normalized internal or webhook-shaped event through the CLI
- `workgraph mcp serve --actor-id ... [--access-scope ...]` — expose the CLI-first command surface as an actor-bound scoped MCP stdio session

`workgraph status` also surfaces graph hygiene (`graph_issues`, `orphan_nodes`)
and thread evidence gaps in both human and JSON output. Phase 3 also adds trigger
health, recent trigger receipts, and pending trigger action counts to those
orientation surfaces.

## Delegation And Handoff

Durable delegation should preserve:

- who requested work
- what thread or run the work belongs to
- what actor lineage produced the result
- what evidence came back
- what follow-up actions remain

Agent lineage fields such as `parent_actor_id` and `root_actor_id` are durable graph-visible coordination facts, even when descendant execution remains operationally opaque.

Future transport, MCP, API, and trigger layers must preserve these semantics instead of collapsing them into generic task execution.