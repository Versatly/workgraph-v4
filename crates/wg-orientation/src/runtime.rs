//! Runtime orientation functions backed by store, ledger, and graph data.

use std::collections::{BTreeMap, BTreeSet};

use chrono::Utc;
use serde_yaml::Value;
use wg_dispatch::Run;
use wg_error::Result;
use wg_graph::build_graph;
use wg_ledger::{LedgerCursor, LedgerReader};
use wg_mission::Mission;
use wg_paths::WorkspacePath;
use wg_store::{
    AuditedWriteRequest, PrimitiveFrontmatter, StoredPrimitive, read_primitive,
    write_primitive_audited_now,
};
use wg_thread::Thread;
use wg_types::{
    ActorId, FieldDefinition, GraphEdgeKind, GraphEdgeSource, LedgerEntry, LedgerOp, PrimitiveType,
    Registry,
};

use crate::{BriefItem, GraphIssue, RecentActivity, ThreadEvidenceGap};

/// Workspace orientation summary derived from real persisted data.
#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct WorkspaceStatus {
    /// Primitive counts by type.
    pub type_counts: BTreeMap<String, usize>,
    /// Most recent immutable ledger activity.
    pub recent_activity: Vec<RecentActivity>,
    /// Typed graph hygiene issues discovered by the graph engine.
    pub graph_issues: Vec<GraphIssue>,
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
    let graph = build_graph(workspace).await?;
    let mut type_counts = BTreeMap::new();

    for node in graph.nodes() {
        *type_counts.entry(node.primitive_type).or_insert(0) += 1;
    }

    let graph_issues = graph
        .broken_links()
        .iter()
        .map(|broken| GraphIssue {
            source_reference: broken.source.reference(),
            target_reference: broken.target.clone(),
            kind: edge_kind_label(broken.kind).to_owned(),
            provenance: edge_source_label(broken.provenance).to_owned(),
            reason: broken.reason.clone(),
        })
        .collect();

    Ok(WorkspaceStatus {
        type_counts,
        recent_activity: load_recent_activity(workspace, 10).await?,
        graph_issues,
        thread_evidence_gaps: load_thread_evidence_gaps(workspace).await?,
    })
}

