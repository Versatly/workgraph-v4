//! Implementation of the `workgraph checkpoint` command.

use crate::app::AppContext;
use crate::output::CheckpointOutput;
use crate::services::codec::checkpoint_to_stored;

/// Saves a checkpoint primitive describing the current work focus.
///
/// # Errors
///
/// Returns an error when the checkpoint cannot be persisted.
pub async fn handle(
    app: &AppContext,
    working_on: &str,
    focus: &str,
) -> anyhow::Result<CheckpointOutput> {
    if app.dry_run() {
        let checkpoint = wg_types::CheckpointPrimitive {
            id: "dry-run-checkpoint".to_owned(),
            title: format!("Checkpoint: {working_on}"),
            working_on: working_on.to_owned(),
            focus: focus.to_owned(),
            actor_id: None,
            created_at: chrono::Utc::now(),
        };
        let primitive = checkpoint_to_stored(&checkpoint)?;
        return Ok(CheckpointOutput {
            action: "checkpoint".to_owned(),
            dry_run: true,
            reference: "checkpoint/dry-run-checkpoint".to_owned(),
            checkpoint: primitive,
        });
    }

    let primitive = wg_orientation::checkpoint(app.workspace(), working_on, focus).await?;
    Ok(CheckpointOutput {
        action: "checkpoint".to_owned(),
        dry_run: false,
        reference: format!(
            "{}/{}",
            primitive.frontmatter.r#type, primitive.frontmatter.id
        ),
        checkpoint: primitive,
    })
}
