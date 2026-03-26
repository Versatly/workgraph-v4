//! Structured discovery metadata for agent-native CLI use.

use serde::Serialize;
use wg_types::{FieldDefinition, Registry};

/// Structured capability and workflow discovery output for the WorkGraph CLI.
#[derive(Debug, Clone, Serialize)]
pub struct CapabilitiesCatalog {
    /// Recommended machine-readable output format for autonomous agents.
    pub recommended_format: String,
    /// High-level workflow groupings exposed by the CLI.
    pub workflows: Vec<WorkflowSkill>,
    /// Concrete command-level capabilities.
    pub commands: Vec<CommandSkill>,
    /// First-class primitive contracts that agents should understand before writing.
    pub primitive_contracts: Vec<PrimitiveContract>,
}

/// A grouped workflow oriented around a common agent objective.
#[derive(Debug, Clone, Serialize)]
pub struct WorkflowSkill {
    /// Stable workflow key.
    pub key: String,
    /// Human-readable workflow title.
    pub title: String,
    /// What the workflow helps an agent accomplish.
    pub description: String,
    /// The commands commonly used in the workflow.
    pub commands: Vec<String>,
    /// Whether this workflow is common and broadly recommended.
    pub common: bool,
}

/// Structured metadata for a single CLI command.
#[derive(Debug, Clone, Serialize)]
pub struct CommandSkill {
    /// The command name.
    pub name: String,
    /// Human-readable description of the command.
    pub description: String,
    /// Canonical examples for agents to imitate.
    pub examples: Vec<String>,
    /// Whether the command supports stable machine-readable output.
    pub machine_readable: bool,
    /// The common follow-up commands after this one succeeds.
    pub next_commands: Vec<String>,
}

/// Structured schema metadata for the CLI and selected commands.
#[derive(Debug, Clone, Serialize)]
pub struct CliSchema {
    /// The stable schema version for machine-readable result envelopes.
    pub schema_version: String,
    /// A description of the top-level envelope fields in JSON mode.
    pub envelope_fields: Vec<SchemaField>,
    /// Structured command definitions.
    pub commands: Vec<CommandSchema>,
    /// Typed primitive contracts discoverable through the CLI.
    pub primitive_contracts: Vec<PrimitiveContract>,
}

/// A field inside a structured result envelope or command definition.
#[derive(Debug, Clone, Serialize)]
pub struct SchemaField {
    /// The stable field name.
    pub name: String,
    /// The field type.
    pub field_type: String,
    /// A short human-readable description.
    pub description: String,
    /// Whether the field is always present.
    pub required: bool,
}

/// Structured description of a durable primitive contract.
#[derive(Debug, Clone, Serialize)]
pub struct PrimitiveContract {
    /// Primitive type name.
    pub name: String,
    /// Human-readable purpose of the primitive.
    pub description: String,
    /// Required fields the primitive must carry.
    pub required_fields: Vec<SchemaField>,
    /// Optional fields the primitive may carry.
    pub optional_fields: Vec<SchemaField>,
    /// Additional semantic notes an agent should preserve.
    pub notes: Vec<String>,
}

/// A structured description of one command's arguments and behavior.
#[derive(Debug, Clone, Serialize)]
pub struct CommandSchema {
    /// The command name.
    pub name: String,
    /// A concise command description.
    pub description: String,
    /// The arguments supported by the command.
    pub arguments: Vec<CommandArgument>,
    /// A machine-readable example invocation.
    pub example: String,
}

/// A structured argument description for a command.
#[derive(Debug, Clone, Serialize)]
pub struct CommandArgument {
    /// The argument name or flag.
    pub name: String,
    /// A concise explanation of the argument.
    pub description: String,
    /// Whether the argument is required.
    pub required: bool,
}

