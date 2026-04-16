# Cortex Platform Plan for OpenClaw Adapter

## Purpose

This document explains what the Cortex platform must provide for a native
OpenClaw adapter plugin to work reliably.

The target outcome is:

- OpenClaw sessions can call Cortex for context, collaboration, and clarification
- Cortex can route follow-up events back to the correct live OpenClaw session
- human replies, agent handoffs, checkpoints, and recent activity stay durable
- the integration is push-oriented where possible, not based on blind polling

This plan assumes:

- Cortex is the system that owns the solo vault, actor registry, shared threads,
  runs, checkpoints, recent activity, and clarification workflow
- OpenClaw remains an execution/runtime environment with its own sessions,
  tools, hooks, and routing model
- the adapter plugin is the translation layer between OpenClaw runtime semantics
  and Cortex semantics

## Why Cortex needs a dedicated OpenClaw adapter

OpenClaw is not just "an agent with a tool call surface." It has:

- isolated sessions
- agent-specific workspaces
- built-in session tools
- subagent orchestration
- deterministic channel routing
- plugin hooks with session-aware context

Because of this, Cortex should not treat OpenClaw as a generic webhook source or
generic MCP client. Cortex must understand:

- which OpenClaw agent made the request
- which live session the request belongs to
- what workspace and route context the session is using
- whether the request belongs to a thread, run, or checkpoint already known to
  Cortex

Without that, Cortex cannot target the correct session when sending follow-up
events such as clarification answers or collaboration updates.

## Official OpenClaw documentation this plan depends on

The adapter design in this document is anchored on the following official
OpenClaw docs:

1. `Session Management`
   - https://docs.openclaw.ai/concepts/session.md
   - Confirms that OpenClaw organizes conversations into sessions, stores all
     session state in the gateway, and persists session metadata and transcripts
     under `~/.openclaw/agents/<agentId>/sessions/`
   - Confirms that routing determines which agent and which session bucket are
     used for incoming messages

2. `Session Tools`
   - https://docs.openclaw.ai/concepts/session-tool.md
   - Documents `sessions_list`, `sessions_history`, `sessions_send`,
     `sessions_spawn`, `sessions_yield`, `subagents`, and `session_status`
   - This is the most important page for the adapter because it shows how
     OpenClaw already models cross-session work and non-blocking subagent
     orchestration
   - `sessions_yield` is especially important because it provides a native way
     to end the current turn and wait for follow-up results instead of building
     polling loops

3. `Channels & routing`
   - https://docs.openclaw.ai/channels/channel-routing.md
   - Defines `SessionKey` as the bucket key used to store context and control
     concurrency
   - Documents the session-key shapes for DMs, channels, groups, and threads
   - Confirms that reply routing is deterministic and host-controlled
   - Confirms that the matched agent determines which workspace and session
     store are used

4. `Plugin SDK Overview`
   - https://docs.openclaw.ai/plugins/sdk-overview.md
   - Defines the typed plugin contract
   - Shows that plugins can register tools, hooks, HTTP routes, gateway methods,
     CLI subcommands, and services
   - Documents the registration API and focused plugin SDK imports

5. `Plugin Runtime Helpers`
   - https://docs.openclaw.ai/plugins/sdk-runtime.md
   - Documents `api.runtime.agent`, `api.runtime.subagent`,
     `api.runtime.taskFlow`, and `api.runtime.events`
   - Confirms the plugin can resolve agent identity/workspace, access the
     session store, launch/wait on subagents, bind Task Flows to trusted session
     keys, and listen for agent/session events

6. `Building Plugins`
   - https://docs.openclaw.ai/plugins/building-plugins.md
   - Confirms that a tool/hook plugin is a valid native extension type
   - Documents `definePluginEntry`, `api.registerTool`, `api.registerHook`,
     `api.registerHttpRoute`, and `api.registerService`

7. `Plugin Setup and Config`
   - https://docs.openclaw.ai/plugins/sdk-setup.md
   - Documents required `package.json` `openclaw` metadata and
     `openclaw.plugin.json`
   - Confirms how plugin config is validated before runtime loads

8. `Plugin Manifest`
   - https://docs.openclaw.ai/plugins/manifest.md
   - Documents the native `openclaw.plugin.json` contract
   - Important because the plugin's configuration and capability ownership need
     to be visible to OpenClaw before Cortex-specific runtime code executes

