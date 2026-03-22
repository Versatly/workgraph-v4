//! Implementation of the `workgraph status` command.

use crate::app::AppContext;
use crate::output::StatusOutput;

/// Collects primitive counts, graph issues, and evidence gaps for the workspace.
///
/// # Errors
///
/// Returns an error when workspace metadata, ledger, or orientation data cannot be read.
pub async fn handle(app: &AppContext) -> anyhow::Result<StatusOutput> {
    let config = app.load_config().await?;
    let workspace_status = wg_orientation::status(app.workspace()).await?;
    let entries = app.read_ledger_entries().await?;

    Ok(StatusOutput {
        config,
        workspace_root: app.root().display().to_string(),
        type_counts: workspace_status.type_counts,
        recent_activity: workspace_status.recent_activity,
        last_entry: entries.last().cloned(),
        graph_issues: workspace_status.graph_issues,
        thread_evidence_gaps: workspace_status.thread_evidence_gaps,
    })
}
