//! Human-readable output rendering for CLI command results.

use std::fmt::Write as _;

use serde_yaml::Value;

use super::{
    ActorListOutput, ActorRegisterOutput, ActorShowOutput, CapabilitiesOutput, CheckpointOutput,
    CommandOutput, ConnectOutput, CreateOutcome, CreateOutput, GraphReferenceOutput, InitOutput,
    LedgerOutput, QueryOutput, RunCreateOutcome, RunCreateOutput, RunLifecycleOutput, SchemaOutput,
    ShowOutput, StatusOutput, ThreadClaimOutput, ThreadCompleteOutput, TriggerIngestOutput,
    TriggerReplayOutput, TriggerValidateOutput, WhoamiOutput,
};

/// Renders a structured command output to human-readable text.
#[must_use]
pub fn render(output: &CommandOutput, next_actions: &[String]) -> String {
    let mut rendered = match output {
        CommandOutput::Init(output) => render_init(output),
        CommandOutput::Connect(output) => render_connect(output),
        CommandOutput::Whoami(output) => render_whoami(output),
        CommandOutput::Serve(output) => render_serve(output),
        CommandOutput::Brief(output) => render_brief(output),
        CommandOutput::Status(output) => render_status(output),
        CommandOutput::Claim(output) => render_claim(output),
        CommandOutput::Complete(output) => render_complete(output),
        CommandOutput::Checkpoint(output) => render_checkpoint(output),
        CommandOutput::Ledger(output) => render_ledger(output),
        CommandOutput::Capabilities(output) => render_capabilities(output),
        CommandOutput::Schema(output) => render_schema(output),
        CommandOutput::ActorRegister(output) => render_actor_register(output),
        CommandOutput::ActorList(output) => render_actor_list(output),
        CommandOutput::ActorShow(output) => render_actor_show(output),
        CommandOutput::Create(output) => render_create(output),
        CommandOutput::RunCreate(output) => render_run_create(output),
        CommandOutput::RunLifecycle(output) => render_run_lifecycle(output),
        CommandOutput::TriggerValidate(output) => render_trigger_validate(output),
        CommandOutput::TriggerReplay(output) => render_trigger_replay(output),
        CommandOutput::TriggerIngest(output) => render_trigger_ingest(output),
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

fn render_connect(output: &ConnectOutput) -> String {
    let mut rendered = String::new();
    let _ = writeln!(rendered, "Connected workspace to hosted WorkGraph");
    let _ = writeln!(
        rendered,
        "mode: {}",
        if output.config.remote.is_some() {
            "hosted"
        } else {
            "local"
        }
    );
    if let Some(remote) = &output.config.remote {
        let _ = writeln!(rendered, "server: {}", remote.server_url);
        let _ = writeln!(rendered, "actor_id: {}", remote.actor_id);
        let _ = writeln!(rendered, "access_scope: {}", remote.access_scope);
    }
    rendered.trim_end().to_owned()
}

fn render_whoami(output: &WhoamiOutput) -> String {
    let mut rendered = String::new();
    let _ = writeln!(rendered, "mode: {}", output.mode);
    let _ = writeln!(rendered, "actor_id: {}", output.actor_id);
    if let Some(access_scope) = &output.access_scope {
        let _ = writeln!(rendered, "access_scope: {access_scope}");
    }
    let _ = writeln!(rendered, "workspace_id: {}", output.workspace_id);
    let _ = writeln!(rendered, "workspace_name: {}", output.workspace_name);
    if let Some(server_url) = &output.hosted_server {
        let _ = writeln!(rendered, "server_url: {server_url}");
    }
    if let Some(profile) = &output.hosted_profile {
        let _ = writeln!(rendered, "profile: {profile}");
    }
    rendered.trim_end().to_owned()
}

fn render_serve(output: &super::ServeOutput) -> String {
    let mut rendered = String::new();
    let _ = writeln!(rendered, "Serving WorkGraph over {}", output.transport);
    if let Some(endpoint) = &output.endpoint {
        let _ = writeln!(rendered, "endpoint: {endpoint}");
    }
    let _ = writeln!(rendered, "workspace_root: {}", output.workspace_root);
    if let Some(actor_id) = &output.actor_id {
        let _ = writeln!(rendered, "actor_id: {actor_id}");
    }
    let _ = writeln!(rendered, "access_scope: {}", output.access_scope);
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

    let _ = writeln!(rendered, "Trigger health:");
    if output.trigger_health.is_empty() {
        let _ = writeln!(rendered, "- none");
    } else {
        for trigger in &output.trigger_health {
            let _ = writeln!(
                rendered,
                "- {} [{}] last_event={} last_receipt={}",
                trigger.trigger_reference,
                trigger.status,
                trigger.last_event_id.as_deref().unwrap_or("none"),
                trigger.last_receipt_id.as_deref().unwrap_or("none")
            );
        }
    }

    let _ = writeln!(rendered, "Recent trigger receipts:");
    if output.recent_trigger_receipts.is_empty() {
        let _ = writeln!(rendered, "- none");
    } else {
        for receipt in &output.recent_trigger_receipts {
            let _ = writeln!(
                rendered,
                "- {} trigger={} source={} pending={}",
                receipt.receipt_reference,
                receipt.trigger_reference,
                receipt.event_source,
                receipt.pending_plans
            );
        }
    }

    let _ = writeln!(
        rendered,
        "Pending trigger actions: {}",
        output.pending_trigger_actions
    );

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

fn render_actor_register(output: &ActorRegisterOutput) -> String {
    let mut rendered = String::new();
    let _ = writeln!(rendered, "Registered actor: {}", output.reference);
    let _ = writeln!(rendered, "title: {}", output.primitive.frontmatter.title);
    if let Some(ledger_entry) = &output.ledger_entry {
        let _ = writeln!(rendered, "ledger_hash: {}", ledger_entry.hash);
    }
    rendered.trim_end().to_owned()
}

fn render_actor_list(output: &ActorListOutput) -> String {
    let mut rendered = String::new();
    let _ = writeln!(rendered, "Registered actors: {}", output.count);
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

fn render_actor_show(output: &ActorShowOutput) -> String {
    render_show(&ShowOutput {
        reference: output.reference.clone(),
        primitive: output.primitive.clone(),
        outbound_references: Vec::new(),
        inbound_references: Vec::new(),
        broken_references: Vec::new(),
    })
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

fn render_trigger_validate(output: &TriggerValidateOutput) -> String {
    let mut rendered = String::new();
    let _ = writeln!(rendered, "Validated trigger: {}", output.reference);
    let _ = writeln!(
        rendered,
        "matches source: {}",
        output.trigger.event_pattern.source.as_str()
    );
    let _ = writeln!(
        rendered,
        "action plans: {}",
        output.trigger.action_plans.len()
    );
    rendered.trim_end().to_owned()
}

fn render_trigger_replay(output: &TriggerReplayOutput) -> String {
    let mut rendered = String::new();
    let emitted_receipts = output
        .results
        .iter()
        .map(|result| result.receipts.len())
        .sum::<usize>();
    let _ = writeln!(
        rendered,
        "Replayed ledger events: {}",
        output.events_replayed
    );
    let _ = writeln!(rendered, "Receipts emitted: {}", emitted_receipts);
    if output.results.is_empty() {
        let _ = writeln!(rendered, "- none");
    } else {
        for result in &output.results {
            let _ = writeln!(
                rendered,
                "- event {} [{}] receipts={}",
                result.event.id,
                result.event.source.as_str(),
                result.receipts.len()
            );
            for receipt in &result.receipts {
                let _ = writeln!(
                    rendered,
                    "  • trigger_receipt/{} — trigger={} pending={}",
                    receipt.id,
                    receipt.trigger_id,
                    receipt
                        .action_outcomes
                        .iter()
                        .filter(|outcome| matches!(
                            outcome.decision,
                            wg_types::TriggerPlanDecision::Allow
                        ))
                        .count()
                );
            }
        }
    }
    rendered.trim_end().to_owned()
}

fn render_trigger_ingest(output: &TriggerIngestOutput) -> String {
    let mut rendered = String::new();
    let _ = writeln!(
        rendered,
        "Ingested trigger event: {} [{}]",
        output.event.id,
        output.event.source.as_str()
    );
    let _ = writeln!(rendered, "Receipts emitted: {}", output.receipts.len());
    if output.receipts.is_empty() {
        let _ = writeln!(rendered, "- none");
    } else {
        for receipt in &output.receipts {
            let _ = writeln!(
                rendered,
                "- trigger_receipt/{} — trigger={} pending={}",
                receipt.id,
                receipt.trigger_id,
                receipt
                    .action_outcomes
                    .iter()
                    .filter(|outcome| matches!(
                        outcome.decision,
                        wg_types::TriggerPlanDecision::Allow
                    ))
                    .count()
            );
        }
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
    if !output.applied_filters.is_empty() {
        let _ = writeln!(rendered, "Filters:");
        for filter in &output.applied_filters {
            let _ = writeln!(rendered, "- {filter}");
        }
    }
    if !output.summary_fields.is_empty() {
        let _ = writeln!(
            rendered,
            "Summary fields: {}",
            output.summary_fields.join(", ")
        );
    }
    if output.items.is_empty() {
        let _ = writeln!(rendered, "- none");
    } else {
        for item in &output.items {
            let mut line = format!(
                "- {}/{} — {}",
                item.frontmatter.r#type, item.frontmatter.id, item.frontmatter.title
            );
            if let Some(summary) = summarize_company_context(item) {
                line.push_str(&format!(" [{summary}]"));
            }
            let _ = writeln!(rendered, "{line}");
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
        "org" | "team" | "person" | "agent" | "client" | "project" => {
            render_company_context_sections(&mut rendered, primitive)
        }
        "thread" => render_thread_sections(&mut rendered, primitive),
        "mission" => render_mission_sections(&mut rendered, primitive),
        "run" => render_run_sections(&mut rendered, primitive),
        "trigger" => render_trigger_sections(&mut rendered, primitive),
        "trigger_receipt" => render_trigger_receipt_sections(&mut rendered, primitive),
        _ => render_generic_fields(&mut rendered, primitive),
    }

    render_reference_sections(
        &mut rendered,
        "Outbound references",
        &output.outbound_references,
    );
    render_reference_sections(
        &mut rendered,
        "Inbound references",
        &output.inbound_references,
    );
    render_reference_sections(&mut rendered, "Broken references", &[]);
    if !output.broken_references.is_empty() {
        let _ = writeln!(rendered);
        let _ = writeln!(rendered, "Broken references:");
        for issue in &output.broken_references {
            let _ = writeln!(
                rendered,
                "- {} [{} via {}] ({})",
                issue.target_reference, issue.kind, issue.provenance, issue.reason
            );
        }
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
    render_section_list(
        rendered,
        "subscription_state",
        primitive.frontmatter.extra_fields.get("subscription_state"),
    );
}

fn render_trigger_receipt_sections(rendered: &mut String, primitive: &wg_store::StoredPrimitive) {
    for key in [
        "trigger_id",
        "trigger_title",
        "event_id",
        "event_source",
        "event_name",
        "provider",
        "actor_id",
        "subject_reference",
        "occurred_at",
        "dedup_key",
    ] {
        render_field(rendered, key, primitive.frontmatter.extra_fields.get(key));
    }
    render_section_list(
        rendered,
        "field_names",
        primitive.frontmatter.extra_fields.get("field_names"),
    );
    render_section_list(
        rendered,
        "payload_fields",
        primitive.frontmatter.extra_fields.get("payload_fields"),
    );
    render_section_list(
        rendered,
        "action_outcomes",
        primitive.frontmatter.extra_fields.get("action_outcomes"),
    );
}

fn render_company_context_sections(rendered: &mut String, primitive: &wg_store::StoredPrimitive) {
    match primitive.frontmatter.r#type.as_str() {
        "org" => {
            for key in ["summary"] {
                render_field(rendered, key, primitive.frontmatter.extra_fields.get(key));
            }
            for key in ["tags", "external_refs", "snapshot"] {
                render_section_list(rendered, key, primitive.frontmatter.extra_fields.get(key));
            }
        }
        "team" => {
            for key in ["org_id", "mission"] {
                render_field(rendered, key, primitive.frontmatter.extra_fields.get(key));
            }
            for key in ["members", "tags", "external_refs"] {
                render_section_list(rendered, key, primitive.frontmatter.extra_fields.get(key));
            }
        }
        "person" => {
            for key in ["email", "role"] {
                render_field(rendered, key, primitive.frontmatter.extra_fields.get(key));
            }
            for key in ["team_ids", "tags", "external_refs"] {
                render_section_list(rendered, key, primitive.frontmatter.extra_fields.get(key));
            }
        }
        "agent" => {
            for key in [
                "runtime",
                "description",
                "owner",
                "parent_actor_id",
                "root_actor_id",
                "lineage_mode",
            ] {
                render_field(rendered, key, primitive.frontmatter.extra_fields.get(key));
            }
            for key in ["capabilities", "tags", "external_refs"] {
                render_section_list(rendered, key, primitive.frontmatter.extra_fields.get(key));
            }
        }
        "client" => {
            for key in ["summary", "account_owner"] {
                render_field(rendered, key, primitive.frontmatter.extra_fields.get(key));
            }
            for key in ["tags", "external_refs", "snapshot"] {
                render_section_list(rendered, key, primitive.frontmatter.extra_fields.get(key));
            }
        }
        "project" => {
            for key in ["status", "client_id"] {
                render_field(rendered, key, primitive.frontmatter.extra_fields.get(key));
            }
            for key in ["team_ids", "tags", "external_refs", "snapshot"] {
                render_section_list(rendered, key, primitive.frontmatter.extra_fields.get(key));
            }
        }
        _ => render_generic_fields(rendered, primitive),
    }
}

fn render_reference_sections(
    rendered: &mut String,
    heading: &str,
    references: &[GraphReferenceOutput],
) {
    let _ = writeln!(rendered);
    let _ = writeln!(rendered, "{heading}:");
    if references.is_empty() {
        let _ = writeln!(rendered, "- none");
        return;
    }

    for reference in references {
        let related_reference = if heading.starts_with("Inbound") {
            &reference.source_reference
        } else {
            &reference.target_reference
        };
        let _ = writeln!(
            rendered,
            "- {} [{} via {}]",
            related_reference, reference.kind, reference.provenance,
        );
    }
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

fn summarize_company_context(primitive: &wg_store::StoredPrimitive) -> Option<String> {
    match primitive.frontmatter.r#type.as_str() {
        "person" => Some(join_summary_parts([
            optional_text(primitive.frontmatter.extra_fields.get("role")),
            optional_list_count("teams", primitive.frontmatter.extra_fields.get("team_ids")),
            None,
        ])),
        "team" => Some(join_summary_parts([
            optional_text(primitive.frontmatter.extra_fields.get("org_id")),
            optional_list_count("members", primitive.frontmatter.extra_fields.get("members")),
            None,
        ])),
        "project" => Some(join_summary_parts([
            optional_text(primitive.frontmatter.extra_fields.get("status")),
            optional_text(primitive.frontmatter.extra_fields.get("client_id")),
            optional_list_count("teams", primitive.frontmatter.extra_fields.get("team_ids")),
        ])),
        "client" => Some(join_summary_parts([
            optional_text(primitive.frontmatter.extra_fields.get("account_owner")),
            None,
            None,
        ])),
        "agent" => Some(join_summary_parts([
            optional_text(primitive.frontmatter.extra_fields.get("runtime")),
            optional_list_count(
                "capabilities",
                primitive.frontmatter.extra_fields.get("capabilities"),
            ),
            None,
        ])),
        _ => None,
    }
    .filter(|summary| !summary.is_empty())
}

fn join_summary_parts(parts: [Option<String>; 3]) -> String {
    parts.into_iter().flatten().collect::<Vec<_>>().join(", ")
}

fn optional_text(value: Option<&Value>) -> Option<String> {
    value.and_then(|value| match value {
        Value::String(text) if !text.trim().is_empty() => Some(text.trim().to_owned()),
        Value::Tagged(tagged) => optional_text(Some(&tagged.value)),
        _ => None,
    })
}

fn optional_list_count(label: &str, value: Option<&Value>) -> Option<String> {
    let count = match value {
        Some(Value::Sequence(items)) if !items.is_empty() => Some(items.len()),
        Some(Value::Tagged(tagged)) => return optional_list_count(label, Some(&tagged.value)),
        _ => None,
    }?;
    Some(format!("{count} {label}"))
}
