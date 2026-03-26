use wg_dispatch::Run;
use wg_error::Result;
use wg_ledger::{LedgerCursor, LedgerReader};
use wg_mission::Mission;
use wg_paths::WorkspacePath;
use wg_thread::Thread;
use wg_types::{GraphEdgeKind, GraphEdgeSource, LedgerEntry};

use crate::{RecentActivity, ThreadEvidenceGap};

pub(crate) async fn load_recent_activity(
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

pub(crate) async fn load_ledger_entries(workspace: &WorkspacePath) -> Result<Vec<LedgerEntry>> {
    let reader = LedgerReader::new(workspace.as_path().to_path_buf());
    let (entries, _) = reader.read_from(LedgerCursor::default()).await?;
    Ok(entries)
}

pub(crate) async fn load_thread_evidence_gaps(
    workspace: &WorkspacePath,
) -> Result<Vec<ThreadEvidenceGap>> {
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

pub(crate) async fn load_threads(workspace: &WorkspacePath) -> Result<Vec<Thread>> {
    wg_thread::list_threads(workspace).await
}

pub(crate) async fn load_missions(workspace: &WorkspacePath) -> Result<Vec<Mission>> {
    wg_mission::list_missions(workspace).await
}

pub(crate) async fn load_runs(workspace: &WorkspacePath) -> Result<Vec<Run>> {
    wg_dispatch::list_runs(workspace).await
}

pub(crate) fn entry_to_recent_activity(entry: LedgerEntry) -> RecentActivity {
    RecentActivity {
        ts: entry.ts.to_rfc3339(),
        actor: entry.actor.to_string(),
        op: format!("{:?}", entry.op).to_lowercase(),
        reference: format!("{}/{}", entry.primitive_type, entry.primitive_id),
    }
}

pub(crate) fn reference_id(reference: &str) -> Option<&str> {
    reference.split_once('/').map(|(_, id)| id)
}

pub(crate) fn missing_criteria(thread: &Thread) -> Vec<String> {
    wg_thread::unsatisfied_exit_criteria(thread)
}

pub(crate) fn edge_kind_label(kind: GraphEdgeKind) -> &'static str {
    match kind {
        GraphEdgeKind::Reference => "reference",
        GraphEdgeKind::Relationship => "relationship",
        GraphEdgeKind::Assignment => "assignment",
        GraphEdgeKind::Containment => "containment",
        GraphEdgeKind::Evidence => "evidence",
        GraphEdgeKind::Trigger => "trigger",
    }
}

pub(crate) fn edge_source_label(source: GraphEdgeSource) -> &'static str {
    match source {
        GraphEdgeSource::WikiLink => "wiki_link",
        GraphEdgeSource::Field => "field",
        GraphEdgeSource::RelationshipPrimitive => "relationship_primitive",
        GraphEdgeSource::EvidenceRecord => "evidence_record",
        GraphEdgeSource::TriggerRule => "trigger_rule",
    }
}
