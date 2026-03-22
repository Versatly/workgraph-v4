#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Durable run persistence and lifecycle helpers for WorkGraph dispatch.

use serde_yaml::Value;
use wg_error::{Result, WorkgraphError};
use wg_paths::WorkspacePath;
use wg_store::{
    PrimitiveFrontmatter, StoredPrimitive, list_primitives, read_primitive, write_primitive,
};
use wg_types::{ActorId, Registry, RunPrimitive, RunStatus};

const RUN_TYPE: &str = "run";

/// Typed run model persisted by this crate.
pub type Run = RunPrimitive;

/// Durable request used to create a new run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchRequest {
    /// Run title.
    pub title: String,
    /// Logical actor responsible for the run.
    pub actor_id: ActorId,
    /// Concrete executor performing the run, when different.
    pub executor_id: Option<ActorId>,
    /// Owning thread identifier.
    pub thread_id: String,
    /// Related mission identifier, when any.
    pub mission_id: Option<String>,
    /// Parent run identifier for delegated execution, when any.
    pub parent_run_id: Option<String>,
    /// Optional summary stored alongside the run.
    pub summary: Option<String>,
}

/// Builds a minimal dispatch request.
#[must_use]
pub fn prepare_dispatch(title: &str, actor_id: ActorId, thread_id: &str) -> DispatchRequest {
    DispatchRequest {
        title: title.to_owned(),
        actor_id,
        executor_id: None,
        thread_id: thread_id.to_owned(),
        mission_id: None,
        parent_run_id: None,
        summary: None,
    }
}

/// Creates and persists a queued run.
///
/// # Errors
///
/// Returns an error when required identifiers are invalid or persistence fails.
pub async fn create_run(
    workspace: &WorkspacePath,
    id: &str,
    request: DispatchRequest,
) -> Result<Run> {
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

    let run = RunPrimitive {
        id: id.to_owned(),
        title: request.title,
        status: RunStatus::Queued,
        actor_id: request.actor_id,
        executor_id: request.executor_id,
        thread_id: request.thread_id,
        mission_id: request.mission_id,
        parent_run_id: request.parent_run_id,
        summary: request.summary,
    };
    save_run(workspace, &run).await?;
    Ok(run)
}

/// Loads a persisted run by identifier.
///
/// # Errors
///
/// Returns an error when the run cannot be loaded or decoded.
pub async fn load_run(workspace: &WorkspacePath, run_id: &str) -> Result<Run> {
    let primitive = read_primitive(workspace, RUN_TYPE, run_id).await?;
    run_from_primitive(&primitive)
}

/// Lists all persisted runs.
///
/// # Errors
///
/// Returns an error when run primitives cannot be loaded or decoded.
pub async fn list_runs(workspace: &WorkspacePath) -> Result<Vec<Run>> {
    list_primitives(workspace, RUN_TYPE)
        .await?
        .iter()
        .map(run_from_primitive)
        .collect()
}

/// Marks a queued or retryable run as running.
///
/// # Errors
///
/// Returns an error when the transition is invalid or persistence fails.
pub async fn start_run(workspace: &WorkspacePath, run_id: &str) -> Result<Run> {
    transition_run(workspace, run_id, RunStatus::Running, None).await
}

/// Marks a run succeeded.
///
/// # Errors
///
/// Returns an error when the transition is invalid or persistence fails.
pub async fn complete_run(
    workspace: &WorkspacePath,
    run_id: &str,
    summary: Option<&str>,
) -> Result<Run> {
    transition_run(workspace, run_id, RunStatus::Succeeded, summary).await
}

/// Marks a run failed.
///
/// # Errors
///
/// Returns an error when the transition is invalid or persistence fails.
pub async fn fail_run(
    workspace: &WorkspacePath,
    run_id: &str,
    summary: Option<&str>,
) -> Result<Run> {
    transition_run(workspace, run_id, RunStatus::Failed, summary).await
}

/// Marks a run cancelled.
///
/// # Errors
///
/// Returns an error when the transition is invalid or persistence fails.
pub async fn cancel_run(
    workspace: &WorkspacePath,
    run_id: &str,
    summary: Option<&str>,
) -> Result<Run> {
    transition_run(workspace, run_id, RunStatus::Cancelled, summary).await
}

async fn transition_run(
    workspace: &WorkspacePath,
    run_id: &str,
    next: RunStatus,
    summary: Option<&str>,
) -> Result<Run> {
    let mut run = load_run(workspace, run_id).await?;
    run.status = run
        .status
        .transition_to(next)
        .map_err(WorkgraphError::ValidationError)?;
    if let Some(summary) = summary {
        run.summary = Some(summary.to_owned());
    }
    save_run(workspace, &run).await?;
    Ok(run)
}

async fn save_run(workspace: &WorkspacePath, run: &Run) -> Result<()> {
    let primitive = run_to_primitive(run)?;
    write_primitive(workspace, &Registry::builtins(), &primitive).await?;
    Ok(())
}

