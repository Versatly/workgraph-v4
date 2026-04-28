//! Structured discovery metadata for agent-native CLI use.

use serde::Serialize;
use wg_types::Registry;

/// Structured command capabilities that agents can use for CLI self-discovery.
#[derive(Debug, Clone, Serialize)]
pub struct CapabilitiesCatalog {
    /// The first command an entering agent should run.
    pub first_command: String,
    /// Supported command contracts.
    pub commands: Vec<CommandCapability>,
}

/// Structured metadata for one CLI command.
#[derive(Debug, Clone, Serialize)]
pub struct CommandCapability {
    /// Stable command name.
    pub name: String,
    /// Human-readable summary of intent.
    pub description: String,
    /// Required positional arguments.
    pub required_args: Vec<String>,
    /// Optional command and global flags.
    pub flags: Vec<String>,
    /// Canonical example invocations.
    pub examples: Vec<String>,
}

/// Structured schema metadata for CLI output envelopes and primitive field contracts.
#[derive(Debug, Clone, Serialize)]
pub struct CliSchema {
    /// Stable schema version for machine-readable envelopes.
    pub schema_version: String,
    /// Description of envelope fields emitted by all commands in JSON mode.
    pub envelope_fields: Vec<SchemaField>,
    /// Primitive type schemas available for create/query operations.
    pub primitive_types: Vec<PrimitiveTypeSchema>,
}

/// A field inside a structured result envelope.
#[derive(Debug, Clone, Serialize)]
pub struct SchemaField {
    /// Stable field name.
    pub name: String,
    /// Logical field type.
    pub field_type: String,
    /// Human-readable description.
    pub description: String,
    /// Whether the field is always present.
    pub required: bool,
}

/// Primitive type schema returned by `workgraph schema`.
#[derive(Debug, Clone, Serialize)]
pub struct PrimitiveTypeSchema {
    /// Primitive type name.
    pub name: String,
    /// On-disk directory for this primitive type.
    pub directory: String,
    /// Human-readable primitive description.
    pub description: String,
    /// Valid field definitions for create/query use.
    pub fields: Vec<PrimitiveFieldSchema>,
}

/// Primitive field schema contract.
#[derive(Debug, Clone, Serialize)]
pub struct PrimitiveFieldSchema {
    /// Stable field name.
    pub name: String,
    /// Logical field type.
    pub field_type: String,
    /// Human-readable field description.
    pub description: String,
    /// Whether this field is required at creation time.
    pub required: bool,
    /// Whether the field accepts repeated values.
    pub repeated: bool,
    /// Query behavior supported for this field.
    pub query_behavior: String,
    /// Allowed primitive target types when this field stores durable references.
    pub reference_types: Vec<String>,
    /// Typed graph edge emitted when the reference resolves.
    pub graph_edge_kind: Option<String>,
}

