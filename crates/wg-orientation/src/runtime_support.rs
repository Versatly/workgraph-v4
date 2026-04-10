use wg_dispatch::Run;
use wg_error::Result;
use wg_ledger::{LedgerCursor, LedgerReader};
use wg_mission::Mission;
use wg_paths::WorkspacePath;
use wg_thread::Thread;
use wg_trigger::{Trigger, TriggerReceipt};
use wg_types::{GraphEdgeKind, GraphEdgeSource, LedgerEntry, TriggerPlanDecision};

use crate::{
    GraphOrphan, RecentActivity, ThreadEvidenceGap, TriggerHealth, TriggerPlannedActionSummary,
    TriggerReceiptSummary,
};

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

pub(crate) async fn load_triggers(workspace: &WorkspacePath) -> Result<Vec<Trigger>> {
    wg_trigger::list_triggers(workspace).await
}

pub(crate) async fn load_trigger_receipts(
    workspace: &WorkspacePath,
) -> Result<Vec<TriggerReceipt>> {
    wg_trigger::list_trigger_receipts(workspace).await
}

pub(crate) async fn load_trigger_health(workspace: &WorkspacePath) -> Result<Vec<TriggerHealth>> {
    Ok(load_triggers(workspace)
        .await?
        .into_iter()
        .map(|trigger| TriggerHealth {
            trigger_reference: format!("trigger/{}", trigger.id),
            status: trigger.status.as_str().to_owned(),
            last_evaluated_at: trigger.subscription_state.as_ref().and_then(|state| {
                state
                    .last_evaluated_at
                    .map(|timestamp| timestamp.to_rfc3339())
            }),
            last_matched_at: trigger.subscription_state.as_ref().and_then(|state| {
                state
                    .last_matched_at
                    .map(|timestamp| timestamp.to_rfc3339())
            }),
            last_event_id: trigger
                .subscription_state
                .as_ref()
                .and_then(|state| state.last_event_id.clone()),
            last_receipt_id: trigger
                .subscription_state
                .as_ref()
                .and_then(|state| state.last_receipt_id.clone()),
        })
        .collect())
}

pub(crate) async fn load_recent_trigger_receipts(
    workspace: &WorkspacePath,
    limit: usize,
) -> Result<Vec<TriggerReceiptSummary>> {
    Ok(load_trigger_receipts(workspace)
        .await?
        .into_iter()
        .rev()
        .take(limit)
        .map(|receipt| {
            let pending_plans = receipt
                .action_outcomes
                .iter()
                .filter(|outcome| outcome.decision == TriggerPlanDecision::Allow)
                .count();
            let suppressed_plans = receipt
                .action_outcomes
                .iter()
                .filter(|outcome| outcome.decision == TriggerPlanDecision::Deny)
                .count();
            TriggerReceiptSummary {
                receipt_reference: format!("trigger_receipt/{}", receipt.id),
                trigger_reference: format!("trigger/{}", receipt.trigger_id),
                event_source: receipt.event_source.as_str().to_owned(),
                event_name: receipt.event_name,
                subject_reference: receipt.subject_reference,
                occurred_at: receipt.occurred_at.to_rfc3339(),
                pending_plans,
                suppressed_plans,
            }
        })
        .collect())
}

pub(crate) async fn pending_trigger_actions(
    workspace: &WorkspacePath,
) -> Result<TriggerPlannedActionSummary> {
    let mut summary = TriggerPlannedActionSummary {
        pending_count: 0,
        suppressed_count: 0,
    };
    for receipt in load_trigger_receipts(workspace).await? {
        for outcome in receipt.action_outcomes {
            match outcome.decision {
                TriggerPlanDecision::Allow => summary.pending_count += 1,
                TriggerPlanDecision::Deny => summary.suppressed_count += 1,
            }
        }
    }
    Ok(summary)
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

pub(crate) fn orphan_nodes(graph: &wg_graph::GraphSnapshot) -> Vec<GraphOrphan> {
    graph
        .orphans()
        .into_iter()
        .map(|node| GraphOrphan {
            reference: node.reference(),
        })
        .collect()
}