fn run_to_primitive(run: &Run) -> Result<StoredPrimitive> {
    let mut extra_fields = std::collections::BTreeMap::new();
    extra_fields.insert(
        "status".to_owned(),
        serde_yaml::to_value(run.status).map_err(encoding_error)?,
    );
    extra_fields.insert(
        "actor_id".to_owned(),
        Value::String(run.actor_id.to_string()),
    );
    extra_fields.insert("thread_id".to_owned(), Value::String(run.thread_id.clone()));
    if let Some(executor_id) = &run.executor_id {
        extra_fields.insert(
            "executor_id".to_owned(),
            Value::String(executor_id.to_string()),
        );
    }
    if let Some(mission_id) = &run.mission_id {
        extra_fields.insert("mission_id".to_owned(), Value::String(mission_id.clone()));
    }
    if let Some(parent_run_id) = &run.parent_run_id {
        extra_fields.insert(
            "parent_run_id".to_owned(),
            Value::String(parent_run_id.clone()),
        );
    }

    Ok(StoredPrimitive {
        frontmatter: PrimitiveFrontmatter {
            r#type: RUN_TYPE.to_owned(),
            id: run.id.clone(),
            title: run.title.clone(),
            extra_fields,
        },
        body: run.summary.clone().unwrap_or_default(),
    })
}

fn run_from_primitive(primitive: &StoredPrimitive) -> Result<Run> {
    if primitive.frontmatter.r#type != RUN_TYPE {
        return Err(WorkgraphError::ValidationError(format!(
            "expected run primitive, found '{}'",
            primitive.frontmatter.r#type
        )));
    }

    let actor_id = primitive
        .frontmatter
        .extra_fields
        .get("actor_id")
        .and_then(string_value)
        .map(ActorId::new)
        .ok_or_else(|| {
            WorkgraphError::ValidationError(format!(
                "run '{}' is missing required actor_id",
                primitive.frontmatter.id
            ))
        })?;
    let thread_id = primitive
        .frontmatter
        .extra_fields
        .get("thread_id")
        .and_then(string_value)
        .map(str::to_owned)
        .ok_or_else(|| {
            WorkgraphError::ValidationError(format!(
                "run '{}' is missing required thread_id",
                primitive.frontmatter.id
            ))
        })?;

    Ok(RunPrimitive {
        id: primitive.frontmatter.id.clone(),
        title: primitive.frontmatter.title.clone(),
        status: primitive
            .frontmatter
            .extra_fields
            .get("status")
            .map_or(Ok(RunStatus::Queued), parse_yaml_value)?,
        actor_id,
        executor_id: primitive
            .frontmatter
            .extra_fields
            .get("executor_id")
            .and_then(string_value)
            .map(ActorId::new),
        thread_id,
        mission_id: primitive
            .frontmatter
            .extra_fields
            .get("mission_id")
            .and_then(string_value)
            .map(str::to_owned),
        parent_run_id: primitive
            .frontmatter
            .extra_fields
            .get("parent_run_id")
            .and_then(string_value)
            .map(str::to_owned),
        summary: (!primitive.body.trim().is_empty()).then(|| primitive.body.clone()),
    })
}

fn parse_yaml_value<T>(value: &Value) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_yaml::from_value::<T>(value.clone()).map_err(encoding_error)
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

fn encoding_error(error: impl std::fmt::Display) -> WorkgraphError {
    WorkgraphError::EncodingError(error.to_string())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use wg_paths::WorkspacePath;
    use wg_store::read_primitive;
    use wg_types::{ActorId, RunStatus};

    use crate::{
        cancel_run, complete_run, create_run, fail_run, load_run, prepare_dispatch, start_run,
    };

    #[tokio::test]
    async fn run_lifecycle_persists_and_roundtrips() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());

        let request = crate::DispatchRequest {
            title: "Cursor analysis run".into(),
            actor_id: ActorId::new("agent:cursor"),
            executor_id: Some(ActorId::new("agent:cursor/subtask")),
            thread_id: "thread-1".into(),
            mission_id: Some("mission-1".into()),
            parent_run_id: Some("run-parent".into()),
            summary: Some("Queued for analysis".into()),
        };
        let created = create_run(&workspace, "run-1", request)
            .await
            .expect("run should be created");
        assert_eq!(created.status, RunStatus::Queued);

        let running = start_run(&workspace, "run-1")
            .await
            .expect("run should start");
        assert_eq!(running.status, RunStatus::Running);

        let completed = complete_run(&workspace, "run-1", Some("Completed successfully"))
            .await
            .expect("run should complete");
        assert_eq!(completed.status, RunStatus::Succeeded);
        assert_eq!(completed.parent_run_id.as_deref(), Some("run-parent"));

        let stored = read_primitive(&workspace, "run", "run-1")
            .await
            .expect("run primitive should be readable");
        assert_eq!(
            stored
                .frontmatter
                .extra_fields
                .get("status")
                .expect("status should be present"),
            &serde_yaml::to_value(RunStatus::Succeeded).expect("status should serialize")
        );

        let loaded = load_run(&workspace, "run-1")
            .await
            .expect("run should roundtrip");
        assert_eq!(loaded.summary.as_deref(), Some("Completed successfully"));
    }

    #[tokio::test]
    async fn run_failure_and_cancellation_are_persisted() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());

        let request = prepare_dispatch("Dispatch run", ActorId::new("agent:cursor"), "thread-2");
        create_run(&workspace, "run-2", request)
            .await
            .expect("run should be created");
        start_run(&workspace, "run-2")
            .await
            .expect("run should start");
        let failed = fail_run(&workspace, "run-2", Some("Execution failed"))
            .await
            .expect("run should fail");
        assert_eq!(failed.status, RunStatus::Failed);

        let cancelled_error = cancel_run(&workspace, "run-2", None)
            .await
            .expect_err("failed run should not be directly cancellable");
        assert!(cancelled_error.to_string().contains("cannot transition"));
    }
}
