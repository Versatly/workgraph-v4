//! Implementation of `workgraph thread ...` commands.

use anyhow::Context;
use wg_store::read_primitive;
use wg_thread::{
    add_completion_action, add_evidence, add_exit_criterion, add_message, add_update_action,
    claim_thread, complete_thread, create_thread, open_thread,
};
use wg_types::{
    ActorId, CoordinationAction, EvidenceItem, ThreadExitCriterion, ThreadPrimitive, ThreadStatus,
};

use crate::app::AppContext;
use crate::args::ThreadCommand;
use crate::output::ThreadOutput;
use crate::services::codec::thread_to_stored;

/// Executes a thread workflow command.
///
/// # Errors
///
/// Returns an error when validation fails or the underlying thread mutation cannot be completed.
pub async fn handle(app: &AppContext, command: ThreadCommand) -> anyhow::Result<ThreadOutput> {
    match command {
        ThreadCommand::Create {
            id,
            title,
            parent_mission_id,
        } => {
            let reference = format!("thread/{id}");
            if app.dry_run() {
                return Ok(ThreadOutput {
                    action: "create".to_owned(),
                    dry_run: true,
                    reference,
                    thread: thread_to_stored(&ThreadPrimitive {
                        id,
                        title,
                        status: ThreadStatus::Draft,
                        assigned_actor: None,
                        parent_mission_id,
                        exit_criteria: Vec::new(),
                        evidence: Vec::new(),
                        update_actions: Vec::new(),
                        completion_actions: Vec::new(),
                        messages: Vec::new(),
                    })?,
                });
            }

            create_thread(app.workspace(), &id, &title, parent_mission_id.as_deref())
                .await
                .with_context(|| format!("failed to create thread '{id}'"))?;
            load_output(app, "create", &reference).await
        }
        ThreadCommand::Open { thread_id } => {
            let reference = format!("thread/{thread_id}");
            if app.dry_run() {
                return dry_run_from_existing_or_placeholder(
                    app,
                    "open",
                    &thread_id,
                    ThreadStatus::Ready,
                )
                .await;
            }

            open_thread(app.workspace(), &thread_id)
                .await
                .with_context(|| format!("failed to open thread '{thread_id}'"))?;
            load_output(app, "open", &reference).await
        }
        ThreadCommand::Claim { thread_id, actor } => {
            let reference = format!("thread/{thread_id}");
            if app.dry_run() {
                return dry_run_from_existing_or_placeholder(
                    app,
                    "claim",
                    &thread_id,
                    ThreadStatus::Active,
                )
                .await;
            }

            claim_thread(app.workspace(), &thread_id, ActorId::new(actor))
                .await
                .with_context(|| format!("failed to claim thread '{thread_id}'"))?;
            load_output(app, "claim", &reference).await
        }
        ThreadCommand::AddExitCriterion {
            thread_id,
            id,
            title,
            description,
            reference,
            optional,
        } => {
            let thread_reference = format!("thread/{thread_id}");
            if app.dry_run() {
                return dry_run_existing(app, "add_exit_criterion", &thread_reference).await;
            }

            add_exit_criterion(
                app.workspace(),
                &thread_id,
                ThreadExitCriterion {
                    id,
                    title,
                    description,
                    required: !optional,
                    reference,
                },
            )
            .await
            .with_context(|| format!("failed to add exit criterion to thread '{thread_id}'"))?;
            load_output(app, "add_exit_criterion", &thread_reference).await
        }
        ThreadCommand::AddEvidence {
            thread_id,
            id,
            title,
            description,
            reference,
            satisfies,
            source,
        } => {
            let thread_reference = format!("thread/{thread_id}");
            if app.dry_run() {
                return dry_run_existing(app, "add_evidence", &thread_reference).await;
            }

            add_evidence(
                app.workspace(),
                &thread_id,
                EvidenceItem {
                    id,
                    title,
                    description,
                    reference,
                    satisfies,
                    recorded_at: None,
                    source,
                },
            )
            .await
            .with_context(|| format!("failed to add evidence to thread '{thread_id}'"))?;
            load_output(app, "add_evidence", &thread_reference).await
        }
        ThreadCommand::AddUpdateAction {
            thread_id,
            id,
            title,
            kind,
            target_reference,
            description,
        } => {
            let reference = format!("thread/{thread_id}");
            if app.dry_run() {
                return dry_run_existing(app, "add_update_action", &reference).await;
            }

            add_update_action(
                app.workspace(),
                &thread_id,
                CoordinationAction {
                    id,
                    title,
                    kind,
                    target_reference,
                    description,
                },
            )
            .await
            .with_context(|| format!("failed to add update action to thread '{thread_id}'"))?;
            load_output(app, "add_update_action", &reference).await
        }
        ThreadCommand::AddCompletionAction {
            thread_id,
            id,
            title,
            kind,
            target_reference,
            description,
        } => {
            let reference = format!("thread/{thread_id}");
            if app.dry_run() {
                return dry_run_existing(app, "add_completion_action", &reference).await;
            }

            add_completion_action(
                app.workspace(),
                &thread_id,
                CoordinationAction {
                    id,
                    title,
                    kind,
                    target_reference,
                    description,
                },
            )
            .await
            .with_context(|| format!("failed to add completion action to thread '{thread_id}'"))?;
            load_output(app, "add_completion_action", &reference).await
        }
        ThreadCommand::AddMessage {
            thread_id,
            actor,
            text,
        } => {
            let reference = format!("thread/{thread_id}");
            if app.dry_run() {
                return dry_run_existing(app, "add_message", &reference).await;
            }

            add_message(app.workspace(), &thread_id, ActorId::new(actor), &text)
                .await
                .with_context(|| format!("failed to append message to thread '{thread_id}'"))?;
            load_output(app, "add_message", &reference).await
        }
        ThreadCommand::Complete { thread_id } => {
            let reference = format!("thread/{thread_id}");
            if app.dry_run() {
                return dry_run_existing(app, "complete", &reference).await;
            }

            complete_thread(app.workspace(), &thread_id)
                .await
                .with_context(|| format!("failed to complete thread '{thread_id}'"))?;
            load_output(app, "complete", &reference).await
        }
    }
}

