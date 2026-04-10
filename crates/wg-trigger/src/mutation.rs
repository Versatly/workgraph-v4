use wg_error::{Result, WorkgraphError};
use wg_paths::WorkspacePath;
use wg_policy::{PolicyAction, PolicyContext, PolicyDecision, evaluate as evaluate_policy};
use wg_store::AuditedWriteRequest;
use wg_types::{
    ActorId, EventEnvelope, LedgerEntry, LedgerOp, TriggerPrimitive, TriggerReceiptPrimitive,
    TriggerSubscriptionState,
};

use crate::{
    TRIGGER_TYPE, Trigger, TriggerReceipt, dedup_key, save_trigger_receipt_with_audit,
    save_trigger_with_audit, trigger_receipt_id, validate_trigger_definition,
};

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
        self.save_trigger_as(trigger, system_actor()).await?;
        Ok(())
    }

    /// Persists a trigger after validating its contract and auditing the invoking actor.
    ///
    /// # Errors
    ///
    /// Returns an error when validation or persistence fails.
    pub async fn save_trigger_as(self, trigger: &Trigger, audit_actor: ActorId) -> Result<()> {
        validate_trigger_definition(trigger)?;
        let op = if trigger_exists(self.workspace, &trigger.id).await? {
            LedgerOp::Update
        } else {
            LedgerOp::Create
        };
        let audit = AuditedWriteRequest::new(audit_actor, op)
            .with_note(format!("Saved trigger '{}'", trigger.id));
        self.persist(trigger, audit).await
    }

    /// Evaluates a normalized event envelope and persists any resulting trigger receipts.
    ///
    /// # Errors
    ///
    /// Returns an error when evaluation or persistence fails.
    pub async fn ingest_event(self, event: &EventEnvelope) -> Result<Vec<TriggerReceipt>> {
        let matches = crate::evaluate_event(self.workspace, event).await?;
        let mut receipts = Vec::with_capacity(matches.len());
        for matched in matches {
            let receipt_id = trigger_receipt_id(&matched.trigger_id, &matched.event.id);
            let receipt = TriggerReceiptPrimitive {
                id: receipt_id.clone(),
                title: format!("Trigger receipt: {}", matched.title),
                trigger_id: matched.trigger_id.clone(),
                trigger_title: matched.title.clone(),
                event_id: matched.event.id.clone(),
                event_source: matched.event.source,
                event_name: matched.event.event_name.clone(),
                provider: matched.event.provider.clone(),
                actor_id: matched.event.actor_id.clone(),
                subject_reference: matched.event.subject_reference.clone(),
                occurred_at: matched.event.occurred_at,
                dedup_key: dedup_key(&matched.trigger_id, &matched.event.id),
                field_names: matched.event.field_names.clone(),
                payload_fields: matched.event.payload_fields.clone(),
                action_outcomes: matched.action_outcomes.clone(),
            };
            let persisted = if trigger_receipt_exists(self.workspace, &receipt.id).await? {
                crate::load_trigger_receipt(self.workspace, &receipt.id).await?
            } else {
                let audit = AuditedWriteRequest::new(system_actor(), LedgerOp::Create)
                    .with_note(format!("Recorded trigger receipt '{}'", receipt.id));
                save_trigger_receipt_with_audit(self.workspace, &receipt, audit).await?;
                receipt
            };
            self.update_subscription_state(&persisted).await?;
            receipts.push(persisted);
        }
        Ok(receipts)
    }

    async fn persist(self, trigger: &TriggerPrimitive, audit: AuditedWriteRequest) -> Result<()> {
        self.authorize(trigger.id.as_str(), &audit).await?;
        let ledger_entry = save_trigger_with_audit(self.workspace, trigger, audit.clone()).await?;
        self.after_mutation(trigger, &audit, &ledger_entry).await
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
        ledger_entry: &LedgerEntry,
    ) -> Result<()> {
        crate::ingest_ledger_entry(self.workspace, ledger_entry).await?;
        Ok(())
    }

    async fn update_subscription_state(self, receipt: &TriggerReceipt) -> Result<()> {
        let mut trigger = crate::load_trigger(self.workspace, &receipt.trigger_id).await?;
        let subscription_state =
            trigger
                .subscription_state
                .get_or_insert(TriggerSubscriptionState {
                    last_evaluated_at: None,
                    last_matched_at: None,
                    last_event_id: None,
                    last_event_name: None,
                    last_event_cursor: None,
                    last_receipt_id: None,
                });
        subscription_state.last_evaluated_at = Some(receipt.occurred_at);
        subscription_state.last_matched_at = Some(receipt.occurred_at);
        subscription_state.last_event_id = Some(receipt.event_id.clone());
        subscription_state.last_event_name = receipt.event_name.clone();
        subscription_state.last_event_cursor = Some(receipt.event_id.clone());
        subscription_state.last_receipt_id = Some(receipt.id.clone());
        save_trigger_with_audit(
            self.workspace,
            &trigger,
            AuditedWriteRequest::new(system_actor(), LedgerOp::Update).with_note(format!(
                "Updated trigger subscription state for '{}'",
                trigger.id
            )),
        )
        .await?;
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

async fn trigger_receipt_exists(workspace: &WorkspacePath, receipt_id: &str) -> Result<bool> {
    match crate::load_trigger_receipt(workspace, receipt_id).await {
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
