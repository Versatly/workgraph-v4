use wg_error::{Result, WorkgraphError};
use wg_paths::WorkspacePath;
use wg_policy::{PolicyAction, PolicyContext, PolicyDecision, evaluate as evaluate_policy};
use wg_store::AuditedWriteRequest;
use wg_types::LedgerOp;

use crate::{
    MISSION_TYPE, Mission, MissionStatus, load_mission, save_mission_with_audit, system_actor,
};

/// Domain mutation service for mission lifecycle and containment changes.
///
/// This service is the contract boundary for mission mutations. It owns
/// operation-specific validation, policy evaluation, audited persistence, and
/// the future hook point for trigger-aware follow-up behavior.
#[derive(Debug, Clone, Copy)]
pub struct MissionMutationService<'a> {
    workspace: &'a WorkspacePath,
}

impl<'a> MissionMutationService<'a> {
    /// Creates a new mission mutation service for a workspace.
    #[must_use]
    pub fn new(workspace: &'a WorkspacePath) -> Self {
        Self { workspace }
    }

    /// Creates and persists a new mission.
    ///
    /// # Errors
    ///
    /// Returns an error when required fields are invalid or persistence fails.
    pub async fn create_mission(self, id: &str, title: &str, objective: &str) -> Result<Mission> {
        if id.trim().is_empty() {
            return Err(WorkgraphError::ValidationError(
                "mission id must not be empty".to_owned(),
            ));
        }
        if title.trim().is_empty() {
            return Err(WorkgraphError::ValidationError(
                "mission title must not be empty".to_owned(),
            ));
        }
        if objective.trim().is_empty() {
            return Err(WorkgraphError::ValidationError(
                "mission objective must not be empty".to_owned(),
            ));
        }

        let mission = Mission {
            id: id.to_owned(),
            title: title.to_owned(),
            status: MissionStatus::Planned,
            objective: objective.to_owned(),
            thread_ids: Vec::new(),
            run_ids: Vec::new(),
        };
        self.persist(
            &mission,
            AuditedWriteRequest::new(system_actor(), LedgerOp::Create)
                .with_note(format!("Created mission '{}'", mission.id)),
        )
        .await?;
        Ok(mission)
    }

    /// Marks a mission active.
    ///
    /// # Errors
    ///
    /// Returns an error when the transition is invalid or persistence fails.
    pub async fn activate_mission(self, mission_id: &str) -> Result<Mission> {
        let mut mission = load_mission(self.workspace, mission_id).await?;
        match mission.status {
            MissionStatus::Planned | MissionStatus::Blocked => {
                mission.status = MissionStatus::Active
            }
            MissionStatus::Active => {}
            MissionStatus::Completed | MissionStatus::Cancelled => {
                return Err(WorkgraphError::ValidationError(format!(
                    "mission '{mission_id}' cannot be activated from status '{}'",
                    mission.status.as_str()
                )));
            }
        }
        self.persist(
            &mission,
            AuditedWriteRequest::new(system_actor(), LedgerOp::Start)
                .with_note(format!("Activated mission '{}'", mission.id)),
        )
        .await?;
        Ok(mission)
    }

    /// Marks a mission blocked.
    ///
    /// # Errors
    ///
    /// Returns an error when the transition is invalid or persistence fails.
    pub async fn block_mission(self, mission_id: &str) -> Result<Mission> {
        let mut mission = load_mission(self.workspace, mission_id).await?;
        match mission.status {
            MissionStatus::Planned | MissionStatus::Active | MissionStatus::Blocked => {
                mission.status = MissionStatus::Blocked;
            }
            MissionStatus::Completed | MissionStatus::Cancelled => {
                return Err(WorkgraphError::ValidationError(format!(
                    "mission '{mission_id}' cannot be blocked from status '{}'",
                    mission.status.as_str()
                )));
            }
        }
        self.persist(
            &mission,
            AuditedWriteRequest::new(system_actor(), LedgerOp::Update)
                .with_note(format!("Blocked mission '{}'", mission.id)),
        )
        .await?;
        Ok(mission)
    }

