#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Mission orchestration for WorkGraph.

use chrono::{DateTime, Utc};
use serde_yaml::Value;
use wg_error::{Result, WorkgraphError};
use wg_paths::WorkspacePath;
use wg_store::{
    AuditedWriteRequest, PrimitiveFrontmatter, StoredPrimitive, list_primitives, read_primitive,
    write_primitive_audited_now,
};
use wg_types::{ActorId, MissionPrimitive, Registry, ThreadStatus};

mod mutation;

pub use mutation::MissionMutationService;
pub use wg_types::MissionStatus;

const MISSION_TYPE: &str = "mission";
const SYSTEM_ACTOR: &str = "system:workgraph";

/// Typed mission model persisted by this crate.
pub type Mission = MissionPrimitive;

/// Planned milestone input used to define mission execution phases.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionMilestoneInput {
    /// Stable milestone identifier scoped to the mission.
    pub id: String,
    /// Human-readable milestone title.
    pub title: String,
    /// Optional longer milestone detail.
    pub description: Option<String>,
}

/// Minimal compatibility plan type retained for placeholder flows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MissionPlan {
    /// Stable mission identifier.
    pub id: String,
    /// Current lifecycle status.
    pub status: MissionStatus,
}

impl MissionPlan {
    /// Returns a copy of the plan marked active.
    #[must_use]
    pub fn start(mut self) -> Self {
        self.status = MissionStatus::Active;
        self
    }
}

/// Progress summary for a mission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MissionProgress {
    /// Number of completed child threads.
    pub completed_threads: usize,
    /// Total tracked child threads.
    pub total_threads: usize,
}

/// Creates and persists a new mission.
///
/// # Errors
///
/// Returns an error when required fields are invalid or persistence fails.
pub async fn create_mission(
    workspace: &WorkspacePath,
    id: &str,
    title: &str,
    objective: &str,
) -> Result<Mission> {
    MissionMutationService::new(workspace)
        .create_mission(id, title, objective)
        .await
}

/// Plans a mission and auto-creates milestone threads.
///
/// # Errors
///
/// Returns an error when the mission lifecycle transition is invalid, milestone
/// definitions are invalid, thread creation fails, or persistence fails.
pub async fn plan_mission(
    workspace: &WorkspacePath,
    mission_id: &str,
    milestones: Vec<MissionMilestoneInput>,
) -> Result<Mission> {
    MissionMutationService::new(workspace)
        .plan_mission(mission_id, milestones)
        .await
}

/// Marks a mission approved.
///
/// # Errors
///
/// Returns an error when the transition is invalid or persistence fails.
pub async fn approve_mission(workspace: &WorkspacePath, mission_id: &str) -> Result<Mission> {
    MissionMutationService::new(workspace)
        .approve_mission(mission_id)
        .await
}

/// Starts approved mission execution.
///
/// # Errors
///
/// Returns an error when the transition is invalid or persistence fails.
pub async fn start_mission(workspace: &WorkspacePath, mission_id: &str) -> Result<Mission> {
    MissionMutationService::new(workspace)
        .start_mission(mission_id)
        .await
}

/// Marks a mission as validating completion readiness.
///
/// # Errors
///
/// Returns an error when the transition is invalid or persistence fails.
pub async fn validate_mission(workspace: &WorkspacePath, mission_id: &str) -> Result<Mission> {
    MissionMutationService::new(workspace)
        .validate_mission(mission_id)
        .await
}

/// Loads a persisted mission by identifier.
///
/// # Errors
///
/// Returns an error when the mission cannot be loaded or decoded.
pub async fn load_mission(workspace: &WorkspacePath, mission_id: &str) -> Result<Mission> {
    let primitive = read_primitive(workspace, MISSION_TYPE, mission_id).await?;
    mission_from_primitive(&primitive)
}

/// Lists all persisted missions.
///
/// # Errors
///
/// Returns an error when mission primitives cannot be loaded or decoded.
pub async fn list_missions(workspace: &WorkspacePath) -> Result<Vec<Mission>> {
    list_primitives(workspace, MISSION_TYPE)
        .await?
        .iter()
        .map(mission_from_primitive)
        .collect()
}

/// Marks a mission active.
///
/// # Errors
///
/// Returns an error when the transition is invalid or persistence fails.
pub async fn activate_mission(workspace: &WorkspacePath, mission_id: &str) -> Result<Mission> {
    start_mission(workspace, mission_id).await
}

