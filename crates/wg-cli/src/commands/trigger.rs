//! Implementation of the `workgraph trigger` command family.

use std::collections::BTreeMap;

use anyhow::{Context, bail};
use chrono::Utc;
use wg_trigger::{
    event_from_ledger_entry, ingest_event, load_trigger, validate_trigger_definition,
};
use wg_types::{ActorId, EventEnvelope, EventSourceKind};

use crate::app::AppContext;
use crate::args::KeyValueInput;
use crate::output::{
    TriggerIngestOutput, TriggerReplayOutput, TriggerReplayResult, TriggerValidateOutput,
};
use crate::util::workspace::parse_reference;

/// Parsed arguments for `workgraph trigger ingest`.
#[derive(Debug, Clone)]
pub struct TriggerIngestArgs {
    /// Stable source kind used for evaluation.
    pub source: String,
    /// Stable event identifier.
    pub event_id: String,
    /// Optional stable event name.
    pub event_name: Option<String>,
    /// Optional provider or emitter name.
    pub provider: Option<String>,
    /// Optional actor identifier associated with the event.
    pub actor_id: Option<String>,
    /// Optional subject reference in `<type>/<id>` form.
    pub subject_reference: Option<String>,
    /// Optional primitive type for the event subject.
    pub primitive_type: Option<String>,
    /// Optional primitive id for the event subject.
    pub primitive_id: Option<String>,
    /// Optional ledger operation for ledger-style events.
    pub op: Option<String>,
    /// Payload fields observed on the event.
    pub fields: Vec<crate::args::KeyValueInput>,
}

/// Validates a trigger definition already stored in the workspace.
///
/// # Errors
///
/// Returns an error when the trigger cannot be loaded or fails validation.
pub async fn validate(app: &AppContext, trigger_id: &str) -> anyhow::Result<TriggerValidateOutput> {
    let (primitive_type, trigger_id) = parse_reference(trigger_id)?;
    if primitive_type != "trigger" {
        bail!("trigger validation expects a trigger/<id> reference");
    }
    let trigger = load_trigger(app.workspace(), trigger_id)
        .await
        .with_context(|| format!("failed to load trigger '{trigger_id}'"))?;
    validate_trigger_definition(&trigger)
        .with_context(|| format!("failed to validate trigger '{trigger_id}'"))?;
    Ok(TriggerValidateOutput {
        reference: format!("trigger/{trigger_id}"),
        trigger,
        valid: true,
    })
}

/// Replays recent ledger entries through the trigger plane.
///
/// # Errors
///
/// Returns an error when the ledger cannot be read or receipts cannot be persisted.
pub async fn replay(app: &AppContext, last: Option<usize>) -> anyhow::Result<TriggerReplayOutput> {
    let mut entries = app.read_ledger_entries().await?;
    if let Some(limit) = last {
        if entries.len() > limit {
            let start = entries.len().saturating_sub(limit);
            entries = entries.split_off(start);
        }
    }

    let mut results = Vec::with_capacity(entries.len());
    for entry in entries {
        let event = event_from_ledger_entry(&entry);
        let receipts = ingest_event(app.workspace(), &event)
            .await
            .with_context(|| format!("failed to replay ledger event '{}'", event.id))?;
        results.push(TriggerReplayResult { event, receipts });
    }

    Ok(TriggerReplayOutput {
        events_replayed: results.len(),
        results,
    })
}

/// Ingests one normalized event through the trigger plane.
///
/// # Errors
///
/// Returns an error when the event payload is invalid or receipts cannot be persisted.
pub async fn ingest(
    app: &AppContext,
    args: TriggerIngestArgs,
) -> anyhow::Result<TriggerIngestOutput> {
    let event = build_event(args)?;
    let receipts = ingest_event(app.workspace(), &event)
        .await
        .with_context(|| format!("failed to ingest event '{}'", event.id))?;
    Ok(TriggerIngestOutput { event, receipts })
}