async fn load_output(
    app: &AppContext,
    action: &str,
    reference: &str,
) -> anyhow::Result<ThreadOutput> {
    let (primitive_type, id) = reference
        .split_once('/')
        .expect("thread references should be well-formed");
    let thread = read_primitive(app.workspace(), primitive_type, id)
        .await
        .with_context(|| format!("failed to load '{reference}' after mutation"))?;
    Ok(ThreadOutput {
        action: action.to_owned(),
        dry_run: false,
        reference: reference.to_owned(),
        thread,
    })
}

async fn dry_run_existing(
    app: &AppContext,
    action: &str,
    reference: &str,
) -> anyhow::Result<ThreadOutput> {
    let (primitive_type, id) = reference
        .split_once('/')
        .expect("thread references should be well-formed");
    let thread = read_primitive(app.workspace(), primitive_type, id)
        .await
        .with_context(|| format!("failed to load '{reference}' for dry-run preview"))?;
    Ok(ThreadOutput {
        action: action.to_owned(),
        dry_run: true,
        reference: reference.to_owned(),
        thread,
    })
}

async fn dry_run_from_existing_or_placeholder(
    app: &AppContext,
    action: &str,
    thread_id: &str,
    status: ThreadStatus,
) -> anyhow::Result<ThreadOutput> {
    let reference = format!("thread/{thread_id}");
    match dry_run_existing(app, action, &reference).await {
        Ok(output) => Ok(output),
        Err(_) => Ok(ThreadOutput {
            action: action.to_owned(),
            dry_run: true,
            reference,
            thread: thread_to_stored(&ThreadPrimitive {
                id: thread_id.to_owned(),
                title: format!("Thread {thread_id}"),
                status,
                assigned_actor: None,
                parent_mission_id: None,
                exit_criteria: Vec::new(),
                evidence: Vec::new(),
                update_actions: Vec::new(),
                completion_actions: Vec::new(),
                messages: Vec::new(),
            })?,
        }),
    }
}