/// Marks a mission blocked.
///
/// # Errors
///
/// Returns an error when the transition is invalid or persistence fails.
pub async fn block_mission(workspace: &WorkspacePath, mission_id: &str) -> Result<Mission> {
    MissionMutationService::new(workspace)
        .block_mission(mission_id)
        .await
}

/// Marks a mission completed.
///
/// # Errors
///
/// Returns an error when the transition is invalid or persistence fails.
pub async fn complete_mission(workspace: &WorkspacePath, mission_id: &str) -> Result<Mission> {
    MissionMutationService::new(workspace)
        .complete_mission(mission_id)
        .await
}

/// Adds a run to a mission.
///
/// # Errors
///
/// Returns an error when the run identifier is invalid or persistence fails.
pub async fn add_run_to_mission(
    workspace: &WorkspacePath,
    mission_id: &str,
    run_id: &str,
) -> Result<Mission> {
    MissionMutationService::new(workspace)
        .add_run_to_mission(mission_id, run_id)
        .await
}

/// Computes mission progress from the stored thread primitives.
///
/// Missing thread primitives are counted in the total but not the completed count.
///
/// # Errors
///
/// Returns an error when mission loading fails or thread loading fails with a
/// non-not-found error.
pub async fn mission_progress(
    workspace: &WorkspacePath,
    mission_id: &str,
) -> Result<MissionProgress> {
    let mission = load_mission(workspace, mission_id).await?;
    let mut completed = 0;

    for thread_id in &mission.thread_ids {
        match read_primitive(workspace, "thread", thread_id).await {
            Ok(thread) => {
                let status = thread
                    .frontmatter
                    .extra_fields
                    .get("status")
                    .map_or(Ok(ThreadStatus::Draft), parse_yaml_value)?;
                if status == ThreadStatus::Done {
                    completed += 1;
                }
            }
            Err(WorkgraphError::IoError(error)) if error.kind() == std::io::ErrorKind::NotFound => {
            }
            Err(other) => return Err(other),
        }
    }

    Ok(MissionProgress {
        completed_threads: completed,
        total_threads: mission.thread_ids.len(),
    })
}

pub(crate) async fn save_mission_with_audit(
    workspace: &WorkspacePath,
    mission: &Mission,
    audit: AuditedWriteRequest,
) -> Result<wg_types::LedgerEntry> {
    let primitive = mission_to_primitive(mission)?;
    let (_, ledger_entry) =
        write_primitive_audited_now(workspace, &Registry::builtins(), &primitive, audit).await?;
    Ok(ledger_entry)
}

pub(crate) fn system_actor() -> ActorId {
    ActorId::new(SYSTEM_ACTOR)
}

fn mission_to_primitive(mission: &Mission) -> Result<StoredPrimitive> {
    let mut extra_fields = std::collections::BTreeMap::new();
    extra_fields.insert(
        "status".to_owned(),
        serde_yaml::to_value(mission.status).map_err(encoding_error)?,
    );
    if !mission.milestones.is_empty() {
        extra_fields.insert(
            "milestones".to_owned(),
            serde_yaml::to_value(&mission.milestones).map_err(encoding_error)?,
        );
    }
    if !mission.thread_ids.is_empty() {
        extra_fields.insert(
            "thread_ids".to_owned(),
            serde_yaml::to_value(&mission.thread_ids).map_err(encoding_error)?,
        );
    }
    if !mission.run_ids.is_empty() {
        extra_fields.insert(
            "run_ids".to_owned(),
            serde_yaml::to_value(&mission.run_ids).map_err(encoding_error)?,
        );
    }
    set_datetime_field(&mut extra_fields, "approved_at", mission.approved_at);
    set_datetime_field(&mut extra_fields, "started_at", mission.started_at);
    set_datetime_field(&mut extra_fields, "validated_at", mission.validated_at);
    set_datetime_field(&mut extra_fields, "completed_at", mission.completed_at);

    Ok(StoredPrimitive {
        frontmatter: PrimitiveFrontmatter {
            r#type: MISSION_TYPE.to_owned(),
            id: mission.id.clone(),
            title: mission.title.clone(),
            extra_fields,
        },
        body: mission.objective.clone(),
    })
}

