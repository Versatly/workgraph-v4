//! Runtime orientation functions backed by store, ledger, and graph data.

use std::collections::BTreeMap;

use wg_error::Result;
use wg_paths::WorkspacePath;
use wg_store::StoredPrimitive;
use wg_types::ActorId;

use crate::{
    BriefItem, CheckpointMutationService, GraphIssue, GraphOrphan, RecentActivity,
    ThreadEvidenceGap, brief_runtime, status_runtime,
};

/// Workspace orientation summary derived from real persisted data.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceStatus {
    /// Primitive counts by type.
    pub type_counts: BTreeMap<String, usize>,
    /// Most recent immutable ledger activity.
    pub recent_activity: Vec<RecentActivity>,
    /// Typed graph hygiene issues discovered by the graph engine.
    pub graph_issues: Vec<GraphIssue>,
    /// Primitives with no inbound graph edges.
    pub orphan_nodes: Vec<GraphOrphan>,
    /// Threads with unsatisfied required exit criteria.
    pub thread_evidence_gaps: Vec<ThreadEvidenceGap>,
}

/// Actor-specific orientation brief based on assignment and recent activity.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct ActorBrief {
    /// Target actor identifier.
    pub actor: String,
    /// Threads currently assigned to this actor.
    pub assigned_threads: Vec<BriefItem>,
    /// Runs currently assigned to this actor.
    pub assigned_runs: Vec<BriefItem>,
    /// Missions related to assigned threads or runs.
    pub assigned_missions: Vec<BriefItem>,
    /// Recent activity relevant to this actor.
    pub recent_relevant_activity: Vec<RecentActivity>,
    /// Warnings that need attention.
    pub warnings: Vec<String>,
}

/// Builds a workspace status summary from persisted primitives and ledger entries.
///
/// # Errors
///
/// Returns an error when graph, store, or ledger data cannot be loaded.
pub async fn status(workspace: &WorkspacePath) -> Result<WorkspaceStatus> {
    status_runtime::status(workspace).await
}

/// Builds an actor-scoped brief showing assignments and relevant changes.
///
/// # Errors
///
/// Returns an error when store or ledger data cannot be loaded.
pub async fn brief(workspace: &WorkspacePath, actor: &ActorId) -> Result<ActorBrief> {
    brief_runtime::brief(workspace, actor).await
}

