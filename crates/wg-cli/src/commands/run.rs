//! Implementation of the `workgraph run` command family.

use anyhow::Context;
use tokio::fs;
use wg_dispatch::{DispatchRequest, Run, load_run};
use wg_types::{ActorId, LedgerEntry, RunStatus};

use crate::app::AppContext;
use crate::output::{RunCreateOutcome, RunCreateOutput, RunLifecycleOutput};
use crate::util::slug::{slugify, unique_slug};

/// Parsed arguments for `workgraph run create`.
#[derive(Debug, Clone)]
pub struct RunCreateArgs {
    /// Human-readable title for the run.
    pub title: String,
    /// Owning thread identifier.
    pub thread_id: String,
    /// Optional actor override.
    pub actor_id: Option<String>,
    /// Optional broad run classification.
    pub kind: Option<String>,
    /// Optional source label.
    pub source: Option<String>,
    /// Optional executor actor id.
    pub executor_id: Option<String>,
    /// Optional mission identifier.
    pub mission_id: Option<String>,
    /// Optional parent run identifier.
    pub parent_run_id: Option<String>,
    /// Optional summary.
    pub summary: Option<String>,
    /// Whether to preview without persisting.
    pub dry_run: bool,
}

/// Creates a new queued run.
///
/// # Errors
///
/// Returns an error when configuration cannot be loaded, the run payload is
/// invalid, or persistence fails.
pub async fn create(app: &AppContext, args: RunCreateArgs) -> anyhow::Result<RunCreateOutput> {
    handle_create(
        app,
        &args.title,
        &args.thread_id,
        args.actor_id.as_deref(),
        args.kind.as_deref(),
        args.source.as_deref(),
        args.executor_id.as_deref(),
        args.mission_id.as_deref(),
        args.parent_run_id.as_deref(),
        args.summary.as_deref(),
        args.dry_run,
    )
    .await
}

/// Starts a queued run.
///
/// # Errors
///
/// Returns an error when the run cannot be transitioned.
pub async fn start(app: &AppContext, run_id: &str) -> anyhow::Result<RunLifecycleOutput> {
    let actor = resolve_actor(app, None).await?;
    let run = wg_dispatch::start_run_as(app.workspace(), actor, run_id)
        .await
        .with_context(|| format!("failed to start run '{run_id}'"))?;
    Ok(run_transition_output("Started", run))
}

/// Completes a run successfully.
///
/// # Errors
///
/// Returns an error when the run cannot be transitioned.
pub async fn complete(
    app: &AppContext,
    run_id: &str,
    summary: Option<&str>,
) -> anyhow::Result<RunLifecycleOutput> {
    let actor = resolve_actor(app, None).await?;
    let run = wg_dispatch::complete_run_as(app.workspace(), actor, run_id, summary)
        .await
        .with_context(|| format!("failed to complete run '{run_id}'"))?;
    Ok(run_transition_output("Completed", run))
}

/// Marks a run as failed.
///
/// # Errors
///
/// Returns an error when the run cannot be transitioned.
pub async fn fail(
    app: &AppContext,
    run_id: &str,
    summary: Option<&str>,
) -> anyhow::Result<RunLifecycleOutput> {
    let actor = resolve_actor(app, None).await?;
    let run = wg_dispatch::fail_run_as(app.workspace(), actor, run_id, summary)
        .await
        .with_context(|| format!("failed to fail run '{run_id}'"))?;
    Ok(run_transition_output("Failed", run))
}

/// Cancels a run.
///
/// # Errors
///
/// Returns an error when the run cannot be transitioned.
pub async fn cancel(
    app: &AppContext,
    run_id: &str,
    summary: Option<&str>,
) -> anyhow::Result<RunLifecycleOutput> {
    let actor = resolve_actor(app, None).await?;
    let run = wg_dispatch::cancel_run_as(app.workspace(), actor, run_id, summary)
        .await
        .with_context(|| format!("failed to cancel run '{run_id}'"))?;
    Ok(run_transition_output("Cancelled", run))
}