fn mission_from_primitive(primitive: &StoredPrimitive) -> Result<Mission> {
    if primitive.frontmatter.r#type != MISSION_TYPE {
        return Err(WorkgraphError::ValidationError(format!(
            "expected mission primitive, found '{}'",
            primitive.frontmatter.r#type
        )));
    }

    if primitive.body.trim().is_empty() {
        return Err(WorkgraphError::ValidationError(format!(
            "mission '{}' must include a non-empty objective body",
            primitive.frontmatter.id
        )));
    }

    Ok(MissionPrimitive {
        id: primitive.frontmatter.id.clone(),
        title: primitive.frontmatter.title.clone(),
        status: primitive
            .frontmatter
            .extra_fields
            .get("status")
            .map_or(Ok(MissionStatus::Draft), parse_yaml_value)?,
        objective: primitive.body.clone(),
        milestones: primitive
            .frontmatter
            .extra_fields
            .get("milestones")
            .map_or(Ok(Vec::new()), parse_yaml_value)?,
        thread_ids: primitive
            .frontmatter
            .extra_fields
            .get("thread_ids")
            .map_or(Ok(Vec::new()), parse_yaml_value)?,
        run_ids: primitive
            .frontmatter
            .extra_fields
            .get("run_ids")
            .map_or(Ok(Vec::new()), parse_yaml_value)?,
        approved_at: primitive
            .frontmatter
            .extra_fields
            .get("approved_at")
            .map_or(Ok(None), parse_optional_datetime)?,
        started_at: primitive
            .frontmatter
            .extra_fields
            .get("started_at")
            .map_or(Ok(None), parse_optional_datetime)?,
        validated_at: primitive
            .frontmatter
            .extra_fields
            .get("validated_at")
            .map_or(Ok(None), parse_optional_datetime)?,
        completed_at: primitive
            .frontmatter
            .extra_fields
            .get("completed_at")
            .map_or(Ok(None), parse_optional_datetime)?,
    })
}

fn parse_yaml_value<T>(value: &Value) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_yaml::from_value::<T>(value.clone()).map_err(encoding_error)
}

fn encoding_error(error: impl std::fmt::Display) -> WorkgraphError {
    WorkgraphError::EncodingError(error.to_string())
}

fn set_datetime_field(
    extra_fields: &mut std::collections::BTreeMap<String, Value>,
    key: &str,
    value: Option<DateTime<Utc>>,
) {
    if let Some(value) = value {
        extra_fields.insert(key.to_owned(), Value::String(value.to_rfc3339()));
    }
}

