//! Orientation-building services that assemble reusable workspace briefs.

use wg_orientation::{BriefItem, BriefSection, WorkspaceBrief, WorkspaceStatus};

use crate::app::AppContext;

/// Builds a reusable workspace brief optimized for entering agents.
///
/// # Errors
///
/// Returns an error when workspace metadata, primitives, or ledger entries cannot be read.
pub async fn build_workspace_brief(app: &AppContext) -> anyhow::Result<WorkspaceBrief> {
    let config = app.load_config().await?;
    let workspace_status = wg_orientation::status(app.workspace()).await?;
    let sections = vec![
        section(
            "workspace",
            "Workspace identity",
            vec![BriefItem {
                kind: "workspace".to_owned(),
                reference: Some(config.workspace_id.to_string()),
                title: config.workspace_name.clone(),
                detail: Some(config.root_dir.clone()),
            }],
        ),
        primitive_counts_section(&workspace_status),
        recent_ledger_section(&workspace_status),
        next_actions_section(),
    ];

    Ok(WorkspaceBrief {
        lens: wg_orientation::ContextLens::Workspace,
        workspace_id: config.workspace_id.to_string(),
        workspace_name: config.workspace_name,
        workspace_root: config.root_dir,
        default_actor_id: config.default_actor_id.map(|actor| actor.to_string()),
        type_counts: workspace_status.type_counts.clone(),
        sections,
        recent_activity: workspace_status
            .recent_activity
            .iter()
            .take(10)
            .cloned()
            .collect(),
        warnings: build_warnings(&workspace_status),
    })
}

fn section(key: &str, title: &str, items: Vec<BriefItem>) -> BriefSection {
    let summary = if items.is_empty() {
        "none recorded".to_owned()
    } else {
        format!("{} item(s)", items.len())
    };

    BriefSection {
        key: key.to_owned(),
        title: title.to_owned(),
        summary,
        items,
    }
}

fn primitive_counts_section(workspace_status: &WorkspaceStatus) -> BriefSection {
    let items = workspace_status
        .type_counts
        .iter()
        .map(|(primitive_type, count)| BriefItem {
            kind: "count".to_owned(),
            reference: None,
            title: primitive_type.clone(),
            detail: Some(count.to_string()),
        })
        .collect::<Vec<_>>();
    section("primitive_counts", "Primitive counts by type", items)
}

fn recent_ledger_section(workspace_status: &WorkspaceStatus) -> BriefSection {
    let items = workspace_status
        .recent_activity
        .iter()
        .take(10)
        .map(|entry| BriefItem {
            kind: "ledger_entry".to_owned(),
            reference: Some(entry.reference.clone()),
            title: format!("{} {}", entry.op, entry.reference),
            detail: Some(format!("{} by {}", entry.ts, entry.actor)),
        })
        .collect::<Vec<_>>();
    section("recent_ledger", "Recent ledger entries", items)
}

fn next_actions_section() -> BriefSection {
    section(
        "next_actions",
        "Suggested next actions",
        vec![
            BriefItem {
                kind: "next_action".to_owned(),
                reference: None,
                title: "Inspect one primitive".to_owned(),
                detail: Some("workgraph show org/versatly".to_owned()),
            },
            BriefItem {
                kind: "next_action".to_owned(),
                reference: None,
                title: "Query primitive type".to_owned(),
                detail: Some("workgraph query org".to_owned()),
            },
            BriefItem {
                kind: "next_action".to_owned(),
                reference: None,
                title: "Capture organization context".to_owned(),
                detail: Some("workgraph create org --title \"Versatly\"".to_owned()),
            },
        ],
    )
}

fn build_warnings(workspace_status: &WorkspaceStatus) -> Vec<String> {
    let mut warnings = Vec::new();

    for gap in workspace_status.thread_evidence_gaps.iter().take(5) {
        warnings.push(format!(
            "Evidence gap: {} is missing {}",
            gap.thread_reference,
            gap.missing_criteria.join(", ")
        ));
    }
    for issue in workspace_status.graph_issues.iter().take(5) {
        warnings.push(format!(
            "Graph issue: {} -> {} [{} via {}]",
            issue.source_reference, issue.target_reference, issue.kind, issue.provenance
        ));
    }

    warnings
}
