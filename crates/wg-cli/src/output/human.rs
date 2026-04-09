//! Human-readable output rendering for CLI command results.

use std::fmt::Write as _;

use serde_yaml::Value;

use super::{
    CapabilitiesOutput, CheckpointOutput, CommandOutput, CreateOutcome, CreateOutput, InitOutput,
    LedgerOutput, QueryOutput, RunCreateOutcome, RunCreateOutput, RunLifecycleOutput, SchemaOutput,
    ShowOutput, StatusOutput, ThreadClaimOutput, ThreadCompleteOutput,
};

/// Renders a structured command output to human-readable text.
#[must_use]
pub fn render(output: &CommandOutput, next_actions: &[String]) -> String {
    let mut rendered = match output {
        CommandOutput::Init(output) => render_init(output),
        CommandOutput::Brief(output) => render_brief(output),
        CommandOutput::Status(output) => render_status(output),
        CommandOutput::Claim(output) => render_claim(output),
        CommandOutput::Complete(output) => render_complete(output),
        CommandOutput::Checkpoint(output) => render_checkpoint(output),
        CommandOutput::Ledger(output) => render_ledger(output),
        CommandOutput::Capabilities(output) => render_capabilities(output),
        CommandOutput::Schema(output) => render_schema(output),
        CommandOutput::Create(output) => render_create(output),
        CommandOutput::RunCreate(output) => render_run_create(output),
        CommandOutput::RunLifecycle(output) => render_run_lifecycle(output),
        CommandOutput::Query(output) => render_query(output),
        CommandOutput::Show(output) => render_show(output),
    };
    if !next_actions.is_empty() {
        rendered.push_str("\n\nNext actions:");
        for action in next_actions {
            rendered.push_str("\n- ");
            rendered.push_str(action);
        }
    }
    rendered
}