/// Returns the static CLI capabilities catalog.
#[must_use]
pub fn capabilities_catalog() -> CapabilitiesCatalog {
    let global_flags = ["--json", "--format json"];
    CapabilitiesCatalog {
        first_command: "workgraph brief --json".to_owned(),
        commands: vec![
            capability(
                "onboard",
                "Initialize a workspace and register the operator plus optional first agents.",
                vec![],
                &[
                    global_flags[0],
                    global_flags[1],
                    "--person-id <actor-id>",
                    "--person-title \"<name>\"",
                    "--org-title \"<title>\"",
                    "--project-title \"<title>\"",
                    "--agent <actor-id>=<runtime>",
                ],
                vec![
                    "workgraph onboard --person-id person:pedro --person-title \"Pedro\" --org-title \"Versatly\" --json",
                    "workgraph onboard --person-id person:pedro --person-title \"Pedro\" --agent agent:pedro-openclaw=openclaw --agent agent:pedro-hermes=hermes",
                ],
            ),
            capability(
                "connect",
                "Connect this CLI profile to a hosted WorkGraph workspace using an actor-bound scoped credential.",
                vec![],
                &[
                    global_flags[0],
                    global_flags[1],
                    "--server <url>",
                    "--actor-id <actor-id>",
                    "--token <token>",
                ],
                vec![
                    "workgraph connect --server http://127.0.0.1:8787 --actor-id agent:cursor --token dev-token --json",
                    "workgraph connect --server https://wg.example.com --actor-id person:pedro --token prod-token",
                ],
            ),
            capability(
                "whoami",
                "Show the active local or hosted CLI connection identity.",
                vec![],
                &global_flags,
                vec!["workgraph whoami --json", "workgraph whoami"],
            ),
            capability(
                "actor register",
                "Register a durable person or agent actor against the active workspace.",
                vec![],
                &[
                    global_flags[0],
                    global_flags[1],
                    "--type <person|agent>",
                    "--id <actor-id>",
                    "--title \"<title>\"",
                    "--role <role>",
                    "--team-id <team-ref>",
                    "--tag <tag>",
                    "--owner <actor-ref>",
                    "--runtime <runtime>",
                    "--capability <capability>",
                ],
                vec![
                    "workgraph actor register --type person --id person:pedro --title \"Pedro\" --role \"Founder\" --team-id team/platform --json",
                    "workgraph actor register --type agent --id agent:cursor --title \"Cursor Agent\" --runtime cursor --owner person/pedro --capability coding",
                ],
            ),
            capability(
                "invite create",
                "Create an actor-bound hosted invite credential and print the invited agent's connect command.",
                vec![],
                &[
                    global_flags[0],
                    global_flags[1],
                    "--label <label>",
                    "--actor-id <actor-id>",
                    "--server <url>",
                    "--access-scope <read|operate|admin>",
                ],
                vec![
                    "workgraph invite create --label openclaw --actor-id agent:pedro-openclaw --server http://127.0.0.1:8787 --json",
                    "workgraph invite create --label hermes --actor-id agent:pedro-hermes --server https://wg.example.com --access-scope operate",
                ],
            ),
            capability(
                "invite list",
                "List hosted invite credentials without revealing raw tokens.",
                vec![],
                &global_flags,
                vec!["workgraph invite list --json", "workgraph invite list"],
            ),
            capability(
                "invite revoke",
                "Revoke one hosted invite credential by label or id.",
                vec!["<label-or-id>"],
                &global_flags,
                vec![
                    "workgraph invite revoke openclaw --json",
                    "workgraph invite revoke invite-openclaw",
                ],
            ),
            capability(
                "actor list",
                "List registered person and agent actors in the active workspace.",
                vec![],
                &global_flags,
                vec!["workgraph actor list --json", "workgraph actor list"],
            ),
            capability(
                "init",
                "Initialize registry, config, ledger, and primitive directories.",
                vec![],
                &global_flags,
                vec!["workgraph init --json", "workgraph init"],
            ),
            capability(
                "brief",
                "Return workspace identity, primitive counts, recent ledger activity, and orientation cues.",
                vec![],
                &[
                    global_flags[0],
                    global_flags[1],
                    "--lens <workspace|delivery|policy|agents>",
                ],
                vec![
                    "workgraph brief --json",
                    "workgraph brief --lens workspace --json",
                    "workgraph brief --lens delivery",
                ],
            ),
            capability(
                "status",
                "Show graph hygiene, evidence gaps, primitive counts, and recent activity.",
                vec![],
                &global_flags,
                vec!["workgraph status --json", "workgraph status"],
            ),
            capability(
                "claim",
                "Claim a thread for the configured actor and mark it active.",
                vec!["<thread-id>"],
                &global_flags,
                vec![
                    "workgraph claim thread-1 --json",
                    "workgraph claim launch-scoping",
                ],
            ),
            capability(
                "complete",
                "Complete a thread after validating required evidence coverage.",
                vec!["<thread-id>"],
                &global_flags,
                vec![
                    "workgraph complete thread-1 --json",
                    "workgraph complete launch-verification",
                ],
            ),
            capability(
                "checkpoint",
                "Save a durable working-context checkpoint for handoff and resume.",
                vec![],
                &[
                    global_flags[0],
                    global_flags[1],
                    "--working-on \"<work item>\"",
                    "--focus \"<focus>\"",
                ],
                vec![
                    "workgraph checkpoint --working-on \"Kernel hardening\" --focus \"Finish tests\" --json",
                    "workgraph checkpoint --working-on \"Phase 2\" --focus \"Evidence gaps\"",
                ],
            ),
            capability(
                "ledger",
                "View recent immutable ledger entries.",
                vec![],
                &[global_flags[0], global_flags[1], "--last <n>"],
                vec![
                    "workgraph ledger --json",
                    "workgraph ledger --last 20",
                    "workgraph ledger --last 5 --json",
                ],
            ),
            capability(
                "run create",
                "Create a queued run bound to a thread, with actor override and dry-run preview support.",
                vec![],
                &[
                    global_flags[0],
                    global_flags[1],
                    "--title \"<title>\"",
                    "--thread-id <thread-id>",
                    "--actor-id <actor-id>",
                    "--kind <kind>",
                    "--source <source>",
                    "--executor-id <executor-id>",
                    "--mission-id <mission-id>",
                    "--parent-run-id <run-id>",
                    "--summary \"<summary>\"",
                    "--dry-run",
                ],
                vec![
                    "workgraph run create --title \"Cursor pass\" --thread-id thread-1 --json",
                    "workgraph run create --title \"Review pass\" --thread-id thread-1 --actor-id agent:reviewer --kind review",
                    "workgraph run create --title \"Preview run\" --thread-id thread-1 --dry-run --json",
                ],
            ),
            capability(
                "run start",
                "Mark a queued run as running.",
                vec!["<run-id>"],
                &global_flags,
                vec![
                    "workgraph run start cursor-pass --json",
                    "workgraph run start review-pass",
                ],
            ),
            capability(
                "run complete",
                "Mark a run as succeeded and optionally persist a final summary.",
                vec!["<run-id>"],
                &[global_flags[0], global_flags[1], "--summary \"<summary>\""],
                vec![
                    "workgraph run complete cursor-pass --json",
                    "workgraph run complete cursor-pass --summary \"Delivered final patch\"",
                ],
            ),
            capability(
                "run fail",
                "Mark a run as failed and optionally persist a failure summary.",
                vec!["<run-id>"],
                &[global_flags[0], global_flags[1], "--summary \"<summary>\""],
                vec![
                    "workgraph run fail cursor-pass --json",
                    "workgraph run fail cursor-pass --summary \"Blocked by missing dependency\"",
                ],
            ),
            capability(
                "run cancel",
                "Mark a run as cancelled and optionally persist a cancellation summary.",
                vec!["<run-id>"],
                &[global_flags[0], global_flags[1], "--summary \"<summary>\""],
                vec![
                    "workgraph run cancel cursor-pass --json",
                    "workgraph run cancel cursor-pass --summary \"Superseded by newer run\"",
                ],
            ),
            capability(
                "trigger validate",
                "Validate a trigger definition by reference against the normalized event-plane contract.",
                vec!["<trigger-ref>"],
                &global_flags,
                vec![
                    "workgraph trigger validate trigger/thread-done --json",
                    "workgraph trigger validate trigger/thread-done",
                ],
            ),
            capability(
                "trigger replay",
                "Replay recent ledger entries through the trigger plane and persist durable trigger receipts.",
                vec![],
                &[global_flags[0], global_flags[1], "--last <n>"],
                vec![
                    "workgraph trigger replay --json",
                    "workgraph trigger replay --last 20",
                ],
            ),
            capability(
                "trigger ingest",
                "Ingest one normalized internal or webhook event payload into the trigger plane.",
                vec![],
                &[
                    global_flags[0],
                    global_flags[1],
                    "--source <ledger|internal|webhook>",
                    "--event-id <event-id>",
                    "--event-name <event-name>",
                    "--provider <provider>",
                    "--subject <type/id>",
                    "--field key=value",
                ],
                vec![
                    "workgraph trigger ingest --source internal --event-id event-1 --event-name handoff.ready --subject thread/thread-1 --json",
                    "workgraph trigger ingest --source webhook --event-id gh-123 --event-name pull_request.merged --provider github --subject project/dealer-portal",
                ],
            ),
            capability(
                "capabilities",
                "List command contracts for autonomous self-discovery.",
                vec![],
                &global_flags,
                vec!["workgraph capabilities --json", "workgraph capabilities"],
            ),
            capability(
                "schema",
                "Show primitive field definitions for one type or all types.",
                vec![],
                &[global_flags[0], global_flags[1], "[type]"],
                vec![
                    "workgraph schema --json",
                    "workgraph schema org --json",
                    "workgraph schema",
                ],
            ),
            capability(
                "create",
                "Create primitives, support idempotent no-op writes, dry-run previews, and stdin payloads.",
                vec!["<type>"],
                &[
                    global_flags[0],
                    global_flags[1],
                    "--title \"<title>\"",
                    "--field key=value",
                    "--dry-run",
                    "--stdin",
                ],
                vec![
                    "workgraph create org --title \"Versatly\" --json",
                    "workgraph create decision --title \"Use Rust\" --field status=decided --json",
                    "workgraph create person --title \"Pedro\" --field team_ids=team/platform --field role=Founder --json",
                    "echo '{\"title\":\"Versatly\",\"fields\":{\"summary\":\"AI-native company\",\"tags\":[\"company\"]}}' | workgraph create org --stdin --json",
                ],
            ),
            capability(
                "query",
                "Query primitives by type with exact scalar filters and repeated-field containment where the schema allows it.",
                vec!["<type>"],
                &[global_flags[0], global_flags[1], "--filter key=value"],
                vec![
                    "workgraph query org --json",
                    "workgraph query decision --filter status=decided --json",
                    "workgraph query person --filter team_ids=team/platform --json",
                    "workgraph query thread",
                ],
            ),
            capability(
                "show",
                "Load one primitive by <type>/<id> with graph-backed references when available.",
                vec!["<type>/<id>"],
                &global_flags,
                vec![
                    "workgraph show org/versatly --json",
                    "workgraph show person/person:pedro --json",
                    "workgraph show decision/rust-for-workgraph-v4 --json",
                    "workgraph show thread/kernel-thread-1",
                ],
            ),
        ],
    }
}

