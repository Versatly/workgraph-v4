//! Implementation of the `workgraph run` command family.

use anyhow::Context;
use wg_dispatch::{
    DispatchRequest, cancel_run, complete_run, create_run, fail_run, load_run, start_run,
};
use wg_types::ActorId;

use crate::app::AppContext;
use crate::args::RunCommand;
use crate::output::RunOutput;
use crate::services::codec::run_to_stored;

/// Executes run workflow commands through the dispatch kernel.
///
/// # Errors
///
/// Returns an error when validation or persistence fails.
pub async fn handle(app: &AppContext, command: RunCommand) -> anyhow::Result<RunOutput> {
    match command {
        RunCommand::Create {
            id,
            title,
            actor,
            thread,
            executor,
            mission,
            parent_run,
            summary,
        } => {
            let reference = format!("run/{id}");
            if app.dry_run() {
                return Ok(RunOutput {
                    action: "create".to_owned(),
                    dry_run: true,
                    reference,
                    run: placeholder_primitive(
                        "run",
                        &id,
                        &title,
                        "Would create queued run through the dispatch kernel.",
                    ),
                });
            }
            let run = create_run(
                app.workspace(),
                &id,
                DispatchRequest {
                    title,
                    actor_id: ActorId::new(actor),
                    executor_id: executor.map(ActorId::new),
                    thread_id: thread,
                    mission_id: mission,
                    parent_run_id: parent_run,
                    summary,
                },
            )
            .await
            .with_context(|| format!("failed to create run '{id}'"))?;
            Ok(RunOutput {
                action: "create".to_owned(),
                dry_run: false,
                reference: format!("run/{}", run.id),
                run: run_to_stored(&run)?,
            })
        }
        RunCommand::Start { run_id } => transition_or_preview(
            app,
            "start",
            &run_id,
            async { start_run(app.workspace(), &run_id).await },
        )
        .await,
        RunCommand::Complete { run_id, summary } => transition_or_preview(
            app,
            "complete",
            &run_id,
            async { complete_run(app.workspace(), &run_id, summary.as_deref()).await },
        )
        .await,
        RunCommand::Fail { run_id, summary } => transition_or_preview(
            app,
            "fail",
            &run_id,
            async { fail_run(app.workspace(), &run_id, summary.as_deref()).await },
        )
        .await,
        RunCommand::Cancel { run_id, summary } => transition_or_preview(
            app,
            "cancel",
            &run_id,
            async { cancel_run(app.workspace(), &run_id, summary.as_deref()).await },
        )
        .await,
    }
}

async fn transition_or_preview<F>(
    app: &AppContext,
    action: &str,
    run_id: &str,
    operation: F,
) -> anyhow::Result<RunOutput>
where
    F: std::future::Future<Output = wg_error::Result<wg_dispatch::Run>>,
{
    let reference = format!("run/{run_id}");
    if app.dry_run() {
        let run = load_run(app.workspace(), run_id).await.with_context(|| {
            format!("failed to load run '{run_id}' for dry-run preview")
        })?;
        return Ok(RunOutput {
            action: action.to_owned(),
            dry_run: true,
            reference,
            run: run_to_stored(&run)?,
        });
    }

    let run = operation.await?;
    Ok(RunOutput {
        action: action.to_owned(),
        dry_run: false,
        reference: format!("run/{}", run.id),
        run: run_to_stored(&run)?,
    })
}

fn placeholder_primitive(
    primitive_type: &str,
    id: &str,
    title: &str,
    body: &str,
) -> wg_store::StoredPrimitive {
    wg_store::StoredPrimitive {
        frontmatter: wg_store::PrimitiveFrontmatter {
            r#type: primitive_type.to_owned(),
            id: id.to_owned(),
            title: title.to_owned(),
            extra_fields: std::collections::BTreeMap::new(),
        },
        body: body.to_owned(),
    }
}