## Core design principle

For OpenClaw, Cortex must route follow-up events by **session**, not only by
actor.

The primary target identity should be:

- `sessionKey`

The secondary routing metadata should include:

- `agentId`
- `sessionId`
- `requesterOrigin`
- `threadId`
- `runId`
- `checkpointId`
- `workspaceDir`

This follows directly from the OpenClaw docs:

- `SessionKey` is the documented bucket key for context/concurrency
- session tools accept session key or session id
- channel routing and completion delivery preserve route context where available

## What Cortex platform must provide

### 1. Actor registry

Cortex must maintain a durable actor registry that can map OpenClaw agents to
Cortex actors.

Minimum fields:

- `actorId`
- `kind` (`person` or `agent`)
- `title`
- `description`
- `runtime` (for OpenClaw-backed actors: `"openclaw"`)
- `ownerUserId`
- `status`
- `capabilities`
- `externalRefs`

OpenClaw-specific mapping fields:

- `openclawAgentId`
- `defaultWorkspaceHint`
- `defaultSessionVisibility`

Why Cortex needs this:

- OpenClaw may have multiple durable agents in one solo-owned system
- agents should know about other agents
- threads, runs, checkpoints, and activity must be attributable to real actors
- recent agent activity should be actor-centric, not just session-centric

### 2. Session binding registry

Cortex must maintain a durable binding layer between OpenClaw runtime sessions
and Cortex coordination objects.

Suggested record:

- `bindingId`
- `integration = "openclaw"`
- `actorId`
- `openclawAgentId`
- `sessionKey`
- `sessionId` (optional)
- `requesterOrigin` (optional but strongly preferred)
- `workspaceDir`
- `threadId`
- `runId`
- `checkpointId` (optional)
- `channelRouteSnapshot` (optional)
- `lastSeenAt`
- `status`

This binding registry is what allows Cortex to answer:

- which Cortex thread is this session currently working in?
- which run should receive activity updates?
- where should a clarification answer be routed back?
- is this session still alive?

### 3. Thread registry and collaboration model

Cortex must provide a first-class thread model for OpenClaw-backed
collaboration.

Minimum thread capabilities:

- create thread
- bind session to thread
- list active threads for an actor
- append thread updates
- attach notes/primitives/sources
- link thread to runs and checkpoints
- expose thread state to agents and UI

OpenClaw agents need this because the product goal is not just "one person with
helpers." It is a solo-owned multi-actor system where agents can collaborate in
shared threads.

### 4. Run registry

Cortex must maintain run records for bounded execution attempts.

OpenClaw calls into Cortex should either:

- open a run explicitly, or
- be able to attach to an existing run bound to the current session

Minimum run fields:

- `runId`
- `threadId`
- `actorId`
- `executor` metadata
- `status`
- `startedAt`
- `endedAt`
- `parentRunId` (optional)
- `sessionBindingId`

### 5. Checkpoint registry

Cortex must support checkpoints as first-class durable resumability objects.

Minimum fields:

- `checkpointId`
- `threadId`
- `runId`
- `actorId`
- `title`
- `workingOn`
- `focus`
- `nextActions`
- `relatedNoteIds`
- `createdAt`

This is important because OpenClaw already has `sessions_yield`, subagent
orchestration, and long-lived session workflows. Cortex should provide the
durable resumability layer on top of those session primitives.

### 6. Recent activity and recent agent activity

Cortex must maintain durable activity records, not just raw logs.

Minimum activity fields:

- `activityId`
- `actorId`
- `threadId` (optional)
- `runId` (optional)
- `sessionKey`
- `kind`
- `title`
- `summary`
- `references`
- `createdAt`

Good `kind` examples:

- `note.updated`
- `primitive.created`
- `primitive.updated`
- `source.ingested`
- `clarification.requested`
- `clarification.answered`
- `thread.bound`
- `checkpoint.created`
- `run.started`
- `run.completed`
- `handoff.sent`

This activity stream is what the product uses to show:

- recent agent activity
- recent work by thread
- recent changes tied to a specific OpenClaw session

### 7. Clarification workflow

Cortex must support a structured clarification workflow.