/// Renders a structured command failure to human-readable text.
#[must_use]
pub fn render_failure(command: Option<&str>, error: &anyhow::Error, fix: &str) -> String {
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
    let _ = writeln!(rendered, "Fix:");
    let _ = writeln!(rendered, "- {fix}");
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

fn render_brief(output: &super::BriefOutput) -> String {
    let mut rendered = String::new();
    let orientation = &output.orientation;
    let _ = writeln!(
        rendered,
        "Workspace brief [{}]: {} ({})",
        orientation.lens.as_str(),
        output.workspace.name,
        output.workspace.id
    );
    let _ = writeln!(rendered, "Root: {}", output.workspace.root);
    match &output.workspace.default_actor_id {
        Some(default_actor) => {
            let _ = writeln!(rendered, "Default actor: {default_actor}");
        }
        None => {
            let _ = writeln!(rendered, "Default actor: none");
        }
    }
    let _ = writeln!(rendered, "Key counts:");
    for (primitive_type, count) in &output.primitive_counts {
        let _ = writeln!(rendered, "- {primitive_type}: {count}");
    }
    for section in &orientation.sections {
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
    if output.recent_ledger_entries.is_empty() {
        let _ = writeln!(rendered, "- none");
    } else {
        for entry in &output.recent_ledger_entries {
            let _ = writeln!(
                rendered,
                "- {} {}/{} {:?}",
                entry.ts.to_rfc3339(),
                entry.primitive_type,
                entry.primitive_id,
                entry.op
            );
        }
    }

    if !orientation.warnings.is_empty() {
        let _ = writeln!(rendered, "Warnings:");
        for warning in &orientation.warnings {
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
    let _ = writeln!(rendered, "Orphan nodes:");
    if output.orphan_nodes.is_empty() {
        let _ = writeln!(rendered, "- none");
    } else {
        for orphan in &output.orphan_nodes {
            let _ = writeln!(rendered, "- {}", orphan.reference);
        }
    }

    rendered.trim_end().to_owned()
}

fn render_claim(output: &ThreadClaimOutput) -> String {
    let mut rendered = String::new();
    let _ = writeln!(rendered, "Claimed thread: {}", output.thread.id);
    let _ = writeln!(rendered, "title: {}", output.thread.title);
    let _ = writeln!(rendered, "status: {}", output.thread.status.as_str());
    let _ = writeln!(
        rendered,
        "assigned_actor: {}",
        output
            .thread
            .assigned_actor
            .as_ref()
            .map_or("none", |actor| actor.as_str())
    );
    rendered.trim_end().to_owned()
}

fn render_complete(output: &ThreadCompleteOutput) -> String {
    let mut rendered = String::new();
    let _ = writeln!(rendered, "Completed thread: {}", output.thread.id);
    let _ = writeln!(rendered, "title: {}", output.thread.title);
    let _ = writeln!(rendered, "status: {}", output.thread.status.as_str());
    rendered.trim_end().to_owned()
}

fn render_checkpoint(output: &CheckpointOutput) -> String {
    let mut rendered = String::new();
    let _ = writeln!(
        rendered,
        "Saved checkpoint: {}/{}",
        output.primitive.frontmatter.r#type, output.primitive.frontmatter.id
    );
    let _ = writeln!(rendered, "title: {}", output.primitive.frontmatter.title);
    rendered.trim_end().to_owned()
}

fn render_ledger(output: &LedgerOutput) -> String {
    let mut rendered = String::new();
    let _ = writeln!(rendered, "Ledger entries: {}", output.count);
    if output.entries.is_empty() {
        let _ = writeln!(rendered, "- none");
    } else {
        for entry in &output.entries {
            let _ = writeln!(
                rendered,
                "- {} {} {}/{} {:?}",
                entry.ts.to_rfc3339(),
                entry.actor,
                entry.primitive_type,
                entry.primitive_id,
                entry.op
            );
        }
    }
    rendered.trim_end().to_owned()
}

fn render_create(output: &CreateOutput) -> String {
    let mut rendered = String::new();
    let action = match output.outcome {
        CreateOutcome::Created => "Created",
        CreateOutcome::Noop => "No-op (already exists)",
        CreateOutcome::DryRun => "Dry run preview",
    };
    let _ = writeln!(rendered, "{action}: {}", output.reference);
    let _ = writeln!(rendered, "Path: {}", output.path);
    if let Some(ledger_entry) = &output.ledger_entry {
        let _ = writeln!(rendered, "Ledger hash: {}", ledger_entry.hash);
    } else {
        let _ = writeln!(rendered, "Ledger hash: n/a");
    }
    rendered.trim_end().to_owned()
}

fn render_run_create(output: &RunCreateOutput) -> String {
    let mut rendered = String::new();
    let action = match output.outcome {
        RunCreateOutcome::Created => "Created",
        RunCreateOutcome::Noop => "No-op (already exists)",
        RunCreateOutcome::DryRun => "Dry run preview",
    };
    let _ = writeln!(rendered, "{action} run: {}", output.reference);
    let _ = writeln!(rendered, "title: {}", output.run.title);
    let _ = writeln!(rendered, "status: {}", output.run.status.as_str());
    let _ = writeln!(rendered, "thread_id: {}", output.run.thread_id);
    let _ = writeln!(rendered, "actor_id: {}", output.run.actor_id);
    let _ = writeln!(rendered, "Path: {}", output.path);
    if let Some(ledger_entry) = &output.ledger_entry {
        let _ = writeln!(rendered, "Ledger hash: {}", ledger_entry.hash);
    } else {
        let _ = writeln!(rendered, "Ledger hash: n/a");
    }
    rendered.trim_end().to_owned()
}

fn render_run_lifecycle(output: &RunLifecycleOutput) -> String {
    let mut rendered = String::new();
    let _ = writeln!(rendered, "{} run: {}", output.action, output.run.id);
    let _ = writeln!(rendered, "title: {}", output.run.title);
    let _ = writeln!(rendered, "status: {}", output.run.status.as_str());
    let _ = writeln!(rendered, "thread_id: {}", output.run.thread_id);
    let _ = writeln!(rendered, "actor_id: {}", output.run.actor_id);
    if let Some(summary) = &output.run.summary {
        let _ = writeln!(rendered, "summary: {summary}");
    }
    rendered.trim_end().to_owned()
}

fn render_capabilities(output: &CapabilitiesOutput) -> String {
    let mut rendered = String::new();
    let _ = writeln!(rendered, "WorkGraph CLI capabilities");
    let _ = writeln!(rendered, "First command: {}", output.first_command);
    let _ = writeln!(rendered, "Commands:");
    for command in &output.commands {
        let _ = writeln!(rendered, "- {} — {}", command.name, command.description);
        if !command.required_args.is_empty() {
            let _ = writeln!(
                rendered,
                "  required args: {}",
                command.required_args.join(", ")
            );
        }
        if !command.flags.is_empty() {
            let _ = writeln!(rendered, "  flags: {}", command.flags.join(", "));
        }
        for example in &command.examples {
            let _ = writeln!(rendered, "  example: {example}");
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
    let _ = writeln!(rendered, "Primitive types:");
    for primitive_type in &output.primitive_types {
        let _ = writeln!(
            rendered,
            "- {} ({}) — {}",
            primitive_type.name, primitive_type.directory, primitive_type.description
        );
        for field in &primitive_type.fields {
            let _ = writeln!(
                rendered,
                "  • {} ({}){} — {}",
                field.name,
                field.field_type,
                if field.required { ", required" } else { "" },
                field.description
            );
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
    if output.items.is_empty() {
        let _ = writeln!(rendered, "- none");
    } else {
        for item in &output.items {
            let _ = writeln!(
                rendered,
                "- {}/{} — {}",
                item.frontmatter.r#type, item.frontmatter.id, item.frontmatter.title
            );
        }
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
