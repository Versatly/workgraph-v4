//! Structured CLI output models and rendering entrypoints.

mod human;
mod json;

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value as JsonValue;
use wg_orientation::{GraphIssue, RecentActivity, ThreadEvidenceGap, WorkspaceBrief};
use wg_store::StoredPrimitive;
use wg_types::{LedgerEntry, TriggerActionPlan, WorkgraphConfig};

/// Stable schema version for the JSON agent contract emitted by the CLI.
pub const AGENT_SCHEMA_VERSION: &str = "workgraph.cli.v1alpha2";

/// A structured command result suitable for either human or JSON rendering.
#[derive(Debug, Serialize)]
#[serde(tag = "command", content = "result", rename_all = "snake_case")]
pub enum CommandOutput {
    /// Result of `workgraph init`.
    Init(InitOutput),
    /// Result of `workgraph brief`.
    Brief(WorkspaceBrief),
    /// Result of `workgraph status`.
    Status(StatusOutput),
    /// Result of `workgraph capabilities`.
    Capabilities(CapabilitiesOutput),
    /// Result of `workgraph schema`.
    Schema(SchemaOutput),
    /// Result of `workgraph create`.
    Create(CreateOutput),
    /// Result of `workgraph thread`.
    Thread(ThreadOutput),
    /// Result of `workgraph mission`.
    Mission(MissionOutput),
    /// Result of `workgraph run`.
    Run(RunOutput),
    /// Result of `workgraph trigger`.
    Trigger(TriggerOutput),
    /// Result of `workgraph checkpoint`.
    Checkpoint(CheckpointOutput),
    /// Result of `workgraph query`.
    Query(QueryOutput),
    /// Result of `workgraph show`.
    Show(ShowOutput),
}

/// Output model produced by the `init` command.
#[derive(Debug, Serialize)]
pub struct InitOutput {
    /// The persisted workspace configuration.
    pub config: WorkgraphConfig,
    /// The path to the serialized registry file.
    pub registry_path: String,
    /// The path to the append-only ledger file.
    pub ledger_path: String,
    /// The path to the serialized config file.
    pub config_path: String,
    /// The primitive directories ensured during initialization.
    pub created_directories: Vec<String>,
}

/// Output model produced by the `status` command.
#[derive(Debug, Serialize)]
pub struct StatusOutput {
    /// The persisted workspace configuration, when available.
    pub config: WorkgraphConfig,
    /// The filesystem root of the workspace.
    pub workspace_root: String,
    /// Primitive counts for each registered type.
    pub type_counts: BTreeMap<String, usize>,
    /// Recent immutable ledger activity summarized for orientation.
    pub recent_activity: Vec<RecentActivity>,
    /// The most recent immutable ledger entry, when present.
    pub last_entry: Option<LedgerEntry>,
    /// Typed graph hygiene issues discovered by the graph builder.
    pub graph_issues: Vec<GraphIssue>,
    /// Threads that cannot yet complete because required evidence is missing.
    pub thread_evidence_gaps: Vec<ThreadEvidenceGap>,
}

/// Output model produced by the `capabilities` command.
#[derive(Debug, Serialize)]
pub struct CapabilitiesOutput {
    /// Recommended machine-readable format for autonomous agents.
    pub recommended_format: String,
    /// Grouped workflows exposed by the CLI.
    pub workflows: Vec<super::services::discovery::WorkflowSkill>,
    /// Command-level structured capabilities.
    pub commands: Vec<super::services::discovery::CommandSkill>,
    /// First-class primitive contracts that agents should understand before writing.
    pub primitive_contracts: Vec<super::services::discovery::PrimitiveContract>,
}

/// Output model produced by the `schema` command.
#[derive(Debug, Serialize)]
pub struct SchemaOutput {
    /// Stable schema version for agent-native CLI envelopes.
    pub schema_version: String,
    /// The top-level structured envelope fields emitted in JSON mode.
    pub envelope_fields: Vec<super::services::discovery::SchemaField>,
    /// Structured command definitions.
    pub commands: Vec<super::services::discovery::CommandSchema>,
    /// Typed primitive contracts discoverable through the CLI.
    pub primitive_contracts: Vec<super::services::discovery::PrimitiveContract>,
}