/// Saves a checkpoint primitive for current work focus.
///
/// # Errors
///
/// Returns an error when checkpoint persistence fails.
pub async fn checkpoint(
    workspace: &WorkspacePath,
    working_on: &str,
    focus: &str,
) -> Result<StoredPrimitive> {
    CheckpointMutationService::new(workspace)
        .checkpoint(working_on, focus)
        .await
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use chrono::{TimeZone, Utc};
    use serde_yaml::Value;
    use tempfile::tempdir;
    use wg_clock::MockClock;
    use wg_ledger::{LedgerEntryDraft, LedgerWriter};
    use wg_paths::WorkspacePath;
    use wg_store::{PrimitiveFrontmatter, StoredPrimitive, write_primitive};
    use wg_types::{
        ActorId, EventPattern, EventSourceKind, EvidenceItem, LedgerOp, Registry, RunStatus,
        ThreadExitCriterion, ThreadStatus, TriggerActionPlan, TriggerStatus,
    };

    use crate::{brief, checkpoint, status};

    fn primitive(
        primitive_type: &str,
        id: &str,
        title: &str,
        body: &str,
        extra_fields: impl IntoIterator<Item = (&'static str, Value)>,
    ) -> StoredPrimitive {
        let mut fields = BTreeMap::new();
        for (key, value) in extra_fields {
            fields.insert(key.to_owned(), value);
        }
        StoredPrimitive {
            frontmatter: PrimitiveFrontmatter {
                r#type: primitive_type.to_owned(),
                id: id.to_owned(),
                title: title.to_owned(),
                extra_fields: fields,
            },
            body: body.to_owned(),
        }
    }

    #[tokio::test]
    async fn status_reads_real_counts_recent_activity_and_contract_gaps() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());
        write_primitive(
            &workspace,
            &Registry::builtins(),
            &primitive(
                "decision",
                "alpha",
                "Alpha",
                "Linked to [[decision/missing]].",
                [],
            ),
        )
        .await
        .expect("decision should write");
        write_primitive(
            &workspace,
            &Registry::builtins(),
            &primitive(
                "thread",
                "thread-1",
                "Thread 1",
                "Thread body",
                [
                    (
                        "status",
                        serde_yaml::to_value(ThreadStatus::Active)
                            .expect("thread status should serialize"),
                    ),
                    ("assigned_actor", Value::String("pedro".to_owned())),
                    (
                        "exit_criteria",
                        serde_yaml::to_value(vec![ThreadExitCriterion {
                            id: "criterion-1".to_owned(),
                            title: "External verification".to_owned(),
                            description: None,
                            required: true,
                            reference: Some("decision/alpha".to_owned()),
                        }])
                        .expect("exit criteria should serialize"),
                    ),
                ],
            ),
        )
        .await
        .expect("thread should write");

        let clock = MockClock::new(
            Utc.with_ymd_and_hms(2026, 3, 17, 10, 0, 0)
                .single()
                .expect("timestamp should be valid"),
        );
        let writer = LedgerWriter::new(temp_dir.path(), clock);
        writer
            .append(LedgerEntryDraft::new(
                ActorId::new("pedro"),
                LedgerOp::Create,
                "decision",
                "alpha",
                vec!["title".to_owned()],
            ))
            .await
            .expect("ledger append should succeed");

        let snapshot = status(&workspace).await.expect("status should load");
        assert_eq!(snapshot.type_counts.get("decision"), Some(&1));
        assert_eq!(snapshot.type_counts.get("thread"), Some(&1));
        assert_eq!(snapshot.recent_activity.len(), 1);
        assert_eq!(snapshot.recent_activity[0].reference, "decision/alpha");
        assert!(!snapshot.graph_issues.is_empty());
        assert!(
            snapshot
                .graph_issues
                .iter()
                .any(|issue| issue.source_reference == "decision/alpha")
        );
        assert!(
            snapshot
                .orphan_nodes
                .iter()
                .any(|orphan| orphan.reference == "decision/alpha")
        );
        assert_eq!(snapshot.thread_evidence_gaps.len(), 1);
        assert_eq!(
            snapshot.thread_evidence_gaps[0].thread_reference,
            "thread/thread-1"
        );
    }

    #[tokio::test]
    async fn brief_returns_actor_assignments_runs_and_relevant_changes() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());
        write_primitive(
            &workspace,
            &Registry::builtins(),
            &primitive(
                "thread",
                "thread-1",
                "Thread 1",
                "Thread body",
                [
                    (
                        "status",
                        serde_yaml::to_value(ThreadStatus::Active)
                            .expect("thread status should serialize"),
                    ),
                    ("assigned_actor", Value::String("pedro".to_owned())),
                    (
                        "exit_criteria",
                        serde_yaml::to_value(vec![ThreadExitCriterion {
                            id: "criterion-1".to_owned(),
                            title: "Verification".to_owned(),
                            description: None,
                            required: true,
                            reference: None,
                        }])
                        .expect("exit criteria should serialize"),
                    ),
                    (
                        "evidence",
                        serde_yaml::to_value(vec![EvidenceItem {
                            id: "evidence-1".to_owned(),
                            title: "Verifier note".to_owned(),
                            description: None,
                            reference: Some("decision/alpha".to_owned()),
                            satisfies: vec!["criterion-1".to_owned()],
                            recorded_at: None,
                            source: Some("manual".to_owned()),
                        }])
                        .expect("evidence should serialize"),
                    ),
                ],
            ),
        )
        .await
        .expect("thread should write");
        write_primitive(
            &workspace,
            &Registry::builtins(),
            &primitive(
                "run",
                "run-1",
                "Run 1",
                "Executor summary",
                [
                    (
                        "status",
                        serde_yaml::to_value(RunStatus::Running)
                            .expect("run status should serialize"),
                    ),
                    ("actor_id", Value::String("pedro".to_owned())),
                    ("thread_id", Value::String("thread-1".to_owned())),
                ],
            ),
        )
        .await
        .expect("run should write");
        write_primitive(
            &workspace,
            &Registry::builtins(),
            &primitive(
                "mission",
                "mission-1",
                "Mission 1",
                "Objective",
                [(
                    "thread_ids",
                    Value::Sequence(vec![Value::String("thread-1".to_owned())]),
                )],
            ),
        )
        .await
        .expect("mission should write");
        write_primitive(
            &workspace,
            &Registry::builtins(),
            &primitive(
                "trigger",
                "trigger-1",
                "Trigger 1",
                "",
                [
                    (
                        "status",
                        serde_yaml::to_value(TriggerStatus::Active)
                            .expect("trigger status should serialize"),
                    ),
                    (
                        "event_pattern",
                        serde_yaml::to_value(EventPattern {
                            source: EventSourceKind::Ledger,
                            event_name: None,
                            ops: vec![LedgerOp::Done],
                            primitive_types: vec!["thread".to_owned()],
                            primitive_id: None,
                            field_names: vec!["evidence".to_owned()],
                            provider: None,
                        })
                        .expect("event pattern should serialize"),
                    ),
                    (
                        "action_plans",
                        serde_yaml::to_value(vec![TriggerActionPlan {
                            kind: "rebrief_actor".to_owned(),
                            target_reference: Some("thread/thread-1".to_owned()),
                            instruction: "Refresh the brief".to_owned(),
                        }])
                        .expect("action plans should serialize"),
                    ),
                ],
            ),
        )
        .await
        .expect("trigger should write");

        let clock = MockClock::new(
            Utc.with_ymd_and_hms(2026, 3, 17, 10, 0, 0)
                .single()
                .expect("timestamp should be valid"),
        );
        let writer = LedgerWriter::new(temp_dir.path(), clock.clone());
        writer
            .append(LedgerEntryDraft::new(
                ActorId::new("ana"),
                LedgerOp::Create,
                "thread",
                "thread-1",
                vec!["status".to_owned()],
            ))
            .await
            .expect("first append should succeed");
        clock.set(
            Utc.with_ymd_and_hms(2026, 3, 17, 10, 5, 0)
                .single()
                .expect("timestamp should be valid"),
        );
        writer
            .append(LedgerEntryDraft::new(
                ActorId::new("pedro"),
                LedgerOp::Update,
                "run",
                "run-1",
                vec!["status".to_owned()],
            ))
            .await
            .expect("second append should succeed");

        let actor_brief = brief(&workspace, &ActorId::new("pedro"))
            .await
            .expect("brief should load");
        assert_eq!(actor_brief.assigned_threads.len(), 1);
        assert_eq!(actor_brief.assigned_runs.len(), 1);
        assert_eq!(actor_brief.assigned_missions.len(), 1);
        assert!(!actor_brief.recent_relevant_activity.is_empty());
    }

    #[tokio::test]
    async fn checkpoint_persists_a_checkpoint_primitive() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());

        let created = checkpoint(&workspace, "Kernel implementation", "Finish policy tests")
            .await
            .expect("checkpoint should write");
        assert_eq!(created.frontmatter.r#type, "checkpoint");
        assert_eq!(
            created
                .frontmatter
                .extra_fields
                .get("working_on")
                .expect("working_on should exist"),
            &Value::String("Kernel implementation".to_owned())
        );
    }
}
