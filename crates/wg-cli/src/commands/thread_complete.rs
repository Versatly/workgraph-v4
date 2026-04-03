//! Implementation of the `workgraph complete` command.

use anyhow::Context;

use crate::app::AppContext;
use crate::output::ThreadCompleteOutput;

/// Completes a thread after validating required evidence criteria.
///
/// # Errors
///
/// Returns an error when the thread cannot be loaded, evidence is missing, or
/// completion persistence fails.
pub async fn handle(app: &AppContext, thread_id: &str) -> anyhow::Result<ThreadCompleteOutput> {
    let thread = wg_thread::complete_thread(app.workspace(), thread_id)
        .await
        .with_context(|| format!("failed to complete thread '{thread_id}'"))?;

    Ok(ThreadCompleteOutput { thread })
}
