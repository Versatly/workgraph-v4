use std::collections::BTreeMap;

use wg_error::Result;
use wg_graph::build_graph;
use wg_paths::WorkspacePath;

use crate::{GraphIssue, WorkspaceStatus};

use super::runtime_support::{
    edge_kind_label, edge_source_label, load_recent_activity, load_thread_evidence_gaps,
};

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
