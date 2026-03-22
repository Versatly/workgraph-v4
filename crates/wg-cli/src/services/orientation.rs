//! Orientation-building services that assemble reusable workspace briefs.

use wg_orientation::{BriefItem, BriefSection, ContextLens, WorkspaceBrief, WorkspaceStatus};
use wg_store::{StoredPrimitive, list_primitives};

use crate::app::AppContext;

/// Builds a reusable workspace brief for the requested context lens.
///
/// # Errors
///
/// Returns an error when workspace metadata, primitives, or ledger entries cannot be read.
pub async fn build_workspace_brief(
    app: &AppContext,
    lens: ContextLens,
) -> anyhow::Result<WorkspaceBrief> {
    let config = app.load_config().await?;
    let workspace_status = wg_orientation::status(app.workspace()).await?;
    let mut sections = Vec::new();

    match lens {
        ContextLens::Workspace => {
            sections.push(section(
                "orgs",
                "Organizations",
                list_items(app, "org", 5).await?,
            ));
            sections.push(section(
                "clients",
                "Clients",
                list_items(app, "client", 5).await?,
            ));
            sections.push(section(
                "projects",
                "Projects",
                list_items(app, "project", 5).await?,
            ));
            sections.push(section(
                "decisions",
                "Recent decisions",
                list_items(app, "decision", 5).await?,
            ));
            sections.push(section(
                "threads",
                "Active threads",
                list_items(app, "thread", 5).await?,
            ));
            sections.push(section(
                "missions",
                "Missions",
                list_items(app, "mission", 5).await?,
            ));
            sections.push(section("runs", "Runs", list_items(app, "run", 5).await?));
            sections.push(section(
                "triggers",
                "Triggers",
                list_items(app, "trigger", 5).await?,
            ));
        }
        ContextLens::Delivery => {
            sections.push(section(
                "clients",
                "Clients",
                list_items(app, "client", 8).await?,
            ));
            sections.push(section(
                "projects",
                "Projects",
                list_items(app, "project", 8).await?,
            ));
            sections.push(section(
                "threads",
                "Delivery threads",
                list_items(app, "thread", 8).await?,
            ));
            sections.push(section(
                "missions",
                "Delivery missions",
                list_items(app, "mission", 6).await?,
            ));
            sections.push(section(
                "runs",
                "Recent runs",
                list_items(app, "run", 6).await?,
            ));
            sections.push(section(
                "decisions",
                "Delivery decisions",
                list_items(app, "decision", 6).await?,
            ));
        }
        ContextLens::Policy => {
            sections.push(section(
                "policies",
                "Policies",
                list_items(app, "policy", 8).await?,
            ));
            sections.push(section(
                "patterns",
                "Patterns",
                list_items(app, "pattern", 8).await?,
            ));
            sections.push(section(
                "lessons",
                "Lessons",
                list_items(app, "lesson", 8).await?,
            ));
            sections.push(section(
                "triggers",
                "Triggers",
                list_items(app, "trigger", 8).await?,
            ));
            sections.push(section(
                "decisions",
                "Decisions",
                list_items(app, "decision", 6).await?,
            ));
        }
        ContextLens::Agents => {
            sections.push(section(
                "agents",
                "Agents",
                list_items(app, "agent", 8).await?,
            ));
            sections.push(section(
                "threads",
                "Assigned threads",
                list_items(app, "thread", 8).await?,
            ));
            sections.push(section(
                "runs",
                "Assigned runs",
                list_items(app, "run", 8).await?,
            ));
            sections.push(section(
                "missions",
                "Coordinated missions",
                list_items(app, "mission", 6).await?,
            ));
            sections.push(section(
                "policies",
                "Operational policies",
                list_items(app, "policy", 6).await?,
            ));
        }
    }

    sections.retain(|section| !section.items.is_empty());

    Ok(WorkspaceBrief {
        lens,
        workspace_id: config.workspace_id.to_string(),
        workspace_name: config.workspace_name,
        workspace_root: config.root_dir,
        default_actor_id: config.default_actor_id.map(|actor| actor.to_string()),
        type_counts: workspace_status.type_counts.clone(),
        sections,
        recent_activity: workspace_status.recent_activity.clone(),
        warnings: build_warnings(app, &workspace_status).await?,
    })
}

async fn list_items(
    app: &AppContext,
    primitive_type: &str,
    limit: usize,
) -> anyhow::Result<Vec<BriefItem>> {
    Ok(list_primitives(app.workspace(), primitive_type)
        .await?
        .into_iter()
        .take(limit)
        .map(|primitive| to_brief_item(primitive_type, &primitive))
        .collect())
}

fn to_brief_item(primitive_type: &str, primitive: &StoredPrimitive) -> BriefItem {
    BriefItem {
        kind: primitive_type.to_owned(),
        reference: Some(format!(
            "{}/{}",
            primitive.frontmatter.r#type, primitive.frontmatter.id
        )),
        title: primitive.frontmatter.title.clone(),
        detail: primitive
            .frontmatter
            .extra_fields
            .get("summary")
            .or_else(|| primitive.frontmatter.extra_fields.get("status"))
            .or_else(|| primitive.frontmatter.extra_fields.get("mission_status"))
            .or_else(|| primitive.frontmatter.extra_fields.get("assigned_actor"))
            .or_else(|| primitive.frontmatter.extra_fields.get("actor_id"))
            .or_else(|| primitive.frontmatter.extra_fields.get("scope"))
            .map(value_to_summary)
            .filter(|value| !value.is_empty()),
    }
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

async fn build_warnings(
    app: &AppContext,
    workspace_status: &WorkspaceStatus,
) -> anyhow::Result<Vec<String>> {
    let mut warnings = Vec::new();

    if list_primitives(app.workspace(), "org").await?.is_empty() {
        warnings.push("No organizations recorded yet".to_owned());
    }
    if list_primitives(app.workspace(), "policy").await?.is_empty() {
        warnings.push("No policies recorded yet".to_owned());
    }
    if list_primitives(app.workspace(), "agent").await?.is_empty() {
        warnings.push("No agents recorded yet".to_owned());
    }

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

    Ok(warnings)
}

fn value_to_summary(value: &serde_yaml::Value) -> String {
    match value {
        serde_yaml::Value::Bool(value) => value.to_string(),
        serde_yaml::Value::Number(value) => value.to_string(),
        serde_yaml::Value::String(value) => value.clone(),
        other => serde_yaml::to_string(other)
            .map(|text| text.trim().replace('\n', " "))
            .unwrap_or_default(),
    }
}