/// Returns the static CLI capabilities catalog.
#[must_use]
pub fn capabilities_catalog() -> CapabilitiesCatalog {
    CapabilitiesCatalog {
        recommended_format: "json".to_owned(),
        workflows: vec![
            WorkflowSkill {
                key: "orientation".to_owned(),
                title: "Workspace orientation".to_owned(),
                description: "Enter a workspace, inspect the typed graph, and notice active work plus evidence gaps.".to_owned(),
                commands: vec![
                    "workgraph --json init".to_owned(),
                    "workgraph --json brief".to_owned(),
                    "workgraph --json status".to_owned(),
                ],
                common: true,
            },
            WorkflowSkill {
                key: "knowledge_capture".to_owned(),
                title: "Context capture".to_owned(),
                description: "Record durable company context and coordination state with provenance in the ledger.".to_owned(),
                commands: vec![
                    "workgraph --json create <type> --title ...".to_owned(),
                    "workgraph --json show <type>/<id>".to_owned(),
                ],
                common: true,
            },
            WorkflowSkill {
                key: "coordination".to_owned(),
                title: "Coordination integrity".to_owned(),
                description: "Inspect thread, mission, run, and trigger contracts before mutating active work.".to_owned(),
                commands: vec![
                    "workgraph --json status".to_owned(),
                    "workgraph --json schema".to_owned(),
                    "workgraph --json show thread/<id>".to_owned(),
                ],
                common: true,
            },
            WorkflowSkill {
                key: "discovery".to_owned(),
                title: "Capability discovery".to_owned(),
                description: "Discover available commands, schemas, and structured agent contracts.".to_owned(),
                commands: vec![
                    "workgraph --json capabilities".to_owned(),
                    "workgraph --json schema".to_owned(),
                ],
                common: true,
            },
        ],
        commands: vec![
            command_skill(
                "init",
                "Initialize registry, config, ledger, and primitive directories.",
                vec!["workgraph --json init".to_owned()],
                vec!["brief".to_owned(), "create".to_owned()],
            ),
            command_skill(
                "brief",
                "Produce a structured workspace orientation including typed coordination warnings.",
                vec![
                    "workgraph --json brief".to_owned(),
                    "workgraph --json brief --lens delivery".to_owned(),
                ],
                vec!["create".to_owned(), "query".to_owned(), "status".to_owned()],
            ),
            command_skill(
                "status",
                "Inspect primitive counts, graph issues, evidence gaps, and the latest immutable ledger event.",
                vec!["workgraph --json status".to_owned()],
                vec!["brief".to_owned(), "query".to_owned(), "schema".to_owned()],
            ),
            command_skill(
                "create",
                "Create a markdown primitive and append a matching ledger entry.",
                vec!["workgraph --json create org --title Versatly".to_owned()],
                vec!["show".to_owned(), "status".to_owned(), "query".to_owned()],
            ),
            command_skill(
                "query",
                "List primitives of one type with optional exact-match filters.",
                vec!["workgraph --json query decision --filter status=decided".to_owned()],
                vec!["show".to_owned(), "brief".to_owned()],
            ),
            command_skill(
                "show",
                "Load a single primitive by <type>/<id> with coordination-aware rendering.",
                vec!["workgraph --json show org/versatly".to_owned()],
                vec!["query".to_owned(), "status".to_owned()],
            ),
            command_skill(
                "capabilities",
                "List structured agent workflows, CLI capabilities, and primitive contracts.",
                vec!["workgraph --json capabilities".to_owned()],
                vec!["schema".to_owned()],
            ),
            command_skill(
                "schema",
                "Describe JSON result envelopes, command contracts, and primitive contracts.",
                vec!["workgraph --json schema".to_owned()],
                vec!["capabilities".to_owned()],
            ),
        ],
        primitive_contracts: primitive_contracts(),
    }
}