/// Output model produced by the `create` command.
#[derive(Debug, Serialize)]
pub struct CreateOutput {
    /// The created primitive reference in `<type>/<id>` form.
    pub reference: String,
    /// Whether the create operation was a dry run.
    pub dry_run: bool,
    /// Whether an existing primitive already satisfied the create request.
    pub idempotent: bool,
    /// The filesystem path where the markdown primitive was stored.
    pub path: String,
    /// The stored primitive that was written.
    pub primitive: StoredPrimitive,
    /// The appended ledger entry corresponding to the creation event.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ledger_entry: Option<LedgerEntry>,
}

/// Output model produced by thread commands.
#[derive(Debug, Serialize)]
pub struct ThreadOutput {
    /// Stable command action, such as create, claim, or complete.
    pub action: String,
    /// Whether the operation was evaluated as a dry run.
    pub dry_run: bool,
    /// Thread reference in `<type>/<id>` form.
    pub reference: String,
    /// Persisted or previewed thread state.
    pub thread: StoredPrimitive,
}

/// Output model produced by mission commands.
#[derive(Debug, Serialize)]
pub struct MissionOutput {
    /// Stable command action, such as create, activate, or progress.
    pub action: String,
    /// Whether the operation was evaluated as a dry run.
    pub dry_run: bool,
    /// Mission reference in `<type>/<id>` form.
    pub reference: String,
    /// Persisted mission primitive, when applicable.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mission: Option<StoredPrimitive>,
    /// Progress snapshot when the command computes mission progress.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<MissionProgressOutput>,
}

/// Output model produced by mission progress commands.
#[derive(Debug, Serialize)]
pub struct MissionProgressOutput {
    /// Number of completed child threads.
    pub completed_threads: usize,
    /// Total tracked child threads.
    pub total_threads: usize,
}

/// Output model produced by run commands.
#[derive(Debug, Serialize)]
pub struct RunOutput {
    /// Stable command action, such as create, start, or complete.
    pub action: String,
    /// Whether the operation was evaluated as a dry run.
    pub dry_run: bool,
    /// Run reference in `<type>/<id>` form.
    pub reference: String,
    /// Persisted or previewed run state.
    pub run: StoredPrimitive,
}

/// Output model produced by trigger commands.
#[derive(Debug, Serialize)]
pub struct TriggerOutput {
    /// Stable command action, such as save or evaluate.
    pub action: String,
    /// Whether the operation was evaluated as a dry run.
    pub dry_run: bool,
    /// Trigger reference when a trigger was created or updated.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// Persisted or previewed trigger primitive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<StoredPrimitive>,
    /// Ledger entry reference used for evaluation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluated_entry: Option<LedgerEntry>,
    /// Matched triggers and their planned action outputs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub matches: Vec<TriggerMatchOutput>,
}

/// Output model produced by checkpoint commands.
#[derive(Debug, Serialize)]
pub struct CheckpointOutput {
    /// Stable command action.
    pub action: String,
    /// Whether the operation was evaluated as a dry run.
    pub dry_run: bool,
    /// Checkpoint reference in `<type>/<id>` form.
    pub reference: String,
    /// Persisted or previewed checkpoint primitive.
    pub checkpoint: StoredPrimitive,
}

/// One matched trigger and its durable action plans.
#[derive(Debug, Serialize)]
pub struct TriggerMatchOutput {
    /// Matched trigger identifier.
    pub trigger_id: String,
    /// Trigger title.
    pub title: String,
    /// Durable action plans yielded by the trigger.
    pub action_plans: Vec<TriggerActionPlan>,
}

/// Output model produced by the `query` command.
#[derive(Debug, Serialize)]
pub struct QueryOutput {
    /// The primitive type that was queried.
    pub primitive_type: String,
    /// The number of matched primitives.
    pub count: usize,
    /// The matched stored primitives.
    pub items: Vec<StoredPrimitive>,
}

/// Output model produced by the `show` command.
#[derive(Debug, Serialize)]
pub struct ShowOutput {
    /// The requested primitive reference in `<type>/<id>` form.
    pub reference: String,
    /// The loaded primitive.
    pub primitive: StoredPrimitive,
}

