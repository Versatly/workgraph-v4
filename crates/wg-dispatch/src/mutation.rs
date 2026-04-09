use chrono::Utc;
use wg_error::{Result, WorkgraphError};
use wg_paths::WorkspacePath;
use wg_policy::{PolicyAction, PolicyContext, PolicyDecision, evaluate as evaluate_policy};
use wg_store::AuditedWriteRequest;
use wg_types::{LedgerOp, RunStatus};

use crate::{DispatchRequest, RUN_TYPE, Run, load_run, save_run_with_audit, system_actor};

/// Domain mutation service for run lifecycle changes.
///
/// This service is the contract boundary for run mutations. It owns
/// operation-specific validation, policy evaluation, audited persistence, and
/// the future hook point for trigger-aware follow-up behavior.
#[derive(Debug, Clone, Copy)]
pub struct RunMutationService<'a> {
    workspace: &'a WorkspacePath,
}

impl<'a> RunMutationService<'a> {
    /// Creates a new run mutation service for a workspace.
    #[must_use]
    pub fn new(workspace: &'a WorkspacePath) -> Self {
        Self { workspace }
    }

    /// Creates and persists a queued run.
    ///
    /// # Errors
    ///
    /// Returns an error when required identifiers are invalid or persistence fails.
    pub async fn create_run(self, id: &str, request: DispatchRequest) -> Result<Run> {
        if id.trim().is_empty() {
            return Err(WorkgraphError::ValidationError(
                "run id must not be empty".to_owned(),
            ));
        }
        if request.title.trim().is_empty() {
            return Err(WorkgraphError::ValidationError(
                "run title must not be empty".to_owned(),
            ));
        }
        if request.thread_id.trim().is_empty() {
            return Err(WorkgraphError::ValidationError(
                "run thread_id must not be empty".to_owned(),
            ));
        }

        let run = Run {
            id: id.to_owned(),
            title: request.title,
            status: RunStatus::Queued,
            actor_id: request.actor_id,
            executor_id: request.executor_id,
            thread_id: request.thread_id,
            mission_id: request.mission_id,
            parent_run_id: request.parent_run_id,
            started_at: None,
            ended_at: None,
            summary: request.summary,
        };
        self.persist(
            &run,
            AuditedWriteRequest::new(system_actor(), LedgerOp::Create)
                .with_note(format!("Created run '{}'", run.id)),
        )
        .await?;
        Ok(run)
    }

    /// Marks a queued or retryable run as running.
    ///
    /// # Errors
    ///
    /// Returns an error when the transition is invalid or persistence fails.
    pub async fn start_run(self, run_id: &str) -> Result<Run> {
        self.transition_run(run_id, RunStatus::Running, LedgerOp::Start, None)
            .await
    }

    /// Marks a run succeeded.
    ///
    /// # Errors
    ///
    /// Returns an error when the transition is invalid or persistence fails.
    pub async fn complete_run(self, run_id: &str, summary: Option<&str>) -> Result<Run> {
        self.transition_run(run_id, RunStatus::Succeeded, LedgerOp::Done, summary)
            .await
    }

    /// Marks a run failed.
    ///
    /// # Errors
    ///
    /// Returns an error when the transition is invalid or persistence fails.
    pub async fn fail_run(self, run_id: &str, summary: Option<&str>) -> Result<Run> {
        self.transition_run(run_id, RunStatus::Failed, LedgerOp::Update, summary)
            .await
    }

    /// Marks a run cancelled.
    ///
    /// # Errors
    ///
    /// Returns an error when the transition is invalid or persistence fails.
    pub async fn cancel_run(self, run_id: &str, summary: Option<&str>) -> Result<Run> {
        self.transition_run(run_id, RunStatus::Cancelled, LedgerOp::Cancel, summary)
            .await
    }

    async fn transition_run(
        self,
        run_id: &str,
        next: RunStatus,
        op: LedgerOp,
        summary: Option<&str>,
    ) -> Result<Run> {
        let mut run = load_run(self.workspace, run_id).await?;
        run.status = run
            .status
            .transition_to(next)
            .map_err(WorkgraphError::ValidationError)?;
        let now = Utc::now();
        match run.status {
            RunStatus::Running => {
                if run.started_at.is_none() {
                    run.started_at = Some(now);
                }
                run.ended_at = None;
            }
            RunStatus::Succeeded
            | RunStatus::Failed
            | RunStatus::TimedOut
            | RunStatus::Cancelled => {
                if run.started_at.is_none() {
                    run.started_at = Some(now);
                }
                run.ended_at = Some(now);
            }
            RunStatus::Queued => {
                run.ended_at = None;
            }
        }
        if let Some(summary) = summary {
            run.summary = Some(summary.to_owned());
        }
        self.persist(
            &run,
            AuditedWriteRequest::new(system_actor(), op).with_note(format!(
                "Transitioned run '{}' to '{}'",
                run.id,
                run.status.as_str()
            )),
        )
        .await?;
        Ok(run)
    }

    async fn persist(self, run: &Run, audit: AuditedWriteRequest) -> Result<()> {
        self.authorize(run.id.as_str(), &audit).await?;
        save_run_with_audit(self.workspace, run, audit.clone()).await?;
        self.after_mutation(run, &audit).await
    }

    async fn authorize(self, run_id: &str, audit: &AuditedWriteRequest) -> Result<()> {
        let action = policy_action_for(audit.op);
        let decision = evaluate_policy(
            self.workspace,
            &audit.actor,
            action,
            RUN_TYPE,
            &PolicyContext::default(),
        )
        .await?;
        if decision == PolicyDecision::Deny {
            return Err(WorkgraphError::ValidationError(format!(
                "policy denied {} of {RUN_TYPE}/{run_id} for actor '{}'",
                policy_action_label(action),
                audit.actor
            )));
        }
        Ok(())
    }

    async fn after_mutation(self, _run: &Run, _audit: &AuditedWriteRequest) -> Result<()> {
        // Reserved for future trigger-aware follow-up hooks.
        Ok(())
    }
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
