#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Mission orchestration for WorkGraph.

use serde_yaml::Value;
use wg_error::{Result, WorkgraphError};
use wg_paths::WorkspacePath;
use wg_store::{
    AuditedWriteRequest, PrimitiveFrontmatter, StoredPrimitive, list_primitives, read_primitive,
    write_primitive_audited_now,
};
use wg_types::{ActorId, LedgerOp, MissionPrimitive, Registry, ThreadStatus};

pub use wg_types::MissionStatus;

const MISSION_TYPE: &str = "mission";
const SYSTEM_ACTOR: &str = "system:workgraph";

/// Typed mission model persisted by this crate.
pub type Mission = MissionPrimitive;

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

    let mission = MissionPrimitive {
        id: id.to_owned(),
        title: title.to_owned(),
        status: MissionStatus::Planned,
        objective: objective.to_owned(),
        thread_ids: Vec::new(),
        run_ids: Vec::new(),
    };
    save_mission_with_audit(
        workspace,
        &mission,
        AuditedWriteRequest::new(system_actor(), LedgerOp::Create)
            .with_note(format!("Created mission '{}'", mission.id)),
    )
    .await?;
    Ok(mission)
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
    let mut mission = load_mission(workspace, mission_id).await?;
    match mission.status {
        MissionStatus::Planned | MissionStatus::Blocked => mission.status = MissionStatus::Active,
        MissionStatus::Active => {}
        MissionStatus::Completed | MissionStatus::Cancelled => {
            return Err(WorkgraphError::ValidationError(format!(
                "mission '{mission_id}' cannot be activated from status '{}'",
                mission.status.as_str()
            )));
        }
    }
    save_mission_with_audit(
        workspace,
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
pub async fn block_mission(workspace: &WorkspacePath, mission_id: &str) -> Result<Mission> {
    let mut mission = load_mission(workspace, mission_id).await?;
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
    save_mission_with_audit(
        workspace,
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
pub async fn complete_mission(workspace: &WorkspacePath, mission_id: &str) -> Result<Mission> {
    let mut mission = load_mission(workspace, mission_id).await?;
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
    save_mission_with_audit(
        workspace,
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
pub async fn add_thread_to_mission(
    workspace: &WorkspacePath,
    mission_id: &str,
    thread_id: &str,
) -> Result<Mission> {
    if thread_id.trim().is_empty() {
        return Err(WorkgraphError::ValidationError(
            "thread id must not be empty".to_owned(),
        ));
    }
    let mut mission = load_mission(workspace, mission_id).await?;
    if !mission.thread_ids.iter().any(|id| id == thread_id) {
        mission.thread_ids.push(thread_id.to_owned());
    }
    save_mission_with_audit(
        workspace,
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
pub async fn add_run_to_mission(
    workspace: &WorkspacePath,
    mission_id: &str,
    run_id: &str,
) -> Result<Mission> {
    if run_id.trim().is_empty() {
        return Err(WorkgraphError::ValidationError(
            "run id must not be empty".to_owned(),
        ));
    }
    let mut mission = load_mission(workspace, mission_id).await?;
    if !mission.run_ids.iter().any(|id| id == run_id) {
        mission.run_ids.push(run_id.to_owned());
    }
    save_mission_with_audit(
        workspace,
        &mission,
        AuditedWriteRequest::new(system_actor(), LedgerOp::Update).with_note(format!(
            "Attached run '{}' to mission '{}'",
            run_id, mission.id
        )),
    )
    .await?;
    Ok(mission)
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

async fn save_mission_with_audit(
    workspace: &WorkspacePath,
    mission: &Mission,
    audit: AuditedWriteRequest,
) -> Result<()> {
    let primitive = mission_to_primitive(mission)?;
    write_primitive_audited_now(workspace, &Registry::builtins(), &primitive, audit).await?;
    Ok(())
}

fn system_actor() -> ActorId {
    ActorId::new(SYSTEM_ACTOR)
}

fn mission_to_primitive(mission: &Mission) -> Result<StoredPrimitive> {
    let mut extra_fields = std::collections::BTreeMap::new();
    extra_fields.insert(
        "status".to_owned(),
        serde_yaml::to_value(mission.status).map_err(encoding_error)?,
    );
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
            .map_or(Ok(MissionStatus::Planned), parse_yaml_value)?,
        objective: primitive.body.clone(),
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use tempfile::tempdir;
    use wg_ledger::LedgerReader;
    use wg_paths::WorkspacePath;
    use wg_store::{PrimitiveFrontmatter, StoredPrimitive, read_primitive, write_primitive};
    use wg_types::{LedgerOp, Registry, ThreadStatus};

    use crate::{
        MissionStatus, activate_mission, add_run_to_mission, add_thread_to_mission,
        complete_mission, create_mission, mission_progress,
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
    async fn mission_lifecycle_and_links_roundtrip() {
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
        let active = activate_mission(&workspace, "launch")
            .await
            .expect("mission should activate");
        assert_eq!(active.status, MissionStatus::Active);

        let linked = add_thread_to_mission(&workspace, "launch", "thread-1")
            .await
            .expect("thread should be linked");
        assert_eq!(linked.thread_ids, vec!["thread-1"]);

        let with_run = add_run_to_mission(&workspace, "launch", "run-1")
            .await
            .expect("run should be linked");
        assert_eq!(with_run.run_ids, vec!["run-1"]);

        let completed = complete_mission(&workspace, "launch")
            .await
            .expect("mission should complete");
        assert_eq!(completed.status, MissionStatus::Completed);

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
        assert_eq!(entries.len(), 5);
        assert_eq!(entries[0].op, LedgerOp::Create);
        assert_eq!(entries[1].op, LedgerOp::Start);
        assert_eq!(entries[2].op, LedgerOp::Update);
        assert_eq!(entries[3].op, LedgerOp::Update);
        assert_eq!(entries[4].op, LedgerOp::Done);
    }

    #[tokio::test]
    async fn mission_progress_counts_done_threads_from_store() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());

        create_mission(&workspace, "quality", "Quality mission", "Objective")
            .await
            .expect("mission should be created");
        add_thread_to_mission(&workspace, "quality", "t-1")
            .await
            .expect("thread should be linked");
        add_thread_to_mission(&workspace, "quality", "t-2")
            .await
            .expect("thread should be linked");
        add_thread_to_mission(&workspace, "quality", "missing-thread")
            .await
            .expect("thread should be linked");

        write_primitive(
            &workspace,
            &Registry::builtins(),
            &thread("t-1", ThreadStatus::Done),
        )
        .await
        .expect("thread t-1 should write");
        write_primitive(
            &workspace,
            &Registry::builtins(),
            &thread("t-2", ThreadStatus::Active),
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
