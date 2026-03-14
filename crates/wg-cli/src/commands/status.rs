//! Implementation of the `workgraph status` command.

use std::collections::BTreeMap;

use anyhow::Context;
use wg_store::list_primitives;

use crate::app::AppContext;
use crate::output::StatusOutput;

/// Collects primitive counts and the most recent ledger entry for the workspace.
///
/// # Errors
///
/// Returns an error when workspace metadata or primitives cannot be read.
pub async fn handle(app: &AppContext) -> anyhow::Result<StatusOutput> {
    let config = app.load_config().await?;
    let registry = app.load_registry().await?;
    let mut counts = BTreeMap::new();

    for primitive_type in registry.list_types() {
        let primitives = list_primitives(app.workspace(), &primitive_type.name)
            .await
            .with_context(|| format!("failed to list primitive type '{}'", primitive_type.name))?;
        counts.insert(primitive_type.name.clone(), primitives.len());
    }

    let entries = app.read_ledger_entries().await?;

    Ok(StatusOutput {
        config,
        workspace_root: app.root().display().to_string(),
        type_counts: counts,
        last_entry: entries.last().cloned(),
    })
}
