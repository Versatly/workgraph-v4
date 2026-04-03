//! Human-readable output rendering for CLI command results.

use std::fmt::Write as _;

use serde_yaml::Value;

use super::{
    CapabilitiesOutput, CheckpointOutput, CommandOutput, CreateOutput, InitOutput, MissionOutput,
    QueryOutput, RunOutput, SchemaOutput, ShowOutput, StatusOutput, ThreadOutput, TriggerOutput,
};
use wg_orientation::WorkspaceBrief;

/// Renders a structured command output to human-readable text.
#[must_use]
pub fn render(output: &CommandOutput) -> String {
    match output {
        CommandOutput::Init(output) => render_init(output),
        CommandOutput::Brief(output) => render_brief(output),
        CommandOutput::Status(output) => render_status(output),
        CommandOutput::Capabilities(output) => render_capabilities(output),
        CommandOutput::Schema(output) => render_schema(output),
        CommandOutput::Create(output) => render_create(output),
        CommandOutput::Thread(output) => render_thread(output),
        CommandOutput::Mission(output) => render_mission(output),
        CommandOutput::Run(output) => render_run(output),
        CommandOutput::Trigger(output) => render_trigger(output),
        CommandOutput::Checkpoint(output) => render_checkpoint(output),
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
    let _ = writeln!(rendered, "- workgraph --json capabilities");
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
            let _ = writeln!(rendered, "- {} {} {}", entry.ts, entry.reference, entry.op);
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

    let _ = writeln!(rendered, "Recent activity:");
    if output.recent_activity.is_empty() {
        let _ = writeln!(rendered, "- none");
    } else {
        for entry in &output.recent_activity {
            let _ = writeln!(rendered, "- {} {} {}", entry.ts, entry.reference, entry.op);
        }
    }

    let _ = writeln!(rendered, "Evidence gaps:");
    if output.thread_evidence_gaps.is_empty() {
        let _ = writeln!(rendered, "- none");
    } else {
        for gap in &output.thread_evidence_gaps {
            let _ = writeln!(
                rendered,
                "- {} missing {}",
                gap.thread_reference,
                gap.missing_criteria.join(", ")
            );
        }
    }

    let _ = writeln!(rendered, "Graph issues:");
    if output.graph_issues.is_empty() {
        let _ = writeln!(rendered, "- none");
    } else {
        for issue in &output.graph_issues {
            let _ = writeln!(
                rendered,
                "- {} -> {} [{} via {}] ({})",
                issue.source_reference,
                issue.target_reference,
                issue.kind,
                issue.provenance,
                issue.reason
            );
        }
    }

    rendered.trim_end().to_owned()
}

fn render_create(output: &CreateOutput) -> String {
    let mut rendered = String::new();
    if output.dry_run {
        let _ = writeln!(
            rendered,
            "Dry run: would create {} at {}",
            output.reference, output.path
        );
        if output.idempotent {
            let _ = writeln!(
                rendered,
                "Existing primitive already matches the requested identity."
            );
        }
        return rendered.trim_end().to_owned();
    }

    if output.idempotent {
        let _ = writeln!(
            rendered,
            "Reused existing {} at {}",
            output.reference, output.path
        );
    } else {
        let _ = writeln!(rendered, "Created {} at {}", output.reference, output.path);
    }

    if let Some(ledger_entry) = &output.ledger_entry {
        let _ = writeln!(rendered, "Ledger hash: {}", ledger_entry.hash);
    }

    rendered.trim_end().to_owned()
}

fn render_capabilities(output: &CapabilitiesOutput) -> String {
    let mut rendered = String::new();
    let _ = writeln!(
        rendered,
        "WorkGraph CLI capabilities (recommended format: {})",
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
    let _ = writeln!(rendered, "Primitive contracts:");
    for contract in &output.primitive_contracts {
        let _ = writeln!(rendered, "- {} — {}", contract.name, contract.description);
        for note in &contract.notes {
            let _ = writeln!(rendered, "  • {note}");
        }
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
    let _ = writeln!(rendered, "Primitive contracts:");
    for contract in &output.primitive_contracts {
        let _ = writeln!(rendered, "- {} — {}", contract.name, contract.description);
        if !contract.required_fields.is_empty() {
            let required = contract
                .required_fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(rendered, "  • required: {required}");
        }
        if !contract.optional_fields.is_empty() {
            let optional = contract
                .optional_fields
                .iter()
                .map(|field| field.name.as_str())
                .collect::<Vec<_>>()
                .join(", ");
            let _ = writeln!(rendered, "  • optional: {optional}");
        }
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
    let primitive = &output.primitive;
    let mut rendered = String::new();
    let _ = writeln!(rendered, "{}", output.reference);
    let _ = writeln!(rendered, "type: {}", primitive.frontmatter.r#type);
    let _ = writeln!(rendered, "id: {}", primitive.frontmatter.id);
    let _ = writeln!(rendered, "title: {}", primitive.frontmatter.title);

    match primitive.frontmatter.r#type.as_str() {
        "thread" => render_thread_sections(&mut rendered, primitive),
        "mission" => render_mission_sections(&mut rendered, primitive),
        "run" => render_run_sections(&mut rendered, primitive),
        "trigger" => render_trigger_sections(&mut rendered, primitive),
        _ => render_generic_fields(&mut rendered, primitive),
    }

    if !primitive.body.trim().is_empty() {
        let _ = writeln!(rendered);
        rendered.push_str(primitive.body.trim_end());
    }
    rendered
}

fn render_thread(output: &ThreadOutput) -> String {
    let mut rendered = String::new();
    let prefix = if output.dry_run { "Dry run: " } else { "" };
    let _ = writeln!(
        rendered,
        "{prefix}thread {} {}",
        output.action, output.reference
    );
    rendered.push_str(&render_show(&ShowOutput {
        reference: output.reference.clone(),
        primitive: output.thread.clone(),
    }));
    rendered
}

fn render_mission(output: &MissionOutput) -> String {
    let mut rendered = String::new();
    let prefix = if output.dry_run { "Dry run: " } else { "" };
    let _ = writeln!(rendered, "{prefix}mission {}", output.action);
    if let Some(progress) = &output.progress {
        let _ = writeln!(
            rendered,
            "{}: {}/{} threads complete",
            output.reference, progress.completed_threads, progress.total_threads
        );
        return rendered.trim_end().to_owned();
    }
    if let Some(mission) = &output.mission {
        rendered.push_str(&render_show(&ShowOutput {
            reference: output.reference.clone(),
            primitive: mission.clone(),
        }));
    }
    rendered
}

fn render_run(output: &RunOutput) -> String {
    let mut rendered = String::new();
    let prefix = if output.dry_run { "Dry run: " } else { "" };
    let _ = writeln!(rendered, "{prefix}run {} {}", output.action, output.reference);
    rendered.push_str(&render_show(&ShowOutput {
        reference: output.reference.clone(),
        primitive: output.run.clone(),
    }));
    rendered
}

fn render_trigger(output: &TriggerOutput) -> String {
    let mut rendered = String::new();
    let prefix = if output.dry_run { "Dry run: " } else { "" };
    let _ = writeln!(rendered, "{prefix}trigger {}", output.action);
    if let Some(reference) = &output.reference {
        let _ = writeln!(rendered, "reference: {reference}");
    }
    if let Some(trigger) = &output.trigger {
        rendered.push_str(&render_show(&ShowOutput {
            reference: output
                .reference
                .clone()
                .unwrap_or_else(|| format!("{}/{}", trigger.frontmatter.r#type, trigger.frontmatter.id)),
            primitive: trigger.clone(),
        }));
        let _ = writeln!(rendered);
    }
    if let Some(entry) = &output.evaluated_entry {
        let _ = writeln!(
            rendered,
            "evaluated ledger entry: {} {} {}/{}",
            entry.ts.to_rfc3339(),
            format!("{:?}", entry.op).to_lowercase(),
            entry.primitive_type,
            entry.primitive_id
        );
    }
    if output.matches.is_empty() {
        let _ = writeln!(rendered, "matches: none");
    } else {
        let _ = writeln!(rendered, "matches:");
        for matched in &output.matches {
            let _ = writeln!(rendered, "- {} ({})", matched.title, matched.trigger_id);
            for action_plan in &matched.action_plans {
                let _ = writeln!(
                    rendered,
                    "  • {} -> {}",
                    action_plan.kind, action_plan.instruction
                );
            }
        }
    }
    rendered.trim_end().to_owned()
}

fn render_checkpoint(output: &CheckpointOutput) -> String {
    let mut rendered = String::new();
    let prefix = if output.dry_run { "Dry run: " } else { "" };
    let _ = writeln!(
        rendered,
        "{prefix}checkpoint {} {}",
        output.action, output.reference
    );
    rendered.push_str(&render_show(&ShowOutput {
        reference: output.reference.clone(),
        primitive: output.checkpoint.clone(),
    }));
    rendered
}

fn render_thread_sections(rendered: &mut String, primitive: &wg_store::StoredPrimitive) {
    render_field(
        rendered,
        "status",
        primitive.frontmatter.extra_fields.get("status"),
    );
    render_field(
        rendered,
        "assigned_actor",
        primitive.frontmatter.extra_fields.get("assigned_actor"),
    );
    render_field(
        rendered,
        "parent_mission_id",
        primitive.frontmatter.extra_fields.get("parent_mission_id"),
    );
    render_section_list(
        rendered,
        "exit_criteria",
        primitive.frontmatter.extra_fields.get("exit_criteria"),
    );
    render_section_list(
        rendered,
        "evidence",
        primitive.frontmatter.extra_fields.get("evidence"),
    );
    render_section_list(
        rendered,
        "update_actions",
        primitive.frontmatter.extra_fields.get("update_actions"),
    );
    render_section_list(
        rendered,
        "completion_actions",
        primitive.frontmatter.extra_fields.get("completion_actions"),
    );
}

fn render_mission_sections(rendered: &mut String, primitive: &wg_store::StoredPrimitive) {
    render_field(
        rendered,
        "status",
        primitive.frontmatter.extra_fields.get("status"),
    );
    render_section_list(
        rendered,
        "thread_ids",
        primitive.frontmatter.extra_fields.get("thread_ids"),
    );
    render_section_list(
        rendered,
        "run_ids",
        primitive.frontmatter.extra_fields.get("run_ids"),
    );
}

fn render_run_sections(rendered: &mut String, primitive: &wg_store::StoredPrimitive) {
    render_field(
        rendered,
        "status",
        primitive.frontmatter.extra_fields.get("status"),
    );
    render_field(
        rendered,
        "actor_id",
        primitive.frontmatter.extra_fields.get("actor_id"),
    );
    render_field(
        rendered,
        "executor_id",
        primitive.frontmatter.extra_fields.get("executor_id"),
    );
    render_field(
        rendered,
        "thread_id",
        primitive.frontmatter.extra_fields.get("thread_id"),
    );
    render_field(
        rendered,
        "mission_id",
        primitive.frontmatter.extra_fields.get("mission_id"),
    );
    render_field(
        rendered,
        "parent_run_id",
        primitive.frontmatter.extra_fields.get("parent_run_id"),
    );
}

fn render_trigger_sections(rendered: &mut String, primitive: &wg_store::StoredPrimitive) {
    render_field(
        rendered,
        "status",
        primitive.frontmatter.extra_fields.get("status"),
    );
    render_section_list(
        rendered,
        "event_pattern",
        primitive.frontmatter.extra_fields.get("event_pattern"),
    );
    render_section_list(
        rendered,
        "action_plans",
        primitive.frontmatter.extra_fields.get("action_plans"),
    );
}

fn render_generic_fields(rendered: &mut String, primitive: &wg_store::StoredPrimitive) {
    for (key, value) in &primitive.frontmatter.extra_fields {
        let _ = writeln!(rendered, "{key}: {}", yaml_scalar_or_inline(value));
    }
}

fn render_field(rendered: &mut String, key: &str, value: Option<&Value>) {
    if let Some(value) = value {
        let _ = writeln!(rendered, "{key}: {}", yaml_scalar_or_inline(value));
    }
}

fn render_section_list(rendered: &mut String, key: &str, value: Option<&Value>) {
    if let Some(value) = value {
        let _ = writeln!(rendered, "{key}: {}", yaml_scalar_or_inline(value));
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
