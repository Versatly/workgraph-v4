use std::collections::BTreeSet;

use chrono::Utc;
use wg_error::{Result, WorkgraphError};
use wg_paths::WorkspacePath;
use wg_policy::{PolicyAction, PolicyContext, PolicyDecision, evaluate as evaluate_policy};
use wg_store::AuditedWriteRequest;
use wg_types::{
    ActorId, ConversationMessage, CoordinationAction, EvidenceItem, LedgerOp, ThreadExitCriterion,
    ThreadStatus,
};

use crate::{
    THREAD_TYPE, Thread, infer_message_kind, load_thread, save_thread_with_audit, system_actor,
    unsatisfied_exit_criteria,
};

/// Domain mutation service for evidence-bearing thread lifecycle changes.
///
/// This service is the contract boundary for thread mutations. It owns
/// operation-specific validation, policy evaluation, audited persistence, and
/// the future hook point for trigger-aware follow-up behavior.
#[derive(Debug, Clone, Copy)]
pub struct ThreadMutationService<'a> {
    workspace: &'a WorkspacePath,
}

impl<'a> ThreadMutationService<'a> {
    /// Creates a new thread mutation service for a workspace.
    #[must_use]
    pub fn new(workspace: &'a WorkspacePath) -> Self {
        Self { workspace }
    }

    /// Creates and persists a new draft thread.
    ///
    /// # Errors
    ///
    /// Returns an error when required fields are empty or persistence fails.
    pub async fn create_thread(
        self,
        id: &str,
        title: &str,
        parent_mission_id: Option<&str>,
    ) -> Result<Thread> {
        if id.trim().is_empty() {
            return Err(WorkgraphError::ValidationError(
                "thread id must not be empty".to_owned(),
            ));
        }
        if title.trim().is_empty() {
            return Err(WorkgraphError::ValidationError(
                "thread title must not be empty".to_owned(),
            ));
        }

        let thread = Thread {
            id: id.to_owned(),
            title: title.to_owned(),
            status: ThreadStatus::Draft,
            assigned_actor: None,
            parent_mission_id: parent_mission_id.map(str::to_owned),
            exit_criteria: Vec::new(),
            evidence: Vec::new(),
            update_actions: Vec::new(),
            completion_actions: Vec::new(),
            messages: Vec::new(),
        };
        self.persist(
            &thread,
            AuditedWriteRequest::new(system_actor(), LedgerOp::Create)
                .with_note(format!("Created thread '{}'", thread.title)),
        )
        .await?;
        Ok(thread)
    }

    /// Transitions a thread from draft or blocked into a ready state.
    ///
    /// # Errors
    ///
    /// Returns an error when the transition is invalid or persistence fails.
    pub async fn open_thread(self, thread_id: &str) -> Result<Thread> {
        let mut thread = load_thread(self.workspace, thread_id).await?;
        match thread.status {
            ThreadStatus::Draft | ThreadStatus::Blocked => thread.status = ThreadStatus::Ready,
            ThreadStatus::Ready => {}
            _ => {
                return Err(WorkgraphError::ValidationError(format!(
                    "thread '{thread_id}' cannot be opened from status {:?}",
                    thread.status
                )));
            }
        }
        self.persist(
            &thread,
            AuditedWriteRequest::new(system_actor(), LedgerOp::Reopen)
                .with_note(format!("Opened thread '{thread_id}'")),
        )
        .await?;
        Ok(thread)
    }

    /// Claims a ready or waiting thread for an actor and marks it active.
    ///
    /// # Errors
    ///
    /// Returns an error when the thread is in an incompatible status, is already
    /// claimed by another actor, or persistence fails.
    pub async fn claim_thread(self, thread_id: &str, actor: ActorId) -> Result<Thread> {
        let mut thread = load_thread(self.workspace, thread_id).await?;

        match thread.status {
            ThreadStatus::Ready | ThreadStatus::Waiting => {
                if let Some(existing) = &thread.assigned_actor {
                    if existing != &actor {
                        return Err(WorkgraphError::ValidationError(format!(
                            "thread '{thread_id}' is already assigned to '{}'",
                            existing
                        )));
                    }
                }
                thread.assigned_actor = Some(actor.clone());
                thread.status = ThreadStatus::Active;
            }
            ThreadStatus::Active => {
                if thread.assigned_actor.as_ref() == Some(&actor) {
                    return Ok(thread);
                }
                return Err(WorkgraphError::ValidationError(format!(
                    "thread '{thread_id}' is already assigned to '{}'",
                    thread
                        .assigned_actor
                        .as_ref()
                        .map_or("unknown", ActorId::as_str)
                )));
            }
            _ => {
                return Err(WorkgraphError::ValidationError(format!(
                    "thread '{thread_id}' cannot be claimed from status {:?}",
                    thread.status
                )));
            }
        }

        self.persist(
            &thread,
            AuditedWriteRequest::new(actor.clone(), LedgerOp::Claim)
                .with_note(format!("Claimed thread '{thread_id}'")),
        )
        .await?;
        Ok(thread)
    }

