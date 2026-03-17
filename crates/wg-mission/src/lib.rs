#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Mission orchestration for WorkGraph.
//!
//! Missions are persisted as `mission` primitives with lifecycle metadata in
//! frontmatter and objective markdown in the body.

use std::collections::BTreeMap;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use wg_error::{Result, WorkgraphError};
use wg_paths::WorkspacePath;
use wg_store::{PrimitiveFrontmatter, StoredPrimitive, read_primitive, write_primitive};
use wg_types::Registry;

const MISSION_TYPE: &str = "mission";

/// Tracks mission lifecycle state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissionStatus {
    /// Mission exists but is not started.
    #[default]
    Planned,
    /// Mission is actively being executed.
    Active,
    /// Mission is complete.
    Completed,
}

impl MissionStatus {
    /// Returns the storage string for this mission status.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Planned => "planned",
            Self::Active => "active",
            Self::Completed => "completed",
        }
    }
}

impl FromStr for MissionStatus {
    type Err = String;

    fn from_str(input: &str) -> std::result::Result<Self, Self::Err> {
        match input {
            "planned" => Ok(Self::Planned),
            "active" => Ok(Self::Active),
            "completed" => Ok(Self::Completed),
            _ => Err(format!("unsupported mission status '{input}'")),
        }
    }
}

/// Minimal compatibility plan type used by placeholder dispatch crate.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MissionPlan {
    /// Human-readable name for the mission.
    pub name: String,
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

/// Domain model for a persisted mission primitive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Mission {
    /// Stable mission identifier.
    pub id: String,
    /// Mission title.
    pub title: String,
    /// Mission lifecycle status.
    pub status: MissionStatus,
    /// Mission objective written in markdown.
    pub objective: String,
    /// Child thread identifiers.
    pub child_threads: Vec<String>,
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

    let mission = Mission {
        id: id.to_owned(),
        title: title.to_owned(),
        status: MissionStatus::Planned,
        objective: objective.to_owned(),
        child_threads: Vec::new(),
    };
    save_mission(workspace, &mission).await?;
    Ok(mission)
}

/// Marks a mission active.
///
/// # Errors
///
/// Returns an error when the transition is invalid or persistence fails.
pub async fn activate_mission(workspace: &WorkspacePath, mission_id: &str) -> Result<Mission> {
    let mut mission = load_mission(workspace, mission_id).await?;
    match mission.status {
        MissionStatus::Planned => mission.status = MissionStatus::Active,
        MissionStatus::Active => {}
        MissionStatus::Completed => {
            return Err(WorkgraphError::ValidationError(format!(
                "mission '{mission_id}' is already completed"
            )));
        }
    }

    save_mission(workspace, &mission).await?;
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
        MissionStatus::Planned | MissionStatus::Active => mission.status = MissionStatus::Completed,
        MissionStatus::Completed => {}
    }

    save_mission(workspace, &mission).await?;
    Ok(mission)
}

/// Adds a child thread to a mission.
///
/// # Errors
///
/// Returns an error when persistence fails.
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
    if !mission.child_threads.iter().any(|id| id == thread_id) {
        mission.child_threads.push(thread_id.to_owned());
    }
    save_mission(workspace, &mission).await?;
    Ok(mission)
}

