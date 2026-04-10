use wg_error::{Result, WorkgraphError};
use wg_paths::WorkspacePath;
use wg_policy::{PolicyAction, PolicyContext, PolicyDecision, evaluate as evaluate_policy};
use wg_store::AuditedWriteRequest;
use wg_trigger::ingest_ledger_entry;
use wg_types::LedgerOp;

use crate::{THREAD_TYPE, Thread, save_thread_with_audit};

pub(super) async fn persist_thread(
    workspace: &WorkspacePath,
    thread: &Thread,
    audit: AuditedWriteRequest,
) -> Result<()> {
    authorize_thread_mutation(workspace, thread.id.as_str(), &audit).await?;
    let ledger_entry = save_thread_with_audit(workspace, thread, audit.clone()).await?;
    after_thread_mutation(workspace, thread, &audit, &ledger_entry).await
}

async fn authorize_thread_mutation(
    workspace: &WorkspacePath,
    thread_id: &str,
    audit: &AuditedWriteRequest,
) -> Result<()> {
    let action = policy_action_for(audit.op);
    let decision = evaluate_policy(
        workspace,
        &audit.actor,
        action,
        THREAD_TYPE,
        &PolicyContext::default(),
    )
    .await?;
    if decision == PolicyDecision::Deny {
        return Err(WorkgraphError::ValidationError(format!(
            "policy denied {} of {THREAD_TYPE}/{thread_id} for actor '{}'",
            policy_action_label(action),
            audit.actor
        )));
    }
    Ok(())
}

async fn after_thread_mutation(
    workspace: &WorkspacePath,
    _thread: &Thread,
    _audit: &AuditedWriteRequest,
    ledger_entry: &wg_types::LedgerEntry,
) -> Result<()> {
    ingest_ledger_entry(workspace, ledger_entry).await?;
    Ok(())
}

fn policy_action_for(op: LedgerOp) -> PolicyAction {
    match op {
        LedgerOp::Create => PolicyAction::Create,
        LedgerOp::Update
        | LedgerOp::Delete
        | LedgerOp::Claim
        | LedgerOp::Release
        | LedgerOp::Start
        | LedgerOp::Done
        | LedgerOp::Cancel
        | LedgerOp::Reopen
        | LedgerOp::Assign
        | LedgerOp::Unassign => PolicyAction::Update,
    }
}

fn policy_action_label(action: PolicyAction) -> &'static str {
    match action {
        PolicyAction::Create => "create",
        PolicyAction::Read => "read",
        PolicyAction::Update => "update",
        PolicyAction::Delete => "delete",
    }
}