    /// Appends a structured exit criterion to a thread.
    ///
    /// # Errors
    ///
    /// Returns an error when the criterion identifier is empty, duplicates an
    /// existing criterion, or persistence fails.
    pub async fn add_exit_criterion(
        self,
        thread_id: &str,
        criterion: ThreadExitCriterion,
    ) -> Result<Thread> {
        if criterion.id.trim().is_empty() {
            return Err(WorkgraphError::ValidationError(
                "thread exit criterion id must not be empty".to_owned(),
            ));
        }
        let mut thread = load_thread(self.workspace, thread_id).await?;
        if thread
            .exit_criteria
            .iter()
            .any(|existing| existing.id == criterion.id)
        {
            return Err(WorkgraphError::ValidationError(format!(
                "thread '{thread_id}' already contains exit criterion '{}'",
                criterion.id
            )));
        }
        let criterion_id = criterion.id.clone();
        thread.exit_criteria.push(criterion);
        self.persist(
            &thread,
            AuditedWriteRequest::new(system_actor(), LedgerOp::Update).with_note(format!(
                "Added exit criterion '{}' to thread '{thread_id}'",
                criterion_id
            )),
        )
        .await?;
        Ok(thread)
    }

    /// Records evidence against a thread.
    ///
    /// # Errors
    ///
    /// Returns an error when the evidence is invalid or persistence fails.
    pub async fn add_evidence(self, thread_id: &str, mut evidence: EvidenceItem) -> Result<Thread> {
        if evidence.id.trim().is_empty() {
            return Err(WorkgraphError::ValidationError(
                "thread evidence id must not be empty".to_owned(),
            ));
        }
        let mut thread = load_thread(self.workspace, thread_id).await?;
        if thread
            .evidence
            .iter()
            .any(|existing| existing.id == evidence.id)
        {
            return Err(WorkgraphError::ValidationError(format!(
                "thread '{thread_id}' already contains evidence '{}'",
                evidence.id
            )));
        }
        let known_criteria = thread
            .exit_criteria
            .iter()
            .map(|criterion| criterion.id.as_str())
            .collect::<BTreeSet<_>>();
        for criterion_id in &evidence.satisfies {
            if !known_criteria.contains(criterion_id.as_str()) {
                return Err(WorkgraphError::ValidationError(format!(
                    "thread '{thread_id}' evidence '{}' references unknown criterion '{}'",
                    evidence.id, criterion_id
                )));
            }
        }
        if evidence.recorded_at.is_none() {
            evidence.recorded_at = Some(Utc::now());
        }
        let evidence_id = evidence.id.clone();
        thread.evidence.push(evidence);
        self.persist(
            &thread,
            AuditedWriteRequest::new(system_actor(), LedgerOp::Update).with_note(format!(
                "Added evidence '{}' to thread '{thread_id}'",
                evidence_id
            )),
        )
        .await?;
        Ok(thread)
    }

    /// Appends a planned update action to a thread.
    ///
    /// # Errors
    ///
    /// Returns an error when the action identifier is invalid or persistence fails.
    pub async fn add_update_action(
        self,
        thread_id: &str,
        action: CoordinationAction,
    ) -> Result<Thread> {
        self.add_action(thread_id, action, ActionList::Update).await
    }

    /// Appends a planned completion action to a thread.
    ///
    /// # Errors
    ///
    /// Returns an error when the action identifier is invalid or persistence fails.
    pub async fn add_completion_action(
        self,
        thread_id: &str,
        action: CoordinationAction,
    ) -> Result<Thread> {
        self.add_action(thread_id, action, ActionList::Completion)
            .await
    }