/// Builds an actor-scoped brief showing assignments and relevant changes.
///
/// # Errors
///
/// Returns an error when store or ledger data cannot be loaded.
pub async fn brief(workspace: &WorkspacePath, actor: &ActorId) -> Result<ActorBrief> {
    let actor_id = actor.as_str();
    let workspace_status = status(workspace).await?;
    let threads = load_threads(workspace).await?;
    let runs = load_runs(workspace).await?;
    let missions = load_missions(workspace).await?;

    let assigned_threads = threads
        .iter()
        .filter(|thread| {
            thread
                .assigned_actor
                .as_ref()
                .is_some_and(|assigned| assigned.as_str() == actor_id)
        })
        .map(|thread| {
            brief_item(
                "thread",
                &format!("thread/{}", thread.id),
                &thread.title,
                Some(thread.status.as_str().to_owned()),
            )
        })
        .collect::<Vec<_>>();
    let assigned_thread_ids = assigned_threads
        .iter()
        .filter_map(|item| item.reference.as_deref())
        .filter_map(reference_id)
        .collect::<BTreeSet<_>>();

    let assigned_runs = runs
        .iter()
        .filter(|run| {
            run.actor_id.as_str() == actor_id
                || run
                    .executor_id
                    .as_ref()
                    .is_some_and(|executor| executor.as_str() == actor_id)
        })
        .map(|run| {
            brief_item(
                "run",
                &format!("run/{}", run.id),
                &run.title,
                Some(run.status.as_str().to_owned()),
            )
        })
        .collect::<Vec<_>>();
    let assigned_run_ids = assigned_runs
        .iter()
        .filter_map(|item| item.reference.as_deref())
        .filter_map(reference_id)
        .collect::<BTreeSet<_>>();

    let assigned_missions = missions
        .iter()
        .filter(|mission| {
            mission
                .thread_ids
                .iter()
                .any(|thread_id| assigned_thread_ids.contains(thread_id.as_str()))
                || mission
                    .run_ids
                    .iter()
                    .any(|run_id| assigned_run_ids.contains(run_id.as_str()))
        })
        .map(|mission| {
            brief_item(
                "mission",
                &format!("mission/{}", mission.id),
                &mission.title,
                Some(mission.status.as_str().to_owned()),
            )
        })
        .collect::<Vec<_>>();

    let relevant_refs = assigned_threads
        .iter()
        .chain(&assigned_runs)
        .chain(&assigned_missions)
        .filter_map(|item| item.reference.clone())
        .collect::<BTreeSet<_>>();
    let recent_relevant_activity = load_ledger_entries(workspace)
        .await?
        .into_iter()
        .rev()
        .filter(|entry| {
            entry.actor.as_str() == actor_id
                || relevant_refs
                    .contains(&format!("{}/{}", entry.primitive_type, entry.primitive_id))
        })
        .take(10)
        .map(entry_to_recent_activity)
        .collect::<Vec<_>>();

    let mut warnings = Vec::new();
    if assigned_threads.is_empty() && assigned_runs.is_empty() && assigned_missions.is_empty() {
        warnings.push(format!(
            "Actor '{actor_id}' has no assigned threads, runs, or missions"
        ));
    }
    for gap in workspace_status.thread_evidence_gaps.iter().filter(|gap| {
        reference_id(&gap.thread_reference)
            .is_some_and(|thread_id| assigned_thread_ids.contains(thread_id))
    }) {
        warnings.push(format!(
            "Thread '{}' is missing required evidence for: {}",
            gap.thread_reference,
            gap.missing_criteria.join(", ")
        ));
    }
    for issue in workspace_status.graph_issues.iter().filter(|issue| {
        relevant_refs.contains(&issue.source_reference)
            || relevant_refs.contains(&issue.target_reference)
    }) {
        warnings.push(format!(
            "Graph issue: {} -> {} [{} via {}] ({})",
            issue.source_reference,
            issue.target_reference,
            issue.kind,
            issue.provenance,
            issue.reason
        ));
    }

    Ok(ActorBrief {
        actor: actor_id.to_owned(),
        assigned_threads,
        assigned_runs,
        assigned_missions,
        recent_relevant_activity,
        warnings,
    })
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
    let id = format!(
        "{}-{}",
        slugify(working_on),
        Utc::now().format("%Y%m%d%H%M%S")
    );
    let title = format!("Checkpoint: {}", working_on.trim());
    let created_at = Utc::now().to_rfc3339();
    let primitive = StoredPrimitive {
        frontmatter: PrimitiveFrontmatter {
            r#type: "checkpoint".to_owned(),
            id: id.clone(),
            title,
            extra_fields: BTreeMap::from([
                (
                    "working_on".to_owned(),
                    Value::String(working_on.trim().to_owned()),
                ),
                ("focus".to_owned(), Value::String(focus.trim().to_owned())),
                ("created_at".to_owned(), Value::String(created_at)),
            ]),
        },
        body: format!("## Working on\n{working_on}\n\n## Focus\n{focus}\n"),
    };

    write_primitive_audited_now(
        workspace,
        &checkpoint_registry(),
        &primitive,
        AuditedWriteRequest::new(ActorId::new("system:workgraph"), LedgerOp::Create)
            .with_note(format!("Saved checkpoint '{}'", id)),
    )
    .await?;
    read_primitive(workspace, "checkpoint", &id).await
}

async fn load_recent_activity(
    workspace: &WorkspacePath,
    limit: usize,
) -> Result<Vec<RecentActivity>> {
    Ok(load_ledger_entries(workspace)
        .await?
        .into_iter()
        .rev()
        .take(limit)
        .map(entry_to_recent_activity)
        .collect())
}

