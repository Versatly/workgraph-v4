//! Implementation of the `workgraph brief` command.

use std::collections::BTreeMap;

use wg_orientation::ContextLens;

use crate::app::AppContext;
use crate::output::{BriefOutput, WorkspaceIdentity};
use crate::services::orientation::build_workspace_brief;

/// Produces a structured agent-first workspace orientation brief.
///
/// # Errors
///
/// Returns an error when workspace metadata, primitives, or ledger entries cannot be read.
pub async fn handle(app: &AppContext, _lens: ContextLens) -> anyhow::Result<BriefOutput> {
    let orientation = build_workspace_brief(app).await?;
    let recent_ledger_entries = app
        .read_ledger_entries()
        .await?
        .into_iter()
        .rev()
        .take(10)
        .collect::<Vec<_>>();

    Ok(BriefOutput {
        workspace: WorkspaceIdentity {
            id: orientation.workspace_id.clone(),
            name: orientation.workspace_name.clone(),
            root: orientation.workspace_root.clone(),
            default_actor_id: orientation.default_actor_id.clone(),
        },
        primitive_counts: orientation.type_counts.clone(),
        recent_ledger_entries,
        suggested_next_actions: build_dynamic_suggestions(&orientation.type_counts),
        orientation,
    })
}

/// Builds context-aware next action suggestions based on what exists in the workspace.
fn build_dynamic_suggestions(type_counts: &BTreeMap<String, usize>) -> Vec<String> {
    let total: usize = type_counts.values().sum();
    if total == 0 {
        // Fresh workspace — guide toward first primitives.
        return vec![
            "workgraph create org --title \"<your org>\"".to_owned(),
            "workgraph create project --title \"<project name>\"".to_owned(),
            "workgraph capabilities".to_owned(),
        ];
    }

    let mut suggestions = Vec::new();

    // Suggest inspecting what exists.
    if let Some((existing_type, _)) = type_counts.iter().find(|(_, count)| **count > 0) {
        suggestions.push(format!("workgraph query {existing_type}"));
    }

    // Suggest creating missing high-value types.
    for missing_type in ["org", "decision", "agent", "project"] {
        if type_counts.get(missing_type).copied().unwrap_or(0) == 0 {
            suggestions.push(format!(
                "workgraph create {missing_type} --title \"<title>\""
            ));
            break;
        }
    }

    if type_counts.get("trigger").copied().unwrap_or(0) > 0 {
        suggestions.push("workgraph trigger replay".to_owned());
    }

    suggestions.push("workgraph status".to_owned());
    suggestions
}