    /// Marks a thread as done once every required exit criterion is satisfied.
    ///
    /// # Errors
    ///
    /// Returns an error when required criteria remain unsatisfied, the transition is
    /// invalid, or persistence fails.
    pub async fn complete_thread(self, thread_id: &str) -> Result<Thread> {
        let mut thread = load_thread(self.workspace, thread_id).await?;
        let missing = unsatisfied_exit_criteria(&thread);
        if !missing.is_empty() {
            return Err(WorkgraphError::ValidationError(format!(
                "thread '{thread_id}' cannot complete; unsatisfied exit criteria: {}",
                missing.join(", ")
            )));
        }

        match thread.status {
            ThreadStatus::Active
            | ThreadStatus::Waiting
            | ThreadStatus::Blocked
            | ThreadStatus::Done => {
                thread.status = ThreadStatus::Done;
            }
            _ => {
                return Err(WorkgraphError::ValidationError(format!(
                    "thread '{thread_id}' cannot be completed from status {:?}",
                    thread.status
                )));
            }
        }

        self.persist(
            &thread,
            AuditedWriteRequest::new(system_actor(), LedgerOp::Done)
                .with_note(format!("Completed thread '{thread_id}'")),
        )
        .await?;
        Ok(thread)
    }

    /// Appends a conversation message to a thread.
    ///
    /// # Errors
    ///
    /// Returns an error when the text is empty, the thread is terminal, or
    /// persistence fails.
    pub async fn add_message(self, thread_id: &str, actor: ActorId, text: &str) -> Result<Thread> {
        if text.trim().is_empty() {
            return Err(WorkgraphError::ValidationError(
                "thread messages must not be empty".to_owned(),
            ));
        }

        let mut thread = load_thread(self.workspace, thread_id).await?;
        if matches!(thread.status, ThreadStatus::Done | ThreadStatus::Cancelled) {
            return Err(WorkgraphError::ValidationError(format!(
                "thread '{thread_id}' is terminal and cannot receive new messages"
            )));
        }

        let message_actor = actor.clone();
        thread.messages.push(ConversationMessage {
            ts: Utc::now(),
            kind: infer_message_kind(&actor),
            actor,
            text: text.to_owned(),
        });

        self.persist(
            &thread,
            AuditedWriteRequest::new(message_actor, LedgerOp::Update)
                .with_note(format!("Added message to thread '{thread_id}'")),
        )
        .await?;
        Ok(thread)
    }

    async fn add_action(
        self,
        thread_id: &str,
        action: CoordinationAction,
        list: ActionList,
    ) -> Result<Thread> {
        if action.id.trim().is_empty() {
            return Err(WorkgraphError::ValidationError(
                "coordination action id must not be empty".to_owned(),
            ));
        }
        let mut thread = load_thread(self.workspace, thread_id).await?;
        let actions = match list {
            ActionList::Update => &mut thread.update_actions,
            ActionList::Completion => &mut thread.completion_actions,
        };
        if actions.iter().any(|existing| existing.id == action.id) {
            return Err(WorkgraphError::ValidationError(format!(
                "thread '{thread_id}' already contains action '{}'",
                action.id
            )));
        }
        let action_id = action.id.clone();
        actions.push(action);
        self.persist(
            &thread,
            AuditedWriteRequest::new(system_actor(), LedgerOp::Update).with_note(format!(
                "Added action '{}' to thread '{thread_id}'",
                action_id
            )),
        )
        .await?;
        Ok(thread)
    }

    async fn persist(self, thread: &Thread, audit: AuditedWriteRequest) -> Result<()> {
        self.authorize(thread.id.as_str(), &audit).await?;
        save_thread_with_audit(self.workspace, thread, audit.clone()).await?;
        self.after_mutation(thread, &audit).await
    }

    async fn authorize(self, thread_id: &str, audit: &AuditedWriteRequest) -> Result<()> {
        let action = policy_action_for(audit.op);
        let decision = evaluate_policy(
            self.workspace,
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

    async fn after_mutation(self, _thread: &Thread, _audit: &AuditedWriteRequest) -> Result<()> {
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

enum ActionList {
    Update,
    Completion,
}