Minimum data model:

- `clarificationRequest`
  - `clarificationId`
  - `threadId`
  - `runId`
  - `actorId`
  - `sessionKey`
  - `question`
  - `status`
  - `createdAt`

- `clarificationResponse`
  - `clarificationId`
  - `responderActorId`
  - `responseText`
  - `capturedAt`
  - `sourceChannel`

- `clarificationResolution`
  - `clarificationId`
  - `threadId`
  - `sessionTarget`
  - `normalizedAnswer`
  - `resumePolicy`

The key requirement is that clarification requests must carry enough session
metadata that Cortex can later target the correct OpenClaw session when a human
or another agent responds.

### 8. Event bus / trigger delivery

Cortex must expose a real event bus for push-style delivery.

Why:

- the goal is to avoid polling
- OpenClaw already has a native notion of yielding and follow-up completion
- Cortex needs to deliver resolved clarification and collaboration events back
  into the right session at the right time

Minimum event types:

- `clarification.requested`
- `clarification.answered`
- `thread.updated`
- `run.started`
- `run.completed`
- `checkpoint.created`
- `handoff.ready`
- `activity.appended`

Recommended transport:

- SSE first
- WebSocket if needed later

Recommended event targeting filter:

- by `integration`
- by `actorId`
- by `openclawAgentId`
- by `sessionKey`
- by `threadId`

### 9. Subscription API for the plugin

The OpenClaw adapter plugin should not force Cortex to make inbound calls into
OpenClaw. Instead, Cortex should expose a plugin-friendly subscription API.

Recommended pattern:

- plugin authenticates to Cortex
- plugin subscribes to events for the session keys it currently owns or has
  registered
- Cortex streams only matching events
- plugin turns those into OpenClaw-native follow-up actions

This is safer and simpler than asking Cortex to directly "reach into" OpenClaw.

### 10. OpenClaw-aware API surface

Cortex should expose a dedicated OpenClaw integration contract rather than only
generic endpoints.

Suggested API surfaces:

- `POST /integrations/openclaw/session-bind`
- `POST /integrations/openclaw/thread-bind`
- `POST /integrations/openclaw/activity`
- `POST /integrations/openclaw/checkpoint`
- `POST /integrations/openclaw/clarifications`
- `GET /integrations/openclaw/events/stream`

This lets Cortex evolve OpenClaw-specific behavior without polluting the
general-purpose product API.

## Recommended OpenClaw adapter plugin shape

### Plugin type

The adapter should be a **tool + hook plugin**.

Why not only a channel plugin:

- the main problem is runtime/session binding, not just messaging
- the adapter needs tools and hooks much more than it needs a new inbound chat
  channel

### Plugin responsibilities

1. attach trusted session metadata on every Cortex interaction
2. bind OpenClaw sessions to Cortex threads/runs
3. emit activity updates into Cortex
4. subscribe to Cortex events
5. route follow-up events back into the correct live session

### Plugin responsibilities supported by docs

This follows directly from:

- `Building Plugins` (`api.registerTool`, `api.registerHook`,
  `api.registerService`, `api.registerHttpRoute`)
- `Plugin SDK Overview` registration API
- `Plugin Runtime Helpers` (`api.runtime.agent`, `api.runtime.subagent`,
  `api.runtime.taskFlow`, `api.runtime.events`)

## Adapter plugin modules

Suggested internal module layout:

- `binding/`
  - OpenClaw session -> Cortex binding logic
- `transport/`
  - Cortex API client + SSE subscriber
- `tools/`
  - Cortex-facing agent tools
- `hooks/`
  - lifecycle hooks
- `activity/`
  - activity projection and publishing
- `routing/`
  - session-targeted resume delivery
- `config/`
  - plugin config schema and manifest support

## Minimum plugin tools

1. `cortex_context_search`
2. `cortex_thread_bind`
3. `cortex_thread_update`
4. `cortex_checkpoint_create`
5. `cortex_recent_activity`
6. `cortex_actor_registry`
7. `cortex_clarification_request`

Every tool execution should attach:

- `sessionKey`
- `agentId`
- `workspaceDir`
- `requesterOrigin` if available
- `threadId` / `runId` if already bound

## Minimum hooks

