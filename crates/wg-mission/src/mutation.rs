use std::collections::BTreeSet;

use chrono::Utc;
use wg_error::{Result, WorkgraphError};
use wg_paths::WorkspacePath;
use wg_policy::{PolicyAction, PolicyContext, PolicyDecision, evaluate as evaluate_policy};
use wg_store::AuditedWriteRequest;
use wg_types::{LedgerOp, MissionMilestone, MissionStatus, ThreadStatus};

use crate::{
    MISSION_TYPE, Mission, MissionMilestoneInput, load_mission, save_mission_with_audit,
    system_actor,
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
            status: MissionStatus::Draft,
            objective: objective.to_owned(),
            milestones: Vec::new(),
            thread_ids: Vec::new(),
            run_ids: Vec::new(),
            approved_at: None,
            started_at: None,
            validated_at: None,
            completed_at: None,
        };
        self.persist(
            &mission,
            AuditedWriteRequest::new(system_actor(), LedgerOp::Create)
                .with_note(format!("Created mission '{}'", mission.id)),
        )
        .await?;
        Ok(mission)
    }

    /// Plans a mission and auto-creates milestone threads.
    ///
    /// # Errors
    ///
    /// Returns an error when the mission lifecycle transition is invalid,
    /// milestone definitions are invalid, thread creation fails, or persistence
    /// fails.
    pub async fn plan_mission(
        self,
        mission_id: &str,
        milestones: Vec<MissionMilestoneInput>,
    ) -> Result<Mission> {
        if milestones.is_empty() {
            return Err(WorkgraphError::ValidationError(
                "mission plan requires at least one milestone".to_owned(),
            ));
        }

        let mut mission = load_mission(self.workspace, mission_id).await?;
        match mission.status {
            MissionStatus::Draft | MissionStatus::Planned => {}
            _ => {
                return Err(WorkgraphError::ValidationError(format!(
                    "mission '{mission_id}' cannot be planned from status '{}'",
                    mission.status.as_str()
                )));
            }
        }

        let mut seen_milestone_ids = BTreeSet::new();
        let mut planned_milestones = Vec::with_capacity(milestones.len());
        for milestone in milestones {
            let milestone_id = normalize_identifier(milestone.id.trim());
            if milestone_id.is_empty() {
                return Err(WorkgraphError::ValidationError(
                    "mission milestone id must not be empty".to_owned(),
                ));
            }
            if !seen_milestone_ids.insert(milestone_id.clone()) {
                return Err(WorkgraphError::ValidationError(format!(
                    "mission '{mission_id}' contains duplicate milestone '{}'",
                    milestone_id
                )));
            }

            let milestone_title = milestone.title.trim();
            if milestone_title.is_empty() {
                return Err(WorkgraphError::ValidationError(format!(
                    "mission '{mission_id}' milestone '{milestone_id}' title must not be empty"
                )));
            }

            let thread_id = format!("{mission_id}-{milestone_id}");
            self.ensure_milestone_thread(mission_id, &thread_id, milestone_title)
                .await?;
            if !mission.thread_ids.iter().any(|id| id == &thread_id) {
                mission.thread_ids.push(thread_id.clone());
            }

            planned_milestones.push(MissionMilestone {
                id: milestone_id,
                title: milestone_title.to_owned(),
                description: sanitize_optional(milestone.description),
                thread_id,
            });
        }

        mission.milestones = planned_milestones;
        mission.status = MissionStatus::Planned;
        mission.approved_at = None;
        mission.started_at = None;
        mission.validated_at = None;
        mission.completed_at = None;

        self.persist(
            &mission,
            AuditedWriteRequest::new(system_actor(), LedgerOp::Update)
                .with_note(format!("Planned mission '{}'", mission.id)),
        )
        .await?;
        Ok(mission)
    }

    /// Marks a mission approved.
    ///
    /// # Errors
    ///
    /// Returns an error when the transition is invalid or persistence fails.
    pub async fn approve_mission(self, mission_id: &str) -> Result<Mission> {
        let mut mission = load_mission(self.workspace, mission_id).await?;
        match mission.status {
            MissionStatus::Planned | MissionStatus::Approved => {}
            _ => {
                return Err(WorkgraphError::ValidationError(format!(
                    "mission '{mission_id}' cannot be approved from status '{}'",
                    mission.status.as_str()
                )));
            }
        }
        if mission.milestones.is_empty() {
            return Err(WorkgraphError::ValidationError(format!(
                "mission '{mission_id}' cannot be approved without planned milestones"
            )));
        }
        mission.status = MissionStatus::Approved;
        if mission.approved_at.is_none() {
            mission.approved_at = Some(Utc::now());
        }
        self.persist(
            &mission,
            AuditedWriteRequest::new(system_actor(), LedgerOp::Update)
                .with_note(format!("Approved mission '{}'", mission.id)),
        )
        .await?;
        Ok(mission)
    }

    /// Starts approved mission execution.
    ///
    /// # Errors
    ///
    /// Returns an error when the transition is invalid or persistence fails.
    pub async fn start_mission(self, mission_id: &str) -> Result<Mission> {
        let mut mission = load_mission(self.workspace, mission_id).await?;
        match mission.status {
            MissionStatus::Approved | MissionStatus::Active => {}
            _ => {
                return Err(WorkgraphError::ValidationError(format!(
                    "mission '{mission_id}' cannot be started from status '{}'",
                    mission.status.as_str()
                )));
            }
        }
        mission.status = MissionStatus::Active;
        if mission.started_at.is_none() {
            mission.started_at = Some(Utc::now());
        }
        self.persist(
            &mission,
            AuditedWriteRequest::new(system_actor(), LedgerOp::Start)
                .with_note(format!("Started mission '{}'", mission.id)),
        )
        .await?;
        Ok(mission)
    }

    /// Marks a mission as validating completion readiness.
    ///
    /// # Errors
    ///
    /// Returns an error when the transition is invalid or persistence fails.
    pub async fn validate_mission(self, mission_id: &str) -> Result<Mission> {
        let mut mission = load_mission(self.workspace, mission_id).await?;
        match mission.status {
            MissionStatus::Active | MissionStatus::Validating => {}
            _ => {
                return Err(WorkgraphError::ValidationError(format!(
                    "mission '{mission_id}' cannot be validated from status '{}'",
                    mission.status.as_str()
                )));
            }
        }
        mission.status = MissionStatus::Validating;
        if mission.validated_at.is_none() {
            mission.validated_at = Some(Utc::now());
        }
        self.persist(
            &mission,
            AuditedWriteRequest::new(system_actor(), LedgerOp::Update)
                .with_note(format!("Validating mission '{}'", mission.id)),
        )
        .await?;
        Ok(mission)
    }

    /// Compatibility alias for mission start.
    ///
    /// # Errors
    ///
    /// Returns an error when mission start fails.
    pub async fn activate_mission(self, mission_id: &str) -> Result<Mission> {
        self.start_mission(mission_id).await
    }

    /// Marks a mission blocked.
    ///
    /// # Errors
    ///
    /// Returns an error when the transition is invalid or persistence fails.
    pub async fn block_mission(self, mission_id: &str) -> Result<Mission> {
        let mut mission = load_mission(self.workspace, mission_id).await?;
        match mission.status {
            MissionStatus::Draft
            | MissionStatus::Planned
            | MissionStatus::Approved
            | MissionStatus::Active
            | MissionStatus::Validating
            | MissionStatus::Blocked => {
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
            MissionStatus::Completed => {}
            MissionStatus::Validating => {
                let mut incomplete_threads = Vec::new();
                for thread_id in &mission.thread_ids {
                    match wg_thread::load_thread(self.workspace, thread_id).await {
                        Ok(thread) => {
                            if thread.status != ThreadStatus::Done {
                                incomplete_threads
                                    .push(format!("{thread_id} ({})", thread.status.as_str()));
                            }
                        }
                        Err(WorkgraphError::IoError(error))
                            if error.kind() == std::io::ErrorKind::NotFound =>
                        {
                            incomplete_threads.push(format!("{thread_id} (missing)"));
                        }
                        Err(error) => return Err(error),
                    }
                }
                if !incomplete_threads.is_empty() {
                    return Err(WorkgraphError::ValidationError(format!(
                        "mission '{mission_id}' cannot complete; incomplete threads: {}",
                        incomplete_threads.join(", ")
                    )));
                }
                mission.status = MissionStatus::Completed;
            }
            _ => {
                return Err(WorkgraphError::ValidationError(format!(
                    "mission '{mission_id}' cannot be completed from status '{}'",
                    mission.status.as_str()
                )));
            }
        }
        if mission.completed_at.is_none() {
            mission.completed_at = Some(Utc::now());
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

    async fn ensure_milestone_thread(
        self,
        mission_id: &str,
        thread_id: &str,
        title: &str,
    ) -> Result<()> {
        match wg_thread::load_thread(self.workspace, thread_id).await {
            Ok(existing) => {
                if existing.parent_mission_id.as_deref() != Some(mission_id) {
                    return Err(WorkgraphError::ValidationError(format!(
                        "milestone thread '{thread_id}' exists but belongs to a different mission"
                    )));
                }
            }
            Err(WorkgraphError::IoError(error)) if error.kind() == std::io::ErrorKind::NotFound => {
                wg_thread::create_thread(self.workspace, thread_id, title, Some(mission_id))
                    .await?;
                let _ = wg_thread::open_thread(self.workspace, thread_id).await?;
            }
            Err(other) => return Err(other),
        }
        Ok(())
    }
}

fn normalize_identifier(input: &str) -> String {
    let mut normalized = String::new();
    let mut previous_dash = false;
    for ch in input.chars() {
        let ch = ch.to_ascii_lowercase();
        if ch.is_ascii_alphanumeric() {
            normalized.push(ch);
            previous_dash = false;
        } else if !previous_dash {
            normalized.push('-');
            previous_dash = true;
        }
    }
    normalized.trim_matches('-').to_owned()
}

fn sanitize_optional(value: Option<String>) -> Option<String> {
    value.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    })
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