/// A suggested follow-up action an autonomous agent can attempt next.
#[derive(Debug, Clone, Serialize)]
pub struct NextAction {
    /// A short stable label for the suggested action.
    pub title: String,
    /// A concrete command template the agent can execute.
    pub command: String,
    /// Why this follow-up is useful.
    pub description: String,
}

/// A structured machine-readable error payload for JSON mode.
#[derive(Debug, Clone, Serialize)]
pub struct AgentError {
    /// Stable machine-readable error code.
    pub code: String,
    /// Human-readable error message.
    pub message: String,
}

/// Renders a structured command output in either human-readable or JSON form.
///
/// # Errors
///
/// Returns an error when JSON serialization fails.
pub fn render_success(output: &CommandOutput, json: bool) -> anyhow::Result<String> {
    if json {
        json::render_success(output)
    } else {
        Ok(human::render(output))
    }
}

/// Renders a structured command failure in either human-readable or JSON form.
///
/// # Errors
///
/// Returns an error when JSON serialization fails.
pub fn render_failure(
    command: Option<&str>,
    error: &anyhow::Error,
    json: bool,
) -> anyhow::Result<String> {
    if json {
        json::render_failure(command, error)
    } else {
        Ok(human::render_failure(command, error))
    }
}

impl CommandOutput {
    /// Returns the stable command name associated with this output.
    #[must_use]
    pub const fn command_name(&self) -> &'static str {
        match self {
            Self::Init(_) => "init",
            Self::Brief(_) => "brief",
            Self::Status(_) => "status",
            Self::Capabilities(_) => "capabilities",
            Self::Schema(_) => "schema",
            Self::Create(_) => "create",
            Self::Thread(_) => "thread",
            Self::Mission(_) => "mission",
            Self::Run(_) => "run",
            Self::Trigger(_) => "trigger",
            Self::Checkpoint(_) => "checkpoint",
            Self::Query(_) => "query",
            Self::Show(_) => "show",
        }
    }

    /// Serializes the successful result payload into JSON.
    ///
    /// # Errors
    ///
    /// Returns an error when the successful output payload cannot be serialized.
    pub fn result_value(&self) -> anyhow::Result<JsonValue> {
        match self {
            Self::Init(output) => serde_json::to_value(output),
            Self::Brief(output) => serde_json::to_value(output),
            Self::Status(output) => serde_json::to_value(output),
            Self::Capabilities(output) => serde_json::to_value(output),
            Self::Schema(output) => serde_json::to_value(output),
            Self::Create(output) => serde_json::to_value(output),
            Self::Thread(output) => serde_json::to_value(output),
            Self::Mission(output) => serde_json::to_value(output),
            Self::Run(output) => serde_json::to_value(output),
            Self::Trigger(output) => serde_json::to_value(output),
            Self::Checkpoint(output) => serde_json::to_value(output),
            Self::Query(output) => serde_json::to_value(output),
            Self::Show(output) => serde_json::to_value(output),
        }
        .map_err(Into::into)
    }

    /// Returns contextual follow-up actions that agents can take next.
    #[must_use]
    pub fn next_actions(&self) -> Vec<NextAction> {
        match self {
            Self::Init(_) => vec![
                next_action(
                    "brief",
                    "workgraph --json brief",
                    "Orient a new agent entering the workspace.",
                ),
                next_action(
                    "capabilities",
                    "workgraph --json capabilities",
                    "Discover structured CLI capabilities and workflows.",
                ),
                next_action(
                    "create-org",
                    "workgraph --json create org --title <title>",
                    "Record the primary organization context.",
                ),
            ],
            Self::Brief(_) => vec![
                next_action(
                    "status",
                    "workgraph --json status",
                    "Inspect workspace counts and the latest immutable activity.",
                ),
                next_action(
                    "query",
                    "workgraph --json query <type>",
                    "Inspect a specific primitive type in more detail.",
                ),
                next_action(
                    "create",
                    "workgraph --json create <type> --title <title>",
                    "Contribute new company context to the graph.",
                ),
            ],
            Self::Status(_) => vec![
                next_action(
                    "brief",
                    "workgraph --json brief",
                    "Get a richer orientation summary than raw counts.",
                ),
                next_action(
                    "query",
                    "workgraph --json query <type>",
                    "Inspect a specific primitive type.",
                ),
            ],
            Self::Capabilities(_) => vec![
                next_action(
                    "schema",
                    "workgraph --json schema",
                    "Inspect the structured output and command contract.",
                ),
                next_action(
                    "brief",
                    "workgraph --json brief",
                    "Use the recommended orientation workflow.",
                ),
            ],
            Self::Schema(_) => vec![
                next_action(
                    "capabilities",
                    "workgraph --json capabilities",
                    "Inspect higher-level workflows and examples.",
                ),
                next_action(
                    "brief",
                    "workgraph --json brief",
                    "Run a concrete orientation command using the schema.",
                ),
            ],
            Self::Create(output) => vec![
                next_action(
                    "show",
                    &format!("workgraph --json show {}", output.reference),
                    "Inspect the newly written primitive and confirm its stored representation.",
                ),
                next_action(
                    "status",
                    "workgraph --json status",
                    "Confirm the ledger and counts reflect the new primitive.",
                ),
                next_action(
                    "query-type",
                    &format!(
                        "workgraph --json query {}",
                        output.primitive.frontmatter.r#type
                    ),
                    "List primitives of the same type for additional context.",
                ),
            ],
            Self::Thread(output) => vec![
                next_action(
                    "show-thread",
                    &format!("workgraph --json show {}", output.reference),
                    "Inspect the thread primitive and its coordination state.",
                ),
                next_action(
                    "status",
                    "workgraph --json status",
                    "Inspect graph issues and evidence gaps after the thread change.",
                ),
            ],
            Self::Mission(output) => {
                let mut actions = vec![next_action(
                    "status",
                    "workgraph --json status",
                    "Inspect mission, thread, and graph state after the mission change.",
                )];
                if let Some(reference) = output.mission.as_ref().map(|mission| {
                    format!("{}/{}", mission.frontmatter.r#type, mission.frontmatter.id)
                }) {
                    actions.push(next_action(
                        "show-mission",
                        &format!("workgraph --json show {reference}"),
                        "Inspect the mission primitive in detail.",
                    ));
                }
                actions
            }
            Self::Run(output) => vec![
                next_action(
                    "show-run",
                    &format!("workgraph --json show {}", output.reference),
                    "Inspect the run primitive and lifecycle state.",
                ),
                next_action(
                    "status",
                    "workgraph --json status",
                    "Inspect workspace activity after the run change.",
                ),
            ],
            Self::Trigger(output) => {
                let mut actions = vec![next_action(
                    "status",
                    "workgraph --json status",
                    "Inspect workspace activity and trigger-related graph issues.",
                )];
                if let Some(reference) = &output.reference {
                    actions.push(next_action(
                        "show-trigger",
                        &format!("workgraph --json show {reference}"),
                        "Inspect the trigger definition in detail.",
                    ));
                }
                actions
            }
            Self::Checkpoint(output) => vec![
                next_action(
                    "show-checkpoint",
                    &format!("workgraph --json show {}", output.reference),
                    "Inspect the saved checkpoint context.",
                ),
                next_action(
                    "brief",
                    "workgraph --json brief",
                    "Re-orient using the latest workspace state.",
                ),
            ],
            Self::Query(output) => {
                let mut actions = vec![next_action(
                    "brief",
                    "workgraph --json brief",
                    "Re-orient using a summarized workspace view.",
                )];
                if let Some(first) = output.items.first() {
                    actions.push(next_action(
                        "show-first",
                        &format!(
                            "workgraph --json show {}/{}",
                            first.frontmatter.r#type, first.frontmatter.id
                        ),
                        "Inspect the first matching primitive in detail.",
                    ));
                }
                actions
            }
            Self::Show(output) => vec![
                next_action(
                    "query-same-type",
                    &format!(
                        "workgraph --json query {}",
                        output.primitive.frontmatter.r#type
                    ),
                    "Inspect more primitives of the same type.",
                ),
                next_action(
                    "status",
                    "workgraph --json status",
                    "Return to a workspace-wide status summary.",
                ),
            ],
        }
    }
}

fn next_action(title: &str, command: &str, description: &str) -> NextAction {
    NextAction {
        title: title.to_owned(),
        command: command.to_owned(),
        description: description.to_owned(),
    }
}