async fn load_ledger_entries(workspace: &WorkspacePath) -> Result<Vec<LedgerEntry>> {
    let reader = LedgerReader::new(workspace.as_path().to_path_buf());
    let (entries, _) = reader.read_from(LedgerCursor::default()).await?;
    Ok(entries)
}

async fn load_thread_evidence_gaps(workspace: &WorkspacePath) -> Result<Vec<ThreadEvidenceGap>> {
    Ok(load_threads(workspace)
        .await?
        .into_iter()
        .filter_map(|thread| {
            let missing_criteria = missing_criteria(&thread);
            (!missing_criteria.is_empty()).then(|| ThreadEvidenceGap {
                thread_reference: format!("thread/{}", thread.id),
                missing_criteria,
            })
        })
        .collect())
}

async fn load_threads(workspace: &WorkspacePath) -> Result<Vec<Thread>> {
    wg_thread::list_threads(workspace).await
}

async fn load_missions(workspace: &WorkspacePath) -> Result<Vec<Mission>> {
    wg_mission::list_missions(workspace).await
}

async fn load_runs(workspace: &WorkspacePath) -> Result<Vec<Run>> {
    wg_dispatch::list_runs(workspace).await
}

fn entry_to_recent_activity(entry: LedgerEntry) -> RecentActivity {
    RecentActivity {
        ts: entry.ts.to_rfc3339(),
        actor: entry.actor.to_string(),
        op: format!("{:?}", entry.op).to_lowercase(),
        reference: format!("{}/{}", entry.primitive_type, entry.primitive_id),
    }
}

fn brief_item(kind: &str, reference: &str, title: &str, detail: Option<String>) -> BriefItem {
    BriefItem {
        kind: kind.to_owned(),
        reference: Some(reference.to_owned()),
        title: title.to_owned(),
        detail,
    }
}

fn reference_id(reference: &str) -> Option<&str> {
    reference.split_once('/').map(|(_, id)| id)
}

fn missing_criteria(thread: &Thread) -> Vec<String> {
    wg_thread::unsatisfied_exit_criteria(thread)
}

fn checkpoint_registry() -> Registry {
    let mut registry = Registry::builtins();
    if registry.get_type("checkpoint").is_none() {
        registry.types.push(PrimitiveType::new(
            "checkpoint",
            "checkpoints",
            "Saved orientation checkpoint",
            vec![
                FieldDefinition::new("id", "string", "Stable checkpoint identifier", true, false),
                FieldDefinition::new("title", "string", "Checkpoint title", true, false),
                FieldDefinition::new("working_on", "string", "Current work item", true, false),
                FieldDefinition::new("focus", "string", "Current focus", true, false),
                FieldDefinition::new(
                    "created_at",
                    "datetime",
                    "Checkpoint timestamp",
                    true,
                    false,
                ),
            ],
        ));
    }
    registry
}

fn slugify(input: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;

    for character in input.chars() {
        let lower = character.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            slug.push(lower);
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }

    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "checkpoint".to_owned()
    } else {
        trimmed.to_owned()
    }
}

fn edge_kind_label(kind: GraphEdgeKind) -> &'static str {
    match kind {
        GraphEdgeKind::Reference => "reference",
        GraphEdgeKind::Relationship => "relationship",
        GraphEdgeKind::Assignment => "assignment",
        GraphEdgeKind::Containment => "containment",
        GraphEdgeKind::Evidence => "evidence",
        GraphEdgeKind::Trigger => "trigger",
    }
}

fn edge_source_label(source: GraphEdgeSource) -> &'static str {
    match source {
        GraphEdgeSource::WikiLink => "wiki_link",
        GraphEdgeSource::Field => "field",
        GraphEdgeSource::RelationshipPrimitive => "relationship_primitive",
        GraphEdgeSource::EvidenceRecord => "evidence_record",
        GraphEdgeSource::TriggerRule => "trigger_rule",
    }
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