1. **session bootstrap hook**
   - create or refresh Cortex session binding
   - discover actor identity
   - discover workspace

2. **activity hook**
   - append meaningful activity to Cortex

3. **yield / handoff hook**
   - create checkpoint before waiting on a follow-up event

4. **tool-result hook**
   - keep Cortex thread/run linkage in sync

## Trigger-based resume design

### Goal

When Cortex has a new answer or update, it should reach the correct OpenClaw
session without polling.

### Recommended flow

1. OpenClaw tool/plugin creates a clarification request in Cortex and includes
   trusted session metadata.
2. OpenClaw session yields or continues as appropriate.
3. Human or another actor answers through Cortex.
4. Cortex emits `clarification.answered` targeted to the bound `sessionKey`.
5. Plugin subscription loop receives the event.
6. Plugin routes the event into the correct OpenClaw session using its
   session-aware integration path.

### Why this matches OpenClaw

This aligns with the official `Session Tools` doc:

- `sessions_yield` is specifically intended to end the current turn and wait
  for follow-up results
- `sessions_send` and `sessions_spawn` already encode cross-session
  coordination patterns

In other words: Cortex should not invent a polling loop when OpenClaw already
has a native waiting/handoff model.

## Session targeting policy

### Primary target

- `sessionKey`

### Secondary validation fields

- `agentId`
- `sessionId`
- `requesterOrigin`
- `threadId`
- `runId`

### Why this matters

From `Channels & routing`, the session key shape already encodes:

- agent
- channel
- peer/group/channel identity
- optional thread/topic identity

That means `sessionKey` is the most information-rich and stable route target
for Cortex replies.

## Security and trust model

The plugin must never let raw untrusted user input bind arbitrary OpenClaw
session keys to Cortex threads.

This follows the warning in `Plugin Runtime Helpers`:

- use `api.runtime.taskFlow.bindSession({ sessionKey, requesterOrigin })` only
  when you already have a **trusted** session key
- do not bind from raw user input

Recommended trust rules:

- plugin derives `sessionKey` from OpenClaw runtime/tool context only
- plugin sends signed/authenticated requests to Cortex
- Cortex verifies plugin identity and actor/session ownership before accepting
  bindings

## Recommended Cortex-side milestones

### Phase 1: Binding and metadata correctness

Build:

- actor registry
- session binding registry
- thread and run binding
- clarification data model
- basic OpenClaw integration endpoints

Acceptance:

- plugin can create a clarification request with correct session metadata
- Cortex can inspect a binding and know exactly which session to target later

### Phase 2: Activity and checkpoints

Build:

- activity model
- recent activity API
- checkpoint model
- plugin activity publishing

Acceptance:

- recent agent activity is visible in Cortex
- OpenClaw work becomes durable and attributable

### Phase 3: Trigger delivery

Build:

- SSE event stream
- event filters by `sessionKey`
- plugin subscription loop
- clarification answer -> session-targeted resume flow

Acceptance:

- no polling required for clarification responses

### Phase 4: Rich collaboration

Build:

- agent-to-agent collaboration thread flows
- session handoff support
- richer run/checkpoint transitions
- better actor-aware recent activity surfaces

Acceptance:

- multiple OpenClaw agents can collaborate through Cortex-backed threads with
  durable visibility

## What Cortex does not need in v1

To make the OpenClaw adapter work, Cortex does **not** need:

- org/team tenancy
- vector-first retrieval
- full dispatch/autonomy runtime
- plugin-owned execution of OpenClaw itself
- direct mutation of OpenClaw internal stores

It needs:

- clean actor/session/thread/run/checkpoint contracts
- event delivery
- trusted binding
- recent activity

## Final recommendation

For the OpenClaw adapter to work reliably, Cortex platform must provide:

1. a durable actor registry
2. a durable OpenClaw session binding registry
3. shared threads, runs, and checkpoints
4. structured recent activity and recent agent activity
5. clarification workflow objects
6. an event bus with session-targeted trigger delivery
7. an OpenClaw-specific integration contract that treats `sessionKey` as the
   primary callback target

The OpenClaw adapter plugin should then be implemented as a native tool + hook
plugin that uses the official OpenClaw session/runtime/plugin surfaces to
translate between OpenClaw live sessions and Cortex durable coordination state.
