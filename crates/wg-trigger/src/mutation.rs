use wg_error::{Result, WorkgraphError};
use wg_paths::WorkspacePath;
use wg_policy::{PolicyAction, PolicyContext, PolicyDecision, evaluate as evaluate_policy};
use wg_store::AuditedWriteRequest;
use wg_types::{LedgerOp, TriggerPrimitive};

use crate::{TRIGGER_TYPE, Trigger, save_trigger_with_audit, validate_trigger_definition};

/// Domain mutation service for durable trigger definitions.
///
/// This service is the contract boundary for trigger mutations. It owns
/// trigger-specific validation, policy evaluation, audited persistence, and the
/// future hook point for post-write trigger infrastructure.
#[derive(Debug, Clone, Copy)]
pub struct TriggerMutationService<'a> {
    workspace: &'a WorkspacePath,
}

impl<'a> TriggerMutationService<'a> {
    /// Creates a new trigger mutation service for a workspace.
    #[must_use]
    pub fn new(workspace: &'a WorkspacePath) -> Self {
        Self { workspace }
    }

    /// Persists a trigger after validating its contract.
    ///
    /// # Errors
    ///
    /// Returns an error when validation or persistence fails.
    pub async fn save_trigger(self, trigger: &Trigger) -> Result<()> {
        validate_trigger_definition(trigger)?;
        let op = if trigger_exists(self.workspace, &trigger.id).await? {
            LedgerOp::Update
        } else {
            LedgerOp::Create
        };
        let audit = AuditedWriteRequest::new(system_actor(), op)
            .with_note(format!("Saved trigger '{}'", trigger.id));
        self.persist(trigger, audit).await
    }

    async fn persist(self, trigger: &TriggerPrimitive, audit: AuditedWriteRequest) -> Result<()> {
        self.authorize(trigger.id.as_str(), &audit).await?;
        save_trigger_with_audit(self.workspace, trigger, audit.clone()).await?;
        self.after_mutation(trigger, &audit).await
    }

    async fn authorize(self, trigger_id: &str, audit: &AuditedWriteRequest) -> Result<()> {
        let action = policy_action_for(audit.op);
        let decision = evaluate_policy(
            self.workspace,
            &audit.actor,
            action,
            TRIGGER_TYPE,
            &PolicyContext::default(),
        )
        .await?;
        if decision == PolicyDecision::Deny {
            return Err(WorkgraphError::ValidationError(format!(
                "policy denied {} of {TRIGGER_TYPE}/{trigger_id} for actor '{}'",
                policy_action_label(action),
                audit.actor
            )));
        }
        Ok(())
    }

    async fn after_mutation(
        self,
        _trigger: &TriggerPrimitive,
        _audit: &AuditedWriteRequest,
    ) -> Result<()> {
        // Reserved for future trigger runtime update hooks.
        Ok(())
    }
}

async fn trigger_exists(workspace: &WorkspacePath, trigger_id: &str) -> Result<bool> {
    match crate::load_trigger(workspace, trigger_id).await {
        Ok(_) => Ok(true),
        Err(WorkgraphError::IoError(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(false)
        }
        Err(other) => Err(other),
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

fn system_actor() -> wg_types::ActorId {
    wg_types::ActorId::new("system:workgraph")
}