/// Returns a structured CLI schema description, optionally narrowed to one primitive type.
#[must_use]
pub fn cli_schema(
    schema_version: &str,
    registry: &Registry,
    requested_primitive_type: Option<&str>,
) -> CliSchema {
    let primitive_types = registry
        .list_types()
        .iter()
        .filter(|primitive_type| {
            requested_primitive_type
                .map(|requested| primitive_type.name == requested)
                .unwrap_or(true)
        })
        .map(|primitive_type| PrimitiveTypeSchema {
            name: primitive_type.name.clone(),
            directory: primitive_type.directory.clone(),
            description: primitive_type.description.clone(),
            fields: primitive_type
                .fields
                .iter()
                .map(|field| PrimitiveFieldSchema {
                    name: field.name.clone(),
                    field_type: field.field_type.clone(),
                    description: field.description.clone(),
                    required: field.required,
                    repeated: field.repeated,
                    query_behavior: match field.query_behavior {
                        wg_types::FieldQueryBehavior::Exact => "exact",
                        wg_types::FieldQueryBehavior::Contains => "contains",
                        wg_types::FieldQueryBehavior::Opaque => "opaque",
                    }
                    .to_owned(),
                    reference_types: field.reference_types.clone(),
                    graph_edge_kind: field.graph_edge_kind.map(|kind| {
                        match kind {
                            wg_types::GraphEdgeKind::Reference => "reference",
                            wg_types::GraphEdgeKind::Relationship => "relationship",
                            wg_types::GraphEdgeKind::Assignment => "assignment",
                            wg_types::GraphEdgeKind::Containment => "containment",
                            wg_types::GraphEdgeKind::Evidence => "evidence",
                            wg_types::GraphEdgeKind::Trigger => "trigger",
                        }
                        .to_owned()
                    }),
                })
                .collect(),
        })
        .collect();

    CliSchema {
        schema_version: schema_version.to_owned(),
        envelope_fields: vec![
            schema_field(
                "schema_version",
                "string",
                "Stable envelope version for machine parsing.",
                true,
            ),
            schema_field(
                "success",
                "boolean",
                "True when the command succeeded.",
                true,
            ),
            schema_field(
                "command",
                "string",
                "The command that produced the envelope.",
                true,
            ),
            schema_field(
                "result",
                "object",
                "Structured command payload for successful responses.",
                true,
            ),
            schema_field(
                "next_actions",
                "string[]",
                "Suggested follow-up command invocations.",
                true,
            ),
            schema_field(
                "error",
                "string",
                "Human-readable error message when success=false.",
                false,
            ),
            schema_field(
                "fix",
                "string",
                "Actionable recovery command when success=false.",
                false,
            ),
        ],
        primitive_types,
    }
}

fn capability(
    name: &str,
    description: &str,
    required_args: Vec<&str>,
    flags: &[&str],
    examples: Vec<&str>,
) -> CommandCapability {
    CommandCapability {
        name: name.to_owned(),
        description: description.to_owned(),
        required_args: required_args.into_iter().map(ToOwned::to_owned).collect(),
        flags: flags.iter().map(ToString::to_string).collect(),
        examples: examples.into_iter().map(ToOwned::to_owned).collect(),
    }
}

fn schema_field(name: &str, field_type: &str, description: &str, required: bool) -> SchemaField {
    SchemaField {
        name: name.to_owned(),
        field_type: field_type.to_owned(),
        description: description.to_owned(),
        required,
    }
}