    /// Marks a mission completed.
    ///
    /// # Errors
    ///
    /// Returns an error when the transition is invalid or persistence fails.
    pub async fn complete_mission(self, mission_id: &str) -> Result<Mission> {
        let mut mission = load_mission(self.workspace, mission_id).await?;
        match mission.status {
            MissionStatus::Planned | MissionStatus::Active | MissionStatus::Blocked => {
                mission.status = MissionStatus::Completed;
            }
            MissionStatus::Completed => {}
            MissionStatus::Cancelled => {
                return Err(WorkgraphError::ValidationError(format!(
                    "mission '{mission_id}' cannot be completed from status '{}'",
                    mission.status.as_str()
                )));
            }
        }
        self.persist(
            &mission,
            AuditedWriteRequest::new(system_actor(), LedgerOp::Done)
                .with_note(format!("Completed mission '{}'", mission.id)),
        )
        .await?;
        Ok(mission)
    }

    /// Adds a child thread to a mission.
    ///
    /// # Errors
    ///
    /// Returns an error when the thread identifier is invalid or persistence fails.
    pub async fn add_thread_to_mission(self, mission_id: &str, thread_id: &str) -> Result<Mission> {
        if thread_id.trim().is_empty() {
            return Err(WorkgraphError::ValidationError(
                "thread id must not be empty".to_owned(),
            ));
        }
        let mut mission = load_mission(self.workspace, mission_id).await?;
        if !mission.thread_ids.iter().any(|id| id == thread_id) {
            mission.thread_ids.push(thread_id.to_owned());
        }
        self.persist(
            &mission,
            AuditedWriteRequest::new(system_actor(), LedgerOp::Update).with_note(format!(
                "Attached thread '{}' to mission '{}'",
                thread_id, mission.id
            )),
        )
        .await?;
        Ok(mission)
    }

    /// Adds a run to a mission.
    ///
    /// # Errors
    ///
    /// Returns an error when the run identifier is invalid or persistence fails.
    pub async fn add_run_to_mission(self, mission_id: &str, run_id: &str) -> Result<Mission> {
        if run_id.trim().is_empty() {
            return Err(WorkgraphError::ValidationError(
                "run id must not be empty".to_owned(),
            ));
        }
        let mut mission = load_mission(self.workspace, mission_id).await?;
        if !mission.run_ids.iter().any(|id| id == run_id) {
            mission.run_ids.push(run_id.to_owned());
        }
        self.persist(
            &mission,
            AuditedWriteRequest::new(system_actor(), LedgerOp::Update).with_note(format!(
                "Attached run '{}' to mission '{}'",
                run_id, mission.id
            )),
        )
        .await?;
        Ok(mission)
    }

    async fn persist(self, mission: &Mission, audit: AuditedWriteRequest) -> Result<()> {
        self.authorize(mission.id.as_str(), &audit).await?;
        save_mission_with_audit(self.workspace, mission, audit.clone()).await?;
        self.after_mutation(mission, &audit).await
    }

    async fn authorize(self, mission_id: &str, audit: &AuditedWriteRequest) -> Result<()> {
        let action = policy_action_for(audit.op);
        let decision = evaluate_policy(
            self.workspace,
            &audit.actor,
            action,
            MISSION_TYPE,
            &PolicyContext::default(),
        )
        .await?;
        if decision == PolicyDecision::Deny {
            return Err(WorkgraphError::ValidationError(format!(
                "policy denied {} of {MISSION_TYPE}/{mission_id} for actor '{}'",
                policy_action_label(action),
                audit.actor
            )));
        }
        Ok(())
    }

    async fn after_mutation(self, _mission: &Mission, _audit: &AuditedWriteRequest) -> Result<()> {
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