/// Computes mission progress from the stored thread primitives.
///
/// Missing thread primitives are counted in the total but not the completed
/// count.
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

    for thread_id in &mission.child_threads {
        match read_primitive(workspace, "thread", thread_id).await {
            Ok(thread) => {
                if thread
                    .frontmatter
                    .extra_fields
                    .get("status")
                    .and_then(string_value)
                    == Some("completed")
                {
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
        total_threads: mission.child_threads.len(),
    })
}

async fn load_mission(workspace: &WorkspacePath, mission_id: &str) -> Result<Mission> {
    let primitive = read_primitive(workspace, MISSION_TYPE, mission_id).await?;
    mission_from_primitive(&primitive)
}

async fn save_mission(workspace: &WorkspacePath, mission: &Mission) -> Result<()> {
    let primitive = mission_to_primitive(mission);
    write_primitive(workspace, &Registry::builtins(), &primitive).await?;
    Ok(())
}

fn mission_to_primitive(mission: &Mission) -> StoredPrimitive {
    let mut extra_fields = BTreeMap::new();
    extra_fields.insert(
        "status".to_owned(),
        Value::String(mission.status.as_str().to_owned()),
    );
    if !mission.child_threads.is_empty() {
        extra_fields.insert(
            "thread_ids".to_owned(),
            Value::Sequence(
                mission
                    .child_threads
                    .iter()
                    .map(|thread| Value::String(thread.clone()))
                    .collect(),
            ),
        );
    }

    StoredPrimitive {
        frontmatter: PrimitiveFrontmatter {
            r#type: MISSION_TYPE.to_owned(),
            id: mission.id.clone(),
            title: mission.title.clone(),
            extra_fields,
        },
        body: mission.objective.clone(),
    }
}

fn mission_from_primitive(primitive: &StoredPrimitive) -> Result<Mission> {
    if primitive.frontmatter.r#type != MISSION_TYPE {
        return Err(WorkgraphError::ValidationError(format!(
            "expected mission primitive, found '{}'",
            primitive.frontmatter.r#type
        )));
    }

    let status = primitive
        .frontmatter
        .extra_fields
        .get("status")
        .and_then(string_value)
        .map_or(Ok(MissionStatus::Planned), |value| {
            MissionStatus::from_str(value).map_err(WorkgraphError::ValidationError)
        })?;
    let child_threads = primitive
        .frontmatter
        .extra_fields
        .get("thread_ids")
        .map_or_else(Vec::new, parse_string_list);

    Ok(Mission {
        id: primitive.frontmatter.id.clone(),
        title: primitive.frontmatter.title.clone(),
        status,
        objective: primitive.body.clone(),
        child_threads,
    })
}

fn parse_string_list(value: &Value) -> Vec<String> {
    match value {
        Value::String(value) => vec![value.clone()],
        Value::Sequence(values) => values
            .iter()
            .filter_map(string_value)
            .map(str::to_owned)
            .collect(),
        Value::Tagged(tagged) => parse_string_list(&tagged.value),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::Mapping(_) => Vec::new(),
    }
}

fn string_value(value: &Value) -> Option<&str> {
    match value {
        Value::String(value) => Some(value.as_str()),
        Value::Tagged(tagged) => string_value(&tagged.value),
        Value::Null
        | Value::Bool(_)
        | Value::Number(_)
        | Value::Sequence(_)
        | Value::Mapping(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_yaml::Value;
    use tempfile::tempdir;
    use wg_paths::WorkspacePath;
    use wg_store::{PrimitiveFrontmatter, StoredPrimitive, read_primitive, write_primitive};
    use wg_types::Registry;

    use crate::{
        MissionStatus, activate_mission, add_thread_to_mission, complete_mission, create_mission,
        mission_progress,
    };

    fn thread(id: &str, status: &str) -> StoredPrimitive {
        StoredPrimitive {
            frontmatter: PrimitiveFrontmatter {
                r#type: "thread".to_owned(),
                id: id.to_owned(),
                title: format!("Thread {id}"),
                extra_fields: BTreeMap::from([(
                    "status".to_owned(),
                    Value::String(status.to_owned()),
                )]),
            },
            body: "## Conversation\n\n```yaml\n[]\n```\n".to_owned(),
        }
    }

    #[tokio::test]
    async fn mission_lifecycle_and_thread_linking_roundtrip() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());

        let created = create_mission(
            &workspace,
            "launch",
            "Launch coordination",
            "## Objective\nShip v1 safely.\n",
        )
        .await
        .expect("mission should be created");
        assert_eq!(created.status, MissionStatus::Planned);

        let activated = activate_mission(&workspace, "launch")
            .await
            .expect("mission should activate");
        assert_eq!(activated.status, MissionStatus::Active);

        let linked = add_thread_to_mission(&workspace, "launch", "thread-1")
            .await
            .expect("thread should be linked");
        assert_eq!(linked.child_threads, vec!["thread-1"]);

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
                .expect("status should exist"),
            &Value::String("completed".to_owned())
        );
        assert!(stored.body.contains("Ship v1 safely"));
    }

    #[tokio::test]
    async fn mission_progress_counts_completed_threads_from_store() {
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
            &thread("t-1", "completed"),
        )
        .await
        .expect("thread t-1 should write");
        write_primitive(&workspace, &Registry::builtins(), &thread("t-2", "open"))
            .await
            .expect("thread t-2 should write");

        let progress = mission_progress(&workspace, "quality")
            .await
            .expect("mission progress should compute");
        assert_eq!(progress.completed_threads, 1);
        assert_eq!(progress.total_threads, 3);
    }
}
