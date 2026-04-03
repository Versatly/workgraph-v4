//! Structured CLI output models and rendering entrypoints.

mod envelope;
mod human;
mod json;

use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value as JsonValue;
use wg_orientation::{GraphIssue, RecentActivity, ThreadEvidenceGap, WorkspaceBrief};
use wg_store::StoredPrimitive;
use wg_types::{LedgerEntry, WorkgraphConfig};

/// Stable schema version for the JSON agent contract emitted by the CLI.
pub const AGENT_SCHEMA_VERSION: &str = "v1";

/// A structured command result suitable for either human or JSON rendering.
#[derive(Debug, Serialize)]
#[serde(tag = "command", content = "result", rename_all = "snake_case")]
pub enum CommandOutput {
    /// Result of `workgraph init`.
    Init(InitOutput),
    /// Result of `workgraph brief`.
    Brief(BriefOutput),
    /// Result of `workgraph status`.
    Status(StatusOutput),
    /// Result of `workgraph capabilities`.
    Capabilities(CapabilitiesOutput),
    /// Result of `workgraph schema`.
    Schema(SchemaOutput),
    /// Result of `workgraph create`.
    Create(CreateOutput),
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

/// Stable workspace identity details included in orientation responses.
#[derive(Debug, Serialize)]
pub struct WorkspaceIdentity {
    /// Stable workspace identifier.
    pub id: String,
    /// Human-readable workspace name.
    pub name: String,
    /// Filesystem root for this workspace.
    pub root: String,
    /// Default actor identifier when configured.
    pub default_actor_id: Option<String>,
}

/// Output model produced by the `brief` command.
#[derive(Debug, Serialize)]
pub struct BriefOutput {
    /// Workspace identity metadata.
    pub workspace: WorkspaceIdentity,
    /// Primitive counts grouped by primitive type.
    pub primitive_counts: BTreeMap<String, usize>,
    /// Last immutable ledger entries for immediate orientation.
    pub recent_ledger_entries: Vec<LedgerEntry>,
    /// Recommended follow-up commands for entering agents.
    pub suggested_next_actions: Vec<String>,
    /// Rich orientation sections and warnings for entering agents.
    pub orientation: WorkspaceBrief,
}

/// Output model produced by the `capabilities` command.
#[derive(Debug, Serialize)]
pub struct CapabilitiesOutput {
    /// The first command agents should call to orient on a workspace.
    pub first_command: String,
    /// Structured command capabilities for self-discovery.
    pub commands: Vec<super::services::discovery::CommandCapability>,
}

/// Output model produced by the `schema` command.
#[derive(Debug, Serialize)]
pub struct SchemaOutput {
    /// Stable schema version for machine-readable discovery responses.
    pub schema_version: String,
    /// The top-level structured envelope fields emitted in JSON mode.
    pub envelope_fields: Vec<super::services::discovery::SchemaField>,
    /// Primitive field definitions for one type (or all when omitted).
    pub primitive_types: Vec<super::services::discovery::PrimitiveTypeSchema>,
}

/// Structured outcome for `create` mutations.
#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CreateOutcome {
    /// A new primitive was persisted and ledgered.
    Created,
    /// The existing primitive already matched the requested payload.
    Noop,
    /// A create preview was returned without writing.
    DryRun,
}

/// Output model produced by the `create` command.
#[derive(Debug, Serialize)]
pub struct CreateOutput {
    /// Whether this create call persisted state, noop'd, or was a dry-run preview.
    pub outcome: CreateOutcome,
    /// The created primitive reference in `<type>/<id>` form.
    pub reference: String,
    /// The filesystem path where the markdown primitive was stored or would be stored.
    pub path: String,
    /// The stored primitive payload.
    pub primitive: StoredPrimitive,
    /// The appended ledger entry corresponding to the creation event, when persisted.
    pub ledger_entry: Option<LedgerEntry>,
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

/// Renders a structured command output in either human-readable or JSON form.
///
/// # Errors
///
/// Returns an error when JSON serialization fails.
pub fn render_success(output: &CommandOutput, json: bool) -> anyhow::Result<String> {
    let envelope = envelope::success(output)?;
    if json {
        json::render_success(&envelope)
    } else {
        Ok(human::render(output, &envelope.next_actions))
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
    let envelope = envelope::failure(command, error);
    if json {
        json::render_failure(&envelope)
    } else {
        Ok(human::render_failure(command, error, &envelope.fix))
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
            Self::Query(output) => serde_json::to_value(output),
            Self::Show(output) => serde_json::to_value(output),
        }
        .map_err(Into::into)
    }

    /// Returns contextual follow-up actions that agents can take next.
    #[must_use]
    pub fn next_actions(&self) -> Vec<String> {
        match self {
            Self::Init(_) => vec![
                "workgraph brief".to_owned(),
                "workgraph capabilities".to_owned(),
                "workgraph create org --title \"<title>\"".to_owned(),
                "workgraph show org/versatly".to_owned(),
            ],
            Self::Brief(_) => vec![
                "workgraph show org/versatly".to_owned(),
                "workgraph query org".to_owned(),
                "workgraph status".to_owned(),
            ],
            Self::Status(_) => vec![
                "workgraph brief".to_owned(),
                "workgraph query org".to_owned(),
            ],
            Self::Capabilities(_) => vec![
                "workgraph schema".to_owned(),
                "workgraph brief".to_owned(),
                "workgraph create org --title \"<title>\"".to_owned(),
            ],
            Self::Schema(_) => vec![
                "workgraph capabilities".to_owned(),
                "workgraph create org --title \"<title>\"".to_owned(),
            ],
            Self::Create(output) => vec![
                format!("workgraph show {}", output.reference),
                "workgraph status".to_owned(),
                format!("workgraph query {}", output.primitive.frontmatter.r#type),
            ],
            Self::Query(output) => {
                let mut actions = vec!["workgraph brief".to_owned()];
                if let Some(first) = output.items.first() {
                    actions.push(format!(
                        "workgraph show {}/{}",
                        first.frontmatter.r#type, first.frontmatter.id
                    ));
                }
                actions
            }
            Self::Show(output) => vec![
                format!("workgraph query {}", output.primitive.frontmatter.r#type),
                "workgraph status".to_owned(),
            ],
        }
    }
}
