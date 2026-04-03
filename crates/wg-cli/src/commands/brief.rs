//! Implementation of the `workgraph brief` command.

use wg_orientation::ContextLens;

use crate::app::AppContext;
use crate::output::{BriefOutput, WorkspaceIdentity};
use crate::services::orientation::build_workspace_brief;

/// Produces a structured agent-first workspace orientation brief.
///
/// # Errors
///
/// Returns an error when workspace metadata, primitives, or ledger entries cannot be read.
pub async fn handle(app: &AppContext, _lens: ContextLens) -> anyhow::Result<BriefOutput> {
    let orientation = build_workspace_brief(app).await?;
    let recent_ledger_entries = app
        .read_ledger_entries()
        .await?
        .into_iter()
        .rev()
        .take(10)
        .collect::<Vec<_>>();

    Ok(BriefOutput {
        workspace: WorkspaceIdentity {
            id: orientation.workspace_id.clone(),
            name: orientation.workspace_name.clone(),
            root: orientation.workspace_root.clone(),
            default_actor_id: orientation.default_actor_id.clone(),
        },
        primitive_counts: orientation.type_counts.clone(),
        recent_ledger_entries,
        suggested_next_actions: vec![
            "workgraph show org/versatly".to_owned(),
            "workgraph query org".to_owned(),
            "workgraph create org --title \"Versatly\"".to_owned(),
        ],
        orientation,
    })
}