fn build_event(args: TriggerIngestArgs) -> anyhow::Result<EventEnvelope> {
    let source = parse_source(&args.source)?;
    let op = args.op.as_deref().map(parse_op).transpose()?;
    if matches!(source, EventSourceKind::Ledger) && op.is_none() {
        bail!("ledger trigger ingest requires --op <create|update|done|...>");
    }

    let payload_fields = args
        .fields
        .iter()
        .map(|field| (field.key.clone(), field.value.clone()))
        .collect::<BTreeMap<_, _>>();
    let primitive_type = args.primitive_type.clone().or_else(|| {
        args.subject_reference.as_deref().and_then(|reference| {
            reference
                .split_once('/')
                .map(|(primitive_type, _)| primitive_type.to_owned())
        })
    });
    let primitive_id = args.primitive_id.clone().or_else(|| {
        args.subject_reference.as_deref().and_then(|reference| {
            reference
                .split_once('/')
                .map(|(_, primitive_id)| primitive_id.to_owned())
        })
    });

    Ok(EventEnvelope {
        id: args.event_id,
        source,
        event_name: args.event_name,
        provider: args.provider,
        actor_id: args.actor_id.map(ActorId::new),
        occurred_at: Utc::now(),
        op,
        primitive_type,
        primitive_id,
        subject_reference: args.subject_reference,
        field_names: payload_fields.keys().cloned().collect(),
        payload_fields,
    })
}

pub(crate) fn field_value(fields: &[KeyValueInput], key: &str) -> Option<String> {
    fields
        .iter()
        .find(|field| field.key == key)
        .map(|field| field.value.clone())
}

fn parse_source(input: &str) -> anyhow::Result<EventSourceKind> {
    match input {
        "ledger" => Ok(EventSourceKind::Ledger),
        "webhook" => Ok(EventSourceKind::Webhook),
        "internal" => Ok(EventSourceKind::Internal),
        other => bail!("unsupported event source '{other}'"),
    }
}

fn parse_op(input: &str) -> anyhow::Result<wg_types::LedgerOp> {
    match input {
        "create" => Ok(wg_types::LedgerOp::Create),
        "update" => Ok(wg_types::LedgerOp::Update),
        "delete" => Ok(wg_types::LedgerOp::Delete),
        "claim" => Ok(wg_types::LedgerOp::Claim),
        "release" => Ok(wg_types::LedgerOp::Release),
        "start" => Ok(wg_types::LedgerOp::Start),
        "done" => Ok(wg_types::LedgerOp::Done),
        "cancel" => Ok(wg_types::LedgerOp::Cancel),
        "reopen" => Ok(wg_types::LedgerOp::Reopen),
        "assign" => Ok(wg_types::LedgerOp::Assign),
        "unassign" => Ok(wg_types::LedgerOp::Unassign),
        other => bail!("unsupported ledger op '{other}'"),
    }
}

#[cfg(test)]
mod tests {
    use super::{TriggerIngestArgs, build_event};
    use crate::args::KeyValueInput;

    #[test]
    fn build_event_infers_subject_parts() {
        let event = build_event(TriggerIngestArgs {
            source: "internal".to_owned(),
            event_id: "event-1".to_owned(),
            event_name: Some("signal.sent".to_owned()),
            provider: Some("signal-bus".to_owned()),
            actor_id: Some("agent:cursor".to_owned()),
            subject_reference: Some("thread/thread-1".to_owned()),
            primitive_type: None,
            primitive_id: None,
            op: None,
            fields: vec![KeyValueInput {
                key: "status".to_owned(),
                value: "active".to_owned(),
            }],
        })
        .expect("event should build");

        assert_eq!(event.primitive_type.as_deref(), Some("thread"));
        assert_eq!(event.primitive_id.as_deref(), Some("thread-1"));
        assert_eq!(
            event.payload_fields.get("status").map(String::as_str),
            Some("active")
        );
    }
}