/// Returns a structured CLI schema description, optionally narrowed to one command.
#[must_use]
pub fn cli_schema(schema_version: &str, requested_command: Option<&str>) -> CliSchema {
    let mut commands = vec![
        command_schema(
            "init",
            "Initialize a workspace.",
            vec![],
            "workgraph --json init",
        ),
        command_schema(
            "brief",
            "Produce a structured workspace brief with graph and coordination warnings.",
            vec![CommandArgument {
                name: "--lens".to_owned(),
                description: "Orientation lens: workspace, delivery, policy, or agents.".to_owned(),
                required: false,
            }],
            "workgraph --json brief --lens workspace",
        ),
        command_schema(
            "status",
            "Show primitive counts, recent activity, graph issues, and evidence gaps.",
            vec![],
            "workgraph --json status",
        ),
        command_schema(
            "create",
            "Create a primitive and record it in the ledger.",
            vec![
                CommandArgument {
                    name: "<type>".to_owned(),
                    description: "Primitive type to create.".to_owned(),
                    required: true,
                },
                CommandArgument {
                    name: "--title".to_owned(),
                    description: "Human-readable primitive title.".to_owned(),
                    required: true,
                },
                CommandArgument {
                    name: "--field".to_owned(),
                    description: "Additional frontmatter as key=value pairs.".to_owned(),
                    required: false,
                },
            ],
            "workgraph --json create org --title Versatly",
        ),
        command_schema(
            "query",
            "Query primitives of one type using exact-match filters.",
            vec![
                CommandArgument {
                    name: "<type>".to_owned(),
                    description: "Primitive type to query.".to_owned(),
                    required: true,
                },
                CommandArgument {
                    name: "--filter".to_owned(),
                    description: "Exact frontmatter filter in key=value form.".to_owned(),
                    required: false,
                },
            ],
            "workgraph --json query decision --filter status=decided",
        ),
        command_schema(
            "show",
            "Show a single primitive by reference with typed coordination sections when relevant.",
            vec![CommandArgument {
                name: "<type>/<id>".to_owned(),
                description: "Primitive reference to display.".to_owned(),
                required: true,
            }],
            "workgraph --json show org/versatly",
        ),
        command_schema(
            "capabilities",
            "List structured CLI capabilities.",
            vec![],
            "workgraph --json capabilities",
        ),
        command_schema(
            "schema",
            "Describe CLI command, output, and primitive contracts.",
            vec![CommandArgument {
                name: "[command]".to_owned(),
                description: "Optional command name to narrow the schema view.".to_owned(),
                required: false,
            }],
            "workgraph --json schema create",
        ),
    ];

    if let Some(requested_command) = requested_command {
        commands.retain(|command| command.name == requested_command);
    }

    CliSchema {
        schema_version: schema_version.to_owned(),
        envelope_fields: vec![
            schema_field(
                "schema_version",
                "string",
                "Stable JSON envelope version.",
                true,
            ),
            schema_field(
                "success",
                "boolean",
                "Whether the command completed successfully.",
                true,
            ),
            schema_field(
                "command",
                "string",
                "The command that produced this response.",
                true,
            ),
            schema_field(
                "result",
                "object|null",
                "Successful command payload.",
                false,
            ),
            schema_field(
                "error",
                "object|null",
                "Structured error details when success=false.",
                false,
            ),
            schema_field("fix", "string|null", "Actionable remediation hint.", false),
            schema_field(
                "next_actions",
                "array",
                "Suggested follow-up commands.",
                true,
            ),
        ],
        commands,
        primitive_contracts: primitive_contracts(),
    }
}

fn command_skill(
    name: &str,
    description: &str,
    examples: Vec<String>,
    next_commands: Vec<String>,
) -> CommandSkill {
    CommandSkill {
        name: name.to_owned(),
        description: description.to_owned(),
        examples,
        machine_readable: true,
        next_commands,
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

fn command_schema(
    name: &str,
    description: &str,
    arguments: Vec<CommandArgument>,
    example: &str,
) -> CommandSchema {
    CommandSchema {
        name: name.to_owned(),
        description: description.to_owned(),
        arguments,
        example: example.to_owned(),
    }
}

fn primitive_contracts() -> Vec<PrimitiveContract> {
    let registry = Registry::builtins();
    registry
        .list_types()
        .iter()
        .map(|primitive_type| PrimitiveContract {
            name: primitive_type.name.clone(),
            description: primitive_type.description.clone(),
            required_fields: primitive_type
                .fields
                .iter()
                .filter(|field| field.required)
                .map(schema_field_from_definition)
                .collect(),
            optional_fields: primitive_type
                .fields
                .iter()
                .filter(|field| !field.required)
                .map(schema_field_from_definition)
                .collect(),
            notes: primitive_notes(&primitive_type.name),
        })
        .collect()
}

fn primitive_notes(name: &str) -> Vec<String> {
    match name {
        "agent" => vec![
            "Agents may declare parent and root actor lineage while leaving descendants opaque."
                .to_owned(),
        ],
        "thread" => vec![
            "Threads close only when required exit criteria are satisfied by recorded evidence."
                .to_owned(),
            "Update and completion actions are durable plans, not auto-executed effects."
                .to_owned(),
        ],
        "mission" => vec![
            "Missions coordinate related threads and runs but are not generic task records."
                .to_owned(),
        ],
        "run" => vec![
            "Each run belongs to exactly one thread and may optionally reference a mission or parent run."
                .to_owned(),
        ],
        "trigger" => vec![
            "Triggers match event patterns and emit action plans without mutating state in this foundation pass."
                .to_owned(),
        ],
        "checkpoint" => vec![
            "Checkpoints preserve resumable working context for future humans or agents.".to_owned(),
        ],
        _ => Vec::new(),
    }
}

fn schema_field_from_definition(definition: &FieldDefinition) -> SchemaField {
    SchemaField {
        name: definition.name.clone(),
        field_type: definition.field_type.clone(),
        description: definition.description.clone(),
        required: definition.required,
    }
}
