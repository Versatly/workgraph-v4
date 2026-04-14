//! Implementation of the `workgraph claim` command.

use anyhow::Context;
use wg_thread::claim_thread;

use crate::app::AppContext;
use crate::output::ThreadClaimOutput;

/// Claims a thread for the configured actor.
///
/// # Errors
///
/// Returns an error when actor configuration is missing, the thread cannot be
/// claimed, or thread loading fails.
pub async fn handle(app: &AppContext, thread_id: &str) -> anyhow::Result<ThreadClaimOutput> {
    let actor = app.effective_actor_id().await?;
    let thread = claim_thread(app.workspace(), thread_id, actor)
        .await
        .with_context(|| format!("failed to claim thread '{thread_id}'"))?;
    Ok(ThreadClaimOutput { thread })
}