async fn handle_create(
    app: &AppContext,
    title: &str,
    thread_id: &str,
    actor_id: Option<&str>,
    kind: Option<&str>,
    source: Option<&str>,
    executor_id: Option<&str>,
    mission_id: Option<&str>,
    parent_run_id: Option<&str>,
    summary: Option<&str>,
    dry_run: bool,
) -> anyhow::Result<RunCreateOutput> {
    let invoking_actor = default_actor(app).await?;
    let responsible_actor = actor_id
        .map(ActorId::new)
        .unwrap_or_else(|| invoking_actor.clone());
    let request = build_request(
        title,
        thread_id,
        responsible_actor,
        kind,
        source,
        executor_id,
        mission_id,
        parent_run_id,
        summary,
    );
    let mut run = Run {
        id: slugify(title),
        title: request.title.clone(),
        status: RunStatus::Queued,
        kind: request.kind.clone(),
        source: request.source.clone(),
        actor_id: request.actor_id.clone(),
        executor_id: request.executor_id.clone(),
        thread_id: request.thread_id.clone(),
        mission_id: request.mission_id.clone(),
        parent_run_id: request.parent_run_id.clone(),
        started_at: None,
        ended_at: None,
        summary: request.summary.clone(),
        external_refs: request.external_refs.clone(),
    };
    let mut run_path = app.workspace().primitive_path("run", &run.id);

    if fs::try_exists(run_path.as_path())
        .await
        .context("failed to inspect existing run path")?
    {
        let reference = format!("run/{}", run.id);
        let existing = load_run(app.workspace(), &run.id)
            .await
            .with_context(|| format!("failed to read existing run '{reference}'"))?;
        if existing == run {
            return Ok(RunCreateOutput {
                outcome: RunCreateOutcome::Noop,
                reference,
                path: run_path.as_path().display().to_string(),
                run,
                ledger_entry: None,
            });
        }

        let unique_id = unique_slug(app.workspace(), "run", title).await?;
        run.id = unique_id;
        run_path = app.workspace().primitive_path("run", &run.id);
    }

    let reference = format!("run/{}", run.id);
    let path = run_path.as_path().display().to_string();
    if dry_run {
        return Ok(RunCreateOutput {
            outcome: RunCreateOutcome::DryRun,
            reference,
            path,
            run,
            ledger_entry: None,
        });
    }

    let created = wg_dispatch::create_run_as(app.workspace(), invoking_actor, &run.id, request)
        .await
        .with_context(|| format!("failed to create run '{}'", run.id))?;
    let entry = latest_run_ledger_entry(app, &created.id).await?;

    Ok(RunCreateOutput {
        outcome: RunCreateOutcome::Created,
        reference,
        path,
        run: created,
        ledger_entry: Some(entry),
    })
}

async fn resolve_actor(app: &AppContext, actor_id: Option<&str>) -> anyhow::Result<ActorId> {
    if let Some(actor_id) = actor_id {
        return Ok(ActorId::new(actor_id));
    }

    default_actor(app).await
}

async fn default_actor(app: &AppContext) -> anyhow::Result<ActorId> {
    let config = app.load_config().await?;
    Ok(config.default_actor_id.unwrap_or_else(|| ActorId::new("cli")))
}

fn build_request(
    title: &str,
    thread_id: &str,
    actor_id: ActorId,
    kind: Option<&str>,
    source: Option<&str>,
    executor_id: Option<&str>,
    mission_id: Option<&str>,
    parent_run_id: Option<&str>,
    summary: Option<&str>,
) -> DispatchRequest {
    DispatchRequest {
        title: title.to_owned(),
        kind: kind.map(ToOwned::to_owned),
        source: source.map(ToOwned::to_owned),
        actor_id,
        executor_id: executor_id.map(ActorId::new),
        thread_id: thread_id.to_owned(),
        mission_id: mission_id.map(ToOwned::to_owned),
        parent_run_id: parent_run_id.map(ToOwned::to_owned),
        summary: summary.map(ToOwned::to_owned),
        external_refs: Vec::new(),
    }
}

async fn latest_run_ledger_entry(
    app: &AppContext,
    run_id: &str,
) -> anyhow::Result<LedgerEntry> {
    let entries = app.read_ledger_entries().await?;
    entries
        .into_iter()
        .rev()
        .find(|entry| entry.primitive_type == "run" && entry.primitive_id == run_id)
        .with_context(|| format!("failed to locate ledger entry for run '{run_id}'"))
}

fn run_transition_output(action: &str, run: Run) -> RunLifecycleOutput {
    RunLifecycleOutput {
        action: action.to_owned(),
        run,
    }
}
