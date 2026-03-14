//! Human-readable output rendering for CLI command results.

use std::fmt::Write as _;

use serde_yaml::Value;

use super::{
    BriefOutput, CommandOutput, CreateOutput, InitOutput, QueryOutput, ShowOutput, StatusOutput,
};

/// Renders a structured command output to human-readable text.
#[must_use]
pub fn render(output: &CommandOutput) -> String {
    match output {
        CommandOutput::Init(output) => render_init(output),
        CommandOutput::Brief(output) => render_brief(output),
        CommandOutput::Status(output) => render_status(output),
        CommandOutput::Create(output) => render_create(output),
        CommandOutput::Query(output) => render_query(output),
        CommandOutput::Show(output) => render_show(output),
    }
}

fn render_init(output: &InitOutput) -> String {
    let mut rendered = String::new();
    let _ = writeln!(
        rendered,
        "Initialized WorkGraph workspace '{}' ({})",
        output.config.workspace_name, output.config.workspace_id
    );
    let _ = writeln!(rendered, "Root: {}", output.config.root_dir);
    let _ = writeln!(rendered, "Config: {}", output.config_path);
    let _ = writeln!(rendered, "Registry: {}", output.registry_path);
    let _ = writeln!(rendered, "Ledger: {}", output.ledger_path);
    let _ = writeln!(rendered, "Primitive directories:");
    for directory in &output.created_directories {
        let _ = writeln!(rendered, "- {directory}");
    }
    rendered.trim_end().to_owned()
}

fn render_brief(output: &BriefOutput) -> String {
    let mut rendered = String::new();
    let _ = writeln!(
        rendered,
        "Workspace brief: {} ({})",
        output.workspace_name, output.workspace_id
    );
    let _ = writeln!(rendered, "Root: {}", output.workspace_root);
    match &output.default_actor_id {
        Some(default_actor) => {
            let _ = writeln!(rendered, "Default actor: {default_actor}");
        }
        None => {
            let _ = writeln!(rendered, "Default actor: none");
        }
    }
    let _ = writeln!(rendered, "Key counts:");
    for (primitive_type, count) in &output.type_counts {
        let _ = writeln!(rendered, "- {primitive_type}: {count}");
    }
    render_titles_section(&mut rendered, "Orgs", &output.orgs);
    render_titles_section(&mut rendered, "Clients", &output.clients);
    render_titles_section(&mut rendered, "Agents", &output.agents);

    let _ = writeln!(rendered, "Recent activity:");
    if output.recent_entries.is_empty() {
        let _ = writeln!(rendered, "- none");
    } else {
        for entry in &output.recent_entries {
            let _ = writeln!(
                rendered,
                "- {} {}/{} {}",
                entry.ts.to_rfc3339(),
                entry.primitive_type,
                entry.primitive_id,
                format!("{:?}", entry.op).to_lowercase()
            );
        }
    }

    rendered.trim_end().to_owned()
}

fn render_status(output: &StatusOutput) -> String {
    let mut rendered = String::new();
    let _ = writeln!(
        rendered,
        "Workspace: {} ({})",
        output.config.workspace_name, output.workspace_root
    );
    let _ = writeln!(rendered, "Type counts:");
    for (primitive_type, count) in &output.type_counts {
        let _ = writeln!(rendered, "- {primitive_type}: {count}");
    }

    match &output.last_entry {
        Some(entry) => {
            let _ = writeln!(
                rendered,
                "Last ledger entry: {} {} {}/{}",
                entry.ts.to_rfc3339(),
                entry.actor,
                entry.primitive_type,
                entry.primitive_id
            );
        }
        None => {
            let _ = writeln!(rendered, "Last ledger entry: none");
        }
    }

    rendered.trim_end().to_owned()
}

fn render_create(output: &CreateOutput) -> String {
    format!(
        "Created {} at {}\nLedger hash: {}",
        output.reference, output.path, output.ledger_entry.hash
    )
}

fn render_query(output: &QueryOutput) -> String {
    let mut rendered = String::new();
    let _ = writeln!(
        rendered,
        "{} result(s) for type '{}':",
        output.count, output.primitive_type
    );
    for item in &output.items {
        let _ = writeln!(
            rendered,
            "- {}/{} — {}",
            item.frontmatter.r#type, item.frontmatter.id, item.frontmatter.title
        );
    }
    rendered.trim_end().to_owned()
}

fn render_show(output: &ShowOutput) -> String {
    let mut rendered = String::new();
    let _ = writeln!(rendered, "{}", output.reference);
    let _ = writeln!(rendered, "type: {}", output.primitive.frontmatter.r#type);
    let _ = writeln!(rendered, "id: {}", output.primitive.frontmatter.id);
    let _ = writeln!(rendered, "title: {}", output.primitive.frontmatter.title);
    for (key, value) in &output.primitive.frontmatter.extra_fields {
        let _ = writeln!(rendered, "{key}: {}", yaml_scalar_or_inline(value));
    }
    let _ = writeln!(rendered);
    rendered.push_str(output.primitive.body.trim_end());
    rendered
}

fn render_titles_section(rendered: &mut String, heading: &str, titles: &[String]) {
    let _ = writeln!(rendered, "{heading}:");
    if titles.is_empty() {
        let _ = writeln!(rendered, "- none");
    } else {
        for title in titles {
            let _ = writeln!(rendered, "- {title}");
        }
    }
}

fn yaml_scalar_or_inline(value: &Value) -> String {
    match value {
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        other => serde_yaml::to_string(other)
            .map(|text| text.trim().replace('\n', " "))
            .unwrap_or_else(|_| "<unrenderable>".to_owned()),
    }
}
