//! Implementation of the `workgraph brief` command.

use wg_orientation::{ContextLens, WorkspaceBrief};

use crate::app::AppContext;
use crate::services::orientation::build_workspace_brief;

/// Produces a structured agent-first workspace orientation brief.
///
/// # Errors
///
/// Returns an error when workspace metadata, primitives, or ledger entries cannot be read.
pub async fn handle(app: &AppContext, lens: ContextLens) -> anyhow::Result<WorkspaceBrief> {
    build_workspace_brief(app, lens).await
}
