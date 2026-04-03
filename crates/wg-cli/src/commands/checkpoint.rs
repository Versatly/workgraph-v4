//! Implementation of the `workgraph checkpoint` command.

use crate::app::AppContext;
use crate::output::CheckpointOutput;

/// Saves a durable working-context checkpoint.
///
/// # Errors
///
/// Returns an error when checkpoint persistence fails.
pub async fn handle(
    app: &AppContext,
    working_on: &str,
    focus: &str,
) -> anyhow::Result<CheckpointOutput> {
    let primitive = wg_orientation::checkpoint(app.workspace(), working_on, focus).await?;
    Ok(CheckpointOutput { primitive })
}
