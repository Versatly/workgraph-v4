//! Structured CLI output models and rendering entrypoints.

mod human;
mod json;

use std::collections::BTreeMap;

use serde::Serialize;
use wg_store::StoredPrimitive;
use wg_types::{LedgerEntry, WorkgraphConfig};

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

/// Output model produced by the `brief` command.
#[derive(Debug, Serialize)]
pub struct BriefOutput {
    /// The stable workspace identifier.
    pub workspace_id: String,
    /// The human-readable workspace name.
    pub workspace_name: String,
    /// The filesystem root of the workspace.
    pub workspace_root: String,
    /// The configured default actor for CLI-originated mutations, when present.
    pub default_actor_id: Option<String>,
    /// Key primitive counts across the workspace.
    pub type_counts: BTreeMap<String, usize>,
    /// Titles of currently stored organizations.
    pub orgs: Vec<String>,
    /// Titles of currently stored clients.
    pub clients: Vec<String>,
    /// Titles of currently stored agents.
    pub agents: Vec<String>,
    /// Recent immutable ledger entries.
    pub recent_entries: Vec<LedgerEntry>,
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
    /// The most recent immutable ledger entry, when present.
    pub last_entry: Option<LedgerEntry>,
}

/// Output model produced by the `create` command.
#[derive(Debug, Serialize)]
pub struct CreateOutput {
    /// The created primitive reference in `<type>/<id>` form.
    pub reference: String,
    /// The filesystem path where the markdown primitive was stored.
    pub path: String,
    /// The stored primitive that was written.
    pub primitive: StoredPrimitive,
    /// The appended ledger entry corresponding to the creation event.
    pub ledger_entry: LedgerEntry,
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
pub fn render(output: &CommandOutput, json: bool) -> anyhow::Result<String> {
    if json {
        json::render(output)
    } else {
        Ok(human::render(output))
    }
}
