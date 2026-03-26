use std::collections::BTreeSet;

use wg_dispatch::Run;
use wg_error::Result;
use wg_mission::Mission;
use wg_paths::WorkspacePath;
use wg_thread::Thread;
use wg_types::ActorId;

use crate::{
    ActorBrief, BriefItem, GraphIssue, RecentActivity, ThreadEvidenceGap, WorkspaceStatus,
    runtime_support::{
        entry_to_recent_activity, load_ledger_entries, load_missions, load_runs, load_threads,
        reference_id,
    },
    status_runtime::status,
};

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

    let assigned_threads = assigned_threads(&threads, actor_id);
    let assigned_thread_ids = assigned_reference_ids(&assigned_threads);

    let assigned_runs = assigned_runs(&runs, actor_id);
    let assigned_run_ids = assigned_reference_ids(&assigned_runs);

    let assigned_missions = assigned_missions(&missions, &assigned_thread_ids, &assigned_run_ids);
    let relevant_refs = relevant_refs(&assigned_threads, &assigned_runs, &assigned_missions);
    let recent_relevant_activity =
        recent_relevant_activity(workspace, actor_id, &relevant_refs).await?;
    let warnings = warnings_for_actor(
        actor_id,
        &workspace_status,
        &assigned_threads,
        &assigned_runs,
        &assigned_missions,
        &assigned_thread_ids,
        &relevant_refs,
    );

    Ok(ActorBrief {
        actor: actor_id.to_owned(),
        assigned_threads,
        assigned_runs,
        assigned_missions,
        recent_relevant_activity,
        warnings,
    })
}

fn assigned_threads(threads: &[Thread], actor_id: &str) -> Vec<BriefItem> {
    threads
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
        .collect()
}

fn assigned_runs(runs: &[Run], actor_id: &str) -> Vec<BriefItem> {
    runs.iter()
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
        .collect()
}

fn assigned_missions(
    missions: &[Mission],
    assigned_thread_ids: &BTreeSet<String>,
    assigned_run_ids: &BTreeSet<String>,
) -> Vec<BriefItem> {
    missions
        .iter()
        .filter(|mission| {
            mission
                .thread_ids
                .iter()
                .any(|thread_id| assigned_thread_ids.contains(thread_id))
                || mission
                    .run_ids
                    .iter()
                    .any(|run_id| assigned_run_ids.contains(run_id))
        })
        .map(|mission| {
            brief_item(
                "mission",
                &format!("mission/{}", mission.id),
                &mission.title,
                Some(mission.status.as_str().to_owned()),
            )
        })
        .collect()
}

fn assigned_reference_ids(items: &[BriefItem]) -> BTreeSet<String> {
    items
        .iter()
        .filter_map(|item| item.reference.as_deref())
        .filter_map(reference_id)
        .map(str::to_owned)
        .collect()
}

fn relevant_refs(
    assigned_threads: &[BriefItem],
    assigned_runs: &[BriefItem],
    assigned_missions: &[BriefItem],
) -> BTreeSet<String> {
    assigned_threads
        .iter()
        .chain(assigned_runs)
        .chain(assigned_missions)
        .filter_map(|item| item.reference.clone())
        .collect()
}

async fn recent_relevant_activity(
    workspace: &WorkspacePath,
    actor_id: &str,
    relevant_refs: &BTreeSet<String>,
) -> Result<Vec<RecentActivity>> {
    Ok(load_ledger_entries(workspace)
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
        .collect())
}

fn warnings_for_actor(
    actor_id: &str,
    workspace_status: &WorkspaceStatus,
    assigned_threads: &[BriefItem],
    assigned_runs: &[BriefItem],
    assigned_missions: &[BriefItem],
    assigned_thread_ids: &BTreeSet<String>,
    relevant_refs: &BTreeSet<String>,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if assigned_threads.is_empty() && assigned_runs.is_empty() && assigned_missions.is_empty() {
        warnings.push(format!(
            "Actor '{actor_id}' has no assigned threads, runs, or missions"
        ));
    }
    warnings.extend(thread_gap_warnings(
        &workspace_status.thread_evidence_gaps,
        assigned_thread_ids,
    ));
    warnings.extend(graph_issue_warnings(
        &workspace_status.graph_issues,
        relevant_refs,
    ));
    warnings
}

fn thread_gap_warnings(
    gaps: &[ThreadEvidenceGap],
    assigned_thread_ids: &BTreeSet<String>,
) -> Vec<String> {
    gaps.iter()
        .filter(|gap| {
            reference_id(&gap.thread_reference)
                .is_some_and(|thread_id| assigned_thread_ids.contains(thread_id))
        })
        .map(|gap| {
            format!(
                "Thread '{}' is missing required evidence for: {}",
                gap.thread_reference,
                gap.missing_criteria.join(", ")
            )
        })
        .collect()
}

fn graph_issue_warnings(issues: &[GraphIssue], relevant_refs: &BTreeSet<String>) -> Vec<String> {
    issues
        .iter()
        .filter(|issue| {
            relevant_refs.contains(&issue.source_reference)
                || relevant_refs.contains(&issue.target_reference)
        })
        .map(|issue| {
            format!(
                "Graph issue: {} -> {} [{} via {}] ({})",
                issue.source_reference,
                issue.target_reference,
                issue.kind,
                issue.provenance,
                issue.reason
            )
        })
        .collect()
}

fn brief_item(kind: &str, reference: &str, title: &str, detail: Option<String>) -> BriefItem {
    BriefItem {
        kind: kind.to_owned(),
        reference: Some(reference.to_owned()),
        title: title.to_owned(),
        detail,
    }
}
