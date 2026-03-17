//! Orientation-building services that assemble reusable workspace briefs.

use std::collections::BTreeMap;

use anyhow::Context;
use wg_orientation::{BriefItem, BriefSection, ContextLens, RecentActivity, WorkspaceBrief};
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
    let registry = app.load_registry().await?;
    let entries = app.read_ledger_entries().await?;
    let mut type_counts = BTreeMap::new();

    for primitive_type in registry.list_types() {
        let primitives = list_primitives(app.workspace(), &primitive_type.name)
            .await
            .with_context(|| format!("failed to list primitive type '{}'", primitive_type.name))?;
        type_counts.insert(primitive_type.name.clone(), primitives.len());
    }

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
                "policies",
                "Policies",
                list_items(app, "policy", 5).await?,
            ));
            sections.push(section(
                "agents",
                "Agents",
                list_items(app, "agent", 5).await?,
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
                "decisions",
                "Delivery-relevant decisions",
                list_items(app, "decision", 8).await?,
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
                "decisions",
                "Decisions that may shape policy",
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
                "patterns",
                "Patterns useful for agents",
                list_items(app, "pattern", 8).await?,
            ));
            sections.push(section(
                "lessons",
                "Lessons useful for agents",
                list_items(app, "lesson", 8).await?,
            ));
            sections.push(section(
                "policies",
                "Operational policies",
                list_items(app, "policy", 6).await?,
            ));
        }
    }

    sections.retain(|section| !section.items.is_empty());
    let recent_activity = entries
        .into_iter()
        .rev()
        .take(5)
        .map(|entry| RecentActivity {
            ts: entry.ts.to_rfc3339(),
            actor: entry.actor.to_string(),
            op: format!("{:?}", entry.op).to_lowercase(),
            reference: format!("{}/{}", entry.primitive_type, entry.primitive_id),
        })
        .collect::<Vec<_>>();

    Ok(WorkspaceBrief {
        lens,
        workspace_id: config.workspace_id.to_string(),
        workspace_name: config.workspace_name,
        workspace_root: config.root_dir,
        default_actor_id: config.default_actor_id.map(|actor| actor.to_string()),
        type_counts,
        sections,
        recent_activity,
        warnings: build_warnings(app).await?,
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

async fn build_warnings(app: &AppContext) -> anyhow::Result<Vec<String>> {
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
