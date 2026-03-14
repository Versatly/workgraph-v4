//! Implementation of the `workgraph brief` command.

use std::collections::BTreeMap;

use anyhow::Context;
use wg_store::list_primitives;

use crate::app::AppContext;
use crate::output::BriefOutput;

/// Produces a compact agent-first orientation summary for the current workspace.
///
/// # Errors
///
/// Returns an error when workspace metadata or stored primitives cannot be read.
pub async fn handle(app: &AppContext) -> anyhow::Result<BriefOutput> {
    let config = app.load_config().await?;
    let registry = app.load_registry().await?;
    let entries = app.read_ledger_entries().await?;
    let mut counts = BTreeMap::new();

    for primitive_type in registry.list_types() {
        let primitives = list_primitives(app.workspace(), &primitive_type.name)
            .await
            .with_context(|| format!("failed to list primitive type '{}'", primitive_type.name))?;
        counts.insert(primitive_type.name.clone(), primitives.len());
    }

    let orgs = list_titles(app, "org").await?;
    let clients = list_titles(app, "client").await?;
    let agents = list_titles(app, "agent").await?;
    let recent_entries = entries.into_iter().rev().take(5).collect::<Vec<_>>();

    Ok(BriefOutput {
        workspace_id: config.workspace_id.to_string(),
        workspace_name: config.workspace_name,
        workspace_root: config.root_dir,
        default_actor_id: config.default_actor_id.map(|actor| actor.to_string()),
        type_counts: counts,
        orgs,
        clients,
        agents,
        recent_entries,
    })
}

async fn list_titles(app: &AppContext, primitive_type: &str) -> anyhow::Result<Vec<String>> {
    Ok(list_primitives(app.workspace(), primitive_type)
        .await?
        .into_iter()
        .map(|primitive| primitive.frontmatter.title)
        .collect())
}