fn parse_optional_datetime(value: &Value) -> Result<Option<DateTime<Utc>>> {
    match value {
        Value::String(value) => DateTime::parse_from_rfc3339(value)
            .map(|timestamp| Some(timestamp.with_timezone(&Utc)))
            .map_err(encoding_error),
        Value::Tagged(tagged) => parse_optional_datetime(&tagged.value),
        Value::Null => Ok(None),
        Value::Bool(_) | Value::Number(_) | Value::Sequence(_) | Value::Mapping(_) => Err(
            WorkgraphError::ValidationError("expected RFC3339 datetime string".to_owned()),
        ),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use tempfile::tempdir;
    use wg_ledger::LedgerReader;
    use wg_paths::WorkspacePath;
    use wg_store::{PrimitiveFrontmatter, StoredPrimitive, read_primitive, write_primitive};
    use wg_thread::{claim_thread, complete_thread, load_thread};
    use wg_types::{ActorId, LedgerOp, Registry, ThreadStatus};

    use crate::{
        MissionMilestoneInput, MissionStatus, approve_mission, complete_mission, create_mission,
        mission_progress, plan_mission, start_mission, validate_mission,
    };

    fn thread(id: &str, status: ThreadStatus) -> StoredPrimitive {
        StoredPrimitive {
            frontmatter: PrimitiveFrontmatter {
                r#type: "thread".to_owned(),
                id: id.to_owned(),
                title: format!("Thread {id}"),
                extra_fields: BTreeMap::from([(
                    "status".to_owned(),
                    serde_yaml::to_value(status).expect("status should serialize"),
                )]),
            },
            body: "## Conversation\n\n```yaml\n[]\n```\n".to_owned(),
        }
    }

    #[tokio::test]
    async fn mission_lifecycle_with_planning_and_milestones_roundtrip() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());

        create_mission(
            &workspace,
            "launch",
            "Launch mission",
            "Ship the launch safely.",
        )
        .await
        .expect("mission should be created");
        let planned = plan_mission(
            &workspace,
            "launch",
            vec![
                MissionMilestoneInput {
                    id: "scoping".into(),
                    title: "Scoping".into(),
                    description: None,
                },
                MissionMilestoneInput {
                    id: "verification".into(),
                    title: "Verification".into(),
                    description: Some("Gather signoff evidence".into()),
                },
            ],
        )
        .await
        .expect("mission should plan");
        assert_eq!(planned.status, MissionStatus::Planned);
        assert_eq!(planned.milestones.len(), 2);
        assert_eq!(
            planned.thread_ids,
            vec![
                "launch-scoping".to_owned(),
                "launch-verification".to_owned()
            ]
        );

        let scoped_thread = load_thread(&workspace, "launch-scoping")
            .await
            .expect("milestone thread should exist");
        assert_eq!(scoped_thread.status, ThreadStatus::Ready);
        assert_eq!(scoped_thread.parent_mission_id.as_deref(), Some("launch"));

        let approved = approve_mission(&workspace, "launch")
            .await
            .expect("mission should be approved");
        assert_eq!(approved.status, MissionStatus::Approved);
        assert!(approved.approved_at.is_some());

        let active = start_mission(&workspace, "launch")
            .await
            .expect("mission should start");
        assert_eq!(active.status, MissionStatus::Active);
        assert!(active.started_at.is_some());

        claim_thread(&workspace, "launch-scoping", ActorId::new("agent:cursor"))
            .await
            .expect("thread should claim");
        complete_thread(&workspace, "launch-scoping")
            .await
            .expect("thread should complete");
        claim_thread(
            &workspace,
            "launch-verification",
            ActorId::new("agent:cursor"),
        )
        .await
        .expect("thread should claim");
        complete_thread(&workspace, "launch-verification")
            .await
            .expect("thread should complete");

        let validating = validate_mission(&workspace, "launch")
            .await
            .expect("mission should validate");
        assert_eq!(validating.status, MissionStatus::Validating);
        assert!(validating.validated_at.is_some());

        let completed = complete_mission(&workspace, "launch")
            .await
            .expect("mission should complete");
        assert_eq!(completed.status, MissionStatus::Completed);
        assert!(completed.completed_at.is_some());

        let stored = read_primitive(&workspace, "mission", "launch")
            .await
            .expect("mission primitive should be readable");
        assert_eq!(
            stored
                .frontmatter
                .extra_fields
                .get("status")
                .expect("status field should be present"),
            &serde_yaml::to_value(MissionStatus::Completed).expect("status should serialize")
        );

        let (entries, _) = LedgerReader::new(temp_dir.path().to_path_buf())
            .read_from(Default::default())
            .await
            .expect("ledger should be readable");
        let mission_ops = entries
            .iter()
            .filter(|entry| entry.primitive_type == "mission" && entry.primitive_id == "launch")
            .map(|entry| entry.op)
            .collect::<Vec<_>>();
        assert_eq!(
            mission_ops,
            vec![
                LedgerOp::Create,
                LedgerOp::Update,
                LedgerOp::Update,
                LedgerOp::Start,
                LedgerOp::Update,
                LedgerOp::Done,
            ]
        );
    }

    #[tokio::test]
    async fn mission_progress_counts_done_threads_from_store() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());

        create_mission(&workspace, "quality", "Quality mission", "Objective")
            .await
            .expect("mission should be created");
        plan_mission(
            &workspace,
            "quality",
            vec![
                MissionMilestoneInput {
                    id: "t-1".into(),
                    title: "Thread one".into(),
                    description: None,
                },
                MissionMilestoneInput {
                    id: "t-2".into(),
                    title: "Thread two".into(),
                    description: None,
                },
                MissionMilestoneInput {
                    id: "missing-thread".into(),
                    title: "Missing thread".into(),
                    description: None,
                },
            ],
        )
        .await
        .expect("mission should plan");

        write_primitive(
            &workspace,
            &Registry::builtins(),
            &thread("quality-t-1", ThreadStatus::Done),
        )
        .await
        .expect("thread t-1 should write");
        write_primitive(
            &workspace,
            &Registry::builtins(),
            &thread("quality-t-2", ThreadStatus::Active),
        )
        .await
        .expect("thread t-2 should write");

        let progress = mission_progress(&workspace, "quality")
            .await
            .expect("mission progress should compute");
        assert_eq!(progress.completed_threads, 1);
        assert_eq!(progress.total_threads, 3);
    }

    #[tokio::test]
    async fn mission_requires_non_empty_objective() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());

        let error = create_mission(&workspace, "bad", "Bad mission", "   ")
            .await
            .expect_err("empty objective should fail");
        assert!(error.to_string().contains("objective"));
    }
}
