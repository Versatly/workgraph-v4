//! Human-readable output rendering for CLI command results.

use std::fmt::Write as _;

use serde_yaml::Value;

use super::{
    CommandOutput, CreateOutput, InitOutput, QueryOutput, SchemaOutput, ShowOutput, SkillsOutput,
    StatusOutput,
};
use wg_orientation::WorkspaceBrief;

/// Renders a structured command output to human-readable text.
#[must_use]
pub fn render(output: &CommandOutput) -> String {
    match output {
        CommandOutput::Init(output) => render_init(output),
        CommandOutput::Brief(output) => render_brief(output),
        CommandOutput::Status(output) => render_status(output),
        CommandOutput::Skills(output) => render_skills(output),
        CommandOutput::Schema(output) => render_schema(output),
        CommandOutput::Create(output) => render_create(output),
        CommandOutput::Query(output) => render_query(output),
        CommandOutput::Show(output) => render_show(output),
    }
}

/// Renders a structured command failure to human-readable text.
#[must_use]
pub fn render_failure(command: Option<&str>, error: &anyhow::Error) -> String {
    let mut rendered = String::new();
    let _ = writeln!(
        rendered,
        "Command failed{}",
        command
            .map(|command| format!(": {command}"))
            .unwrap_or_default()
    );
    let _ = writeln!(rendered, "{}", error);
    let _ = writeln!(rendered);
    let _ = writeln!(rendered, "Try:");
    let _ = writeln!(rendered, "- workgraph --json skills");
    let _ = writeln!(rendered, "- workgraph --json schema");
    rendered.trim_end().to_owned()
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

fn render_brief(output: &WorkspaceBrief) -> String {
    let mut rendered = String::new();
    let _ = writeln!(
        rendered,
        "Workspace brief [{}]: {} ({})",
        output.lens.as_str(),
        output.workspace_name,
        output.workspace_id
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
    for section in &output.sections {
        let _ = writeln!(rendered, "{} ({})", section.title, section.summary);
        if section.items.is_empty() {
            let _ = writeln!(rendered, "- none");
        } else {
            for item in &section.items {
                let mut line = format!("- {}", item.title);
                if let Some(detail) = &item.detail {
                    line.push_str(&format!(" — {detail}"));
                }
                if let Some(reference) = &item.reference {
                    line.push_str(&format!(" ({reference})"));
                }
                let _ = writeln!(rendered, "{line}");
            }
        }
    }

    let _ = writeln!(rendered, "Recent activity:");
    if output.recent_activity.is_empty() {
        let _ = writeln!(rendered, "- none");
    } else {
        for entry in &output.recent_activity {
            let _ = writeln!(
                rendered,
                "- {} {}/{} {}",
                entry.ts, entry.reference, entry.actor, entry.op
            );
        }
    }

    if !output.warnings.is_empty() {
        let _ = writeln!(rendered, "Warnings:");
        for warning in &output.warnings {
            let _ = writeln!(rendered, "- {warning}");
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

fn render_skills(output: &SkillsOutput) -> String {
    let mut rendered = String::new();
    let _ = writeln!(
        rendered,
        "WorkGraph CLI skills (recommended format: {})",
        output.recommended_format
    );
    let _ = writeln!(rendered, "Workflows:");
    for workflow in &output.workflows {
        let _ = writeln!(rendered, "- {} — {}", workflow.title, workflow.description);
        for command in &workflow.commands {
            let _ = writeln!(rendered, "  • {command}");
        }
    }
    let _ = writeln!(rendered, "Commands:");
    for command in &output.commands {
        let _ = writeln!(rendered, "- {} — {}", command.name, command.description);
    }
    rendered.trim_end().to_owned()
}

fn render_schema(output: &SchemaOutput) -> String {
    let mut rendered = String::new();
    let _ = writeln!(rendered, "CLI schema version: {}", output.schema_version);
    let _ = writeln!(rendered, "Envelope fields:");
    for field in &output.envelope_fields {
        let _ = writeln!(
            rendered,
            "- {} ({}){} — {}",
            field.name,
            field.field_type,
            if field.required { ", required" } else { "" },
            field.description
        );
    }
    let _ = writeln!(rendered, "Commands:");
    for command in &output.commands {
        let _ = writeln!(rendered, "- {} — {}", command.name, command.description);
        for argument in &command.arguments {
            let _ = writeln!(
                rendered,
                "  • {}{} — {}",
                argument.name,
                if argument.required { " (required)" } else { "" },
                argument.description
            );
        }
        let _ = writeln!(rendered, "  example: {}", command.example);
    }
    rendered.trim_end().to_owned()
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
