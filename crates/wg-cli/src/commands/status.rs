//! Implementation of the `workgraph status` command.

use crate::app::AppContext;
use crate::output::StatusOutput;

/// Collects primitive counts, graph issues, and evidence gaps for the workspace.
///
/// # Errors
///
/// Returns an error when workspace metadata, ledger, or orientation data cannot be read.
pub async fn handle(app: &AppContext) -> anyhow::Result<StatusOutput> {
    let mut config = app.load_config().await?;
    if let Some(actor) = app.actor_override() {
        config.default_actor_id = Some(actor.clone());
    }
    let workspace_status = wg_orientation::status(app.workspace()).await?;
    let mut entries = app.read_ledger_entries().await?;
    entries.reverse();

    Ok(StatusOutput {
        config,
        workspace_root: app.root().display().to_string(),
        type_counts: workspace_status.type_counts,
        recent_activity: workspace_status.recent_activity,
        last_entry: entries.first().cloned(),
        graph_issues: workspace_status.graph_issues,
        orphan_nodes: workspace_status.orphan_nodes,
        thread_evidence_gaps: workspace_status.thread_evidence_gaps,
        trigger_health: workspace_status.trigger_health,
        recent_trigger_receipts: workspace_status.recent_trigger_receipts,
        pending_trigger_actions: workspace_status.pending_trigger_actions,
    })
}
