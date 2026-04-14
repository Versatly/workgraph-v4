//! Workspace-oriented helpers shared across CLI commands.

use std::path::Path;

use anyhow::anyhow;
use wg_types::{ActorId, WorkgraphConfig, WorkspaceId};

use crate::app::AppContext;
use crate::util::slug::slugify;

/// Builds a default persisted workspace configuration for a freshly initialized workspace.
#[must_use]
pub fn default_config(app: &AppContext, default_actor_id: Option<ActorId>) -> WorkgraphConfig {
    let workspace_name = derive_workspace_name(app.root());
    let workspace_id = WorkspaceId::new(slugify(&workspace_name));

    WorkgraphConfig {
        workspace_id,
        workspace_name,
        root_dir: app.root().display().to_string(),
        store_dir: app.root().display().to_string(),
        metadata_dir: app.metadata_dir_path().display().to_string(),
        ledger_file: app.ledger_path().display().to_string(),
        registry_file: app.registry_path().display().to_string(),
        config_file: app.config_path().display().to_string(),
        default_actor_id,
        local_node_id: None,
        remote: None,
    }
}

/// Derives a human-readable workspace name from the workspace directory name.
#[must_use]
pub fn derive_workspace_name(root: &Path) -> String {
    let raw_name = root
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.trim().is_empty())
        .unwrap_or("workgraph-workspace");

    raw_name
        .split(['-', '_', '.'])
        .filter(|segment| !segment.is_empty())
        .map(capitalize)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Parses a primitive reference of the form `<type>/<id>`.
///
/// # Errors
///
/// Returns an error when the input does not include both a type and identifier.
pub fn parse_reference(reference: &str) -> anyhow::Result<(&str, &str)> {
    reference
        .split_once('/')
        .ok_or_else(|| anyhow!("primitive reference must be in the form <type>/<id>"))
}

fn capitalize(input: &str) -> String {
    let mut characters = input.chars();
    match characters.next() {
        Some(first) => first.to_uppercase().chain(characters).collect(),
        None => String::new(),
    }
}
