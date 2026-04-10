#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Durable run persistence and lifecycle helpers for WorkGraph dispatch.

use chrono::{DateTime, Utc};
use serde_yaml::Value;
use wg_error::{Result, WorkgraphError};
use wg_paths::WorkspacePath;
use wg_store::{
    AuditedWriteRequest, PrimitiveFrontmatter, StoredPrimitive, list_primitives, read_primitive,
    write_primitive_audited_now,
};
use wg_types::{ActorId, ExternalRef, Registry, RunPrimitive, RunStatus};

mod mutation;

const RUN_TYPE: &str = "run";
const SYSTEM_ACTOR: &str = "system:workgraph";

/// Typed run model persisted by this crate.
pub type Run = RunPrimitive;

pub use mutation::RunMutationService;

/// Durable request used to create a new run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DispatchRequest {
    /// Run title.
    pub title: String,
    /// Optional classification for the kind of bounded work attempt.
    pub kind: Option<String>,
    /// Optional source that created or observed the run receipt.
    pub source: Option<String>,
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
    /// Links back to authoritative external records related to this run.
    pub external_refs: Vec<ExternalRef>,
}

/// Builds a minimal dispatch request.
#[must_use]
pub fn prepare_dispatch(title: &str, actor_id: ActorId, thread_id: &str) -> DispatchRequest {
    DispatchRequest {
        title: title.to_owned(),
        kind: None,
        source: None,
        actor_id,
        executor_id: None,
        thread_id: thread_id.to_owned(),
        mission_id: None,
        parent_run_id: None,
        summary: None,
        external_refs: Vec::new(),
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
    RunMutationService::new(workspace)
        .create_run_as(id, request, system_actor())
        .await
}

/// Creates and persists a queued run, auditing as the supplied invoking actor.
///
/// # Errors
///
/// Returns an error when required identifiers are invalid or persistence fails.
pub async fn create_run_as(
    workspace: &WorkspacePath,
    audit_actor: ActorId,
    id: &str,
    request: DispatchRequest,
) -> Result<Run> {
    RunMutationService::new(workspace)
        .create_run_as(id, request, audit_actor)
        .await
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
    RunMutationService::new(workspace)
        .start_run_as(system_actor(), run_id)
        .await
}

/// Marks a queued or retryable run as running, auditing as the supplied invoking actor.
///
/// # Errors
///
/// Returns an error when the transition is invalid or persistence fails.
pub async fn start_run_as(
    workspace: &WorkspacePath,
    audit_actor: ActorId,
    run_id: &str,
) -> Result<Run> {
    RunMutationService::new(workspace)
        .start_run_as(audit_actor, run_id)
        .await
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
    RunMutationService::new(workspace)
        .complete_run_as(system_actor(), run_id, summary)
        .await
}

/// Marks a run succeeded, auditing as the supplied invoking actor.
///
/// # Errors
///
/// Returns an error when the transition is invalid or persistence fails.
pub async fn complete_run_as(
    workspace: &WorkspacePath,
    audit_actor: ActorId,
    run_id: &str,
    summary: Option<&str>,
) -> Result<Run> {
    RunMutationService::new(workspace)
        .complete_run_as(audit_actor, run_id, summary)
        .await
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
    RunMutationService::new(workspace)
        .fail_run_as(system_actor(), run_id, summary)
        .await
}

/// Marks a run failed, auditing as the supplied invoking actor.
///
/// # Errors
///
/// Returns an error when the transition is invalid or persistence fails.
pub async fn fail_run_as(
    workspace: &WorkspacePath,
    audit_actor: ActorId,
    run_id: &str,
    summary: Option<&str>,
) -> Result<Run> {
    RunMutationService::new(workspace)
        .fail_run_as(audit_actor, run_id, summary)
        .await
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
    RunMutationService::new(workspace)
        .cancel_run_as(system_actor(), run_id, summary)
        .await
}

/// Marks a run cancelled, auditing as the supplied invoking actor.
///
/// # Errors
///
/// Returns an error when the transition is invalid or persistence fails.
pub async fn cancel_run_as(
    workspace: &WorkspacePath,
    audit_actor: ActorId,
    run_id: &str,
    summary: Option<&str>,
) -> Result<Run> {
    RunMutationService::new(workspace)
        .cancel_run_as(audit_actor, run_id, summary)
        .await
}

pub(crate) async fn save_run_with_audit(
    workspace: &WorkspacePath,
    run: &Run,
    audit: AuditedWriteRequest,
) -> Result<wg_types::LedgerEntry> {
    let primitive = run_to_primitive(run)?;
    let (_, ledger_entry) =
        write_primitive_audited_now(workspace, &Registry::builtins(), &primitive, audit).await?;
    Ok(ledger_entry)
}

fn run_to_primitive(run: &Run) -> Result<StoredPrimitive> {
    let mut extra_fields = std::collections::BTreeMap::new();
    extra_fields.insert(
        "status".to_owned(),
        serde_yaml::to_value(run.status).map_err(encoding_error)?,
    );
    if let Some(kind) = &run.kind {
        extra_fields.insert("kind".to_owned(), Value::String(kind.clone()));
    }
    if let Some(source) = &run.source {
        extra_fields.insert("source".to_owned(), Value::String(source.clone()));
    }
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
    set_datetime_field(&mut extra_fields, "started_at", run.started_at);
    set_datetime_field(&mut extra_fields, "ended_at", run.ended_at);
    if !run.external_refs.is_empty() {
        extra_fields.insert(
            "external_refs".to_owned(),
            serde_yaml::to_value(&run.external_refs).map_err(encoding_error)?,
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
        kind: primitive
            .frontmatter
            .extra_fields
            .get("kind")
            .and_then(string_value)
            .map(str::to_owned),
        source: primitive
            .frontmatter
            .extra_fields
            .get("source")
            .and_then(string_value)
            .map(str::to_owned),
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
        started_at: primitive
            .frontmatter
            .extra_fields
            .get("started_at")
            .map_or(Ok(None), parse_optional_datetime)?,
        ended_at: primitive
            .frontmatter
            .extra_fields
            .get("ended_at")
            .map_or(Ok(None), parse_optional_datetime)?,
        summary: (!primitive.body.trim().is_empty()).then(|| primitive.body.clone()),
        external_refs: primitive
            .frontmatter
            .extra_fields
            .get("external_refs")
            .map_or(Ok(Vec::new()), parse_yaml_value)?,
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

fn system_actor() -> ActorId {
    ActorId::new(SYSTEM_ACTOR)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use tempfile::tempdir;
    use wg_ledger::LedgerReader;
    use wg_paths::WorkspacePath;
    use wg_store::read_primitive;
    use wg_types::{ActorId, ExternalRef, LedgerOp, RunStatus};

    use crate::{
        cancel_run, complete_run, create_run, create_run_as, fail_run, load_run, prepare_dispatch,
        start_run, start_run_as,
    };

    #[tokio::test]
    async fn run_lifecycle_persists_and_roundtrips() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());

        let request = crate::DispatchRequest {
            title: "Cursor analysis run".into(),
            kind: Some("agent_pass".into()),
            source: Some("sdk".into()),
            actor_id: ActorId::new("agent:cursor"),
            executor_id: Some(ActorId::new("agent:cursor/subtask")),
            thread_id: "thread-1".into(),
            mission_id: Some("mission-1".into()),
            parent_run_id: Some("run-parent".into()),
            summary: Some("Queued for analysis".into()),
            external_refs: vec![ExternalRef {
                provider: "cursor".into(),
                kind: "session".into(),
                url: "cursor://sessions/abc123".into(),
                id: Some("abc123".into()),
                metadata: BTreeMap::from([("workspace".into(), "workgraph-v4".into())]),
            }],
        };
        let created = create_run(&workspace, "run-1", request)
            .await
            .expect("run should be created");
        assert_eq!(created.status, RunStatus::Queued);

        let running = start_run(&workspace, "run-1")
            .await
            .expect("run should start");
        assert_eq!(running.status, RunStatus::Running);
        assert!(running.started_at.is_some());
        assert!(running.ended_at.is_none());

        let completed = complete_run(&workspace, "run-1", Some("Completed successfully"))
            .await
            .expect("run should complete");
        assert_eq!(completed.status, RunStatus::Succeeded);
        assert_eq!(completed.kind.as_deref(), Some("agent_pass"));
        assert_eq!(completed.source.as_deref(), Some("sdk"));
        assert_eq!(completed.parent_run_id.as_deref(), Some("run-parent"));
        assert!(completed.started_at.is_some());
        assert!(completed.ended_at.is_some());
        assert_eq!(completed.external_refs.len(), 1);

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
        assert_eq!(loaded.external_refs[0].provider, "cursor");

        let (entries, _) = LedgerReader::new(temp_dir.path().to_path_buf())
            .read_from(Default::default())
            .await
            .expect("ledger should be readable");
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].op, LedgerOp::Create);
        assert_eq!(entries[1].op, LedgerOp::Start);
        assert_eq!(entries[2].op, LedgerOp::Done);
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
        assert!(failed.started_at.is_some());
        assert!(failed.ended_at.is_some());

        let cancelled_error = cancel_run(&workspace, "run-2", None)
            .await
            .expect_err("failed run should not be directly cancellable");
        assert!(cancelled_error.to_string().contains("cannot transition"));
    }

    #[tokio::test]
    async fn actor_aware_audit_helpers_record_invoking_actor() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());

        let request = prepare_dispatch("Audit run", ActorId::new("agent:cursor"), "thread-3");
        create_run_as(
            &workspace,
            ActorId::new("person:pedro"),
            "audit-run",
            request,
        )
        .await
        .expect("run should be created");

        start_run_as(&workspace, ActorId::new("person:pedro"), "audit-run")
            .await
            .expect("run should start");

        let (entries, _) = LedgerReader::new(temp_dir.path().to_path_buf())
            .read_from(Default::default())
            .await
            .expect("ledger should be readable");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].actor.as_str(), "person:pedro");
        assert_eq!(entries[1].actor.as_str(), "person:pedro");
        assert_eq!(entries[0].op, LedgerOp::Create);
        assert_eq!(entries[1].op, LedgerOp::Start);
    }
}
