//! Implementation of the `workgraph trigger` command family.

use anyhow::{Context, bail};
use wg_trigger::{Trigger, evaluate_ledger_entry, save_trigger};
use wg_types::{EventPattern, EventSourceKind, LedgerOp, TriggerActionPlan, TriggerStatus};

use crate::app::AppContext;
use crate::args::TriggerCommand;
use crate::output::{TriggerMatchOutput, TriggerOutput};
use crate::services::codec::trigger_to_stored;

/// Executes a trigger workflow command.
///
/// # Errors
///
/// Returns an error when parsing, persistence, or evaluation fails.
pub async fn handle(app: &AppContext, command: TriggerCommand) -> anyhow::Result<TriggerOutput> {
    match command {
        TriggerCommand::Save {
            id,
            title,
            status,
            event_source,
            event_name,
            ops,
            primitive_types,
            primitive_id,
            field_names,
            provider,
            action_kind,
            action_target,
            action_instruction,
        } => {
            let trigger = Trigger {
                id,
                title,
                status: parse_trigger_status(&status)?,
                event_pattern: EventPattern {
                    source: parse_event_source(&event_source)?,
                    event_name,
                    ops: parse_ledger_ops(&ops)?,
                    primitive_types,
                    primitive_id,
                    field_names,
                    provider,
                },
                action_plans: vec![TriggerActionPlan {
                    kind: action_kind,
                    target_reference: action_target,
                    instruction: action_instruction,
                }],
            };
            let reference = format!("trigger/{}", trigger.id);
            let primitive = trigger_to_stored(&trigger)?;

            if app.dry_run() {
                return Ok(TriggerOutput {
                    action: "save".to_owned(),
                    dry_run: true,
                    reference: Some(reference),
                    trigger: Some(primitive),
                    evaluated_entry: None,
                    matches: Vec::new(),
                });
            }

            save_trigger(app.workspace(), &trigger)
                .await
                .with_context(|| format!("failed to save trigger '{}'", trigger.id))?;

            Ok(TriggerOutput {
                action: "save".to_owned(),
                dry_run: false,
                reference: Some(reference),
                trigger: Some(primitive),
                evaluated_entry: None,
                matches: Vec::new(),
            })
        }
        TriggerCommand::Evaluate { entry_index } => {
            let entries = app.read_ledger_entries().await?;
            let entry = entries.get(entry_index).cloned().ok_or_else(|| {
                anyhow::anyhow!("ledger entry index {entry_index} does not exist")
            })?;
            let matches = evaluate_ledger_entry(app.workspace(), &entry)
                .await
                .context("failed to evaluate triggers for ledger entry")?;

            Ok(TriggerOutput {
                action: "evaluate".to_owned(),
                dry_run: false,
                reference: None,
                trigger: None,
                evaluated_entry: Some(entry),
                matches: matches.into_iter().map(to_trigger_match_output).collect(),
            })
        }
    }
}

fn parse_trigger_status(input: &str) -> anyhow::Result<TriggerStatus> {
    match input {
        "draft" => Ok(TriggerStatus::Draft),
        "active" => Ok(TriggerStatus::Active),
        "paused" => Ok(TriggerStatus::Paused),
        "disabled" => Ok(TriggerStatus::Disabled),
        _ => bail!("unsupported trigger status '{input}'"),
    }
}

fn parse_event_source(input: &str) -> anyhow::Result<EventSourceKind> {
    match input {
        "ledger" => Ok(EventSourceKind::Ledger),
        "webhook" => Ok(EventSourceKind::Webhook),
        "internal" => Ok(EventSourceKind::Internal),
        _ => bail!("unsupported event source '{input}'"),
    }
}

fn parse_ledger_ops(values: &[String]) -> anyhow::Result<Vec<LedgerOp>> {
    values
        .iter()
        .map(|value| match value.as_str() {
            "create" => Ok(LedgerOp::Create),
            "update" => Ok(LedgerOp::Update),
            "delete" => Ok(LedgerOp::Delete),
            "claim" => Ok(LedgerOp::Claim),
            "release" => Ok(LedgerOp::Release),
            "start" => Ok(LedgerOp::Start),
            "done" => Ok(LedgerOp::Done),
            "cancel" => Ok(LedgerOp::Cancel),
            "reopen" => Ok(LedgerOp::Reopen),
            "assign" => Ok(LedgerOp::Assign),
            "unassign" => Ok(LedgerOp::Unassign),
            _ => bail!("unsupported ledger op '{value}'"),
        })
        .collect()
}

fn to_trigger_match_output(matched: wg_trigger::MatchedTrigger) -> TriggerMatchOutput {
    TriggerMatchOutput {
        trigger_id: matched.trigger_id,
        title: matched.title,
        action_plans: matched.action_plans,
    }
}
