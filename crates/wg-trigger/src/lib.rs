#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Durable trigger parsing, validation, and event-plane matching for WorkGraph.

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use serde_yaml::Value;
use sha2::{Digest, Sha256};
use wg_error::{Result, WorkgraphError};
use wg_paths::WorkspacePath;
use wg_policy::{PolicyAction, PolicyContext, PolicyDecision, evaluate as evaluate_policy};
use wg_store::{
    AuditedWriteRequest, PrimitiveFrontmatter, StoredPrimitive, list_primitives, read_primitive,
    write_primitive_audited_now,
};
use wg_types::{
    ActorId, EventEnvelope, EventPattern, EventSourceKind, LedgerEntry, Registry,
    TriggerActionOutcome, TriggerPlanDecision, TriggerPrimitive, TriggerReceiptPrimitive,
    TriggerStatus,
};

mod mutation;

const TRIGGER_TYPE: &str = "trigger";
const TRIGGER_RECEIPT_TYPE: &str = "trigger_receipt";
const TRIGGER_ACTION_PLAN_TYPE: &str = "trigger_action_plan";

/// Typed trigger model persisted by this crate.
pub type Trigger = TriggerPrimitive;
/// Typed trigger receipt model persisted by this crate.
pub type TriggerReceipt = TriggerReceiptPrimitive;

pub use mutation::TriggerMutationService;

/// Result of matching a trigger against a concrete event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchedTrigger {
    /// Matched trigger identifier.
    pub trigger_id: String,
    /// Matched trigger title.
    pub title: String,
    /// Trigger event that matched the rule.
    pub event: EventEnvelope,
    /// Replay-safe trigger/event deduplication key.
    pub dedup_key: String,
    /// Action outcomes emitted by the trigger.
    pub action_outcomes: Vec<TriggerActionOutcome>,
}

/// Persists a trigger after validating its contract.
///
/// # Errors
///
/// Returns an error when validation or persistence fails.
pub async fn save_trigger(workspace: &WorkspacePath, trigger: &Trigger) -> Result<()> {
    TriggerMutationService::new(workspace)
        .save_trigger(trigger)
        .await
}

/// Loads a persisted trigger by identifier.
///
/// # Errors
///
/// Returns an error when the trigger cannot be loaded or decoded.
pub async fn load_trigger(workspace: &WorkspacePath, trigger_id: &str) -> Result<Trigger> {
    let primitive = read_primitive(workspace, TRIGGER_TYPE, trigger_id).await?;
    trigger_from_primitive(&primitive)
}

/// Lists all persisted triggers.
///
/// # Errors
///
/// Returns an error when trigger primitives cannot be loaded or decoded.
pub async fn list_triggers(workspace: &WorkspacePath) -> Result<Vec<Trigger>> {
    list_primitives(workspace, TRIGGER_TYPE)
        .await?
        .iter()
        .map(trigger_from_primitive)
        .collect()
}

/// Loads a persisted trigger receipt by identifier.
///
/// # Errors
///
/// Returns an error when the trigger receipt cannot be loaded or decoded.
pub async fn load_trigger_receipt(
    workspace: &WorkspacePath,
    receipt_id: &str,
) -> Result<TriggerReceipt> {
    let primitive = read_primitive(workspace, TRIGGER_RECEIPT_TYPE, receipt_id).await?;
    trigger_receipt_from_primitive(&primitive)
}

/// Lists all persisted trigger receipts.
///
/// # Errors
///
/// Returns an error when trigger receipt primitives cannot be loaded or decoded.
pub async fn list_trigger_receipts(workspace: &WorkspacePath) -> Result<Vec<TriggerReceipt>> {
    list_primitives(workspace, TRIGGER_RECEIPT_TYPE)
        .await?
        .iter()
        .map(trigger_receipt_from_primitive)
        .collect()
}

/// Ingests one normalized event, evaluates matching triggers, and persists durable receipts.
///
/// # Errors
///
/// Returns an error when evaluation or receipt persistence fails.
pub async fn ingest_event(
    workspace: &WorkspacePath,
    event: &EventEnvelope,
) -> Result<Vec<TriggerReceipt>> {
    TriggerMutationService::new(workspace)
        .ingest_event(event)
        .await
}

/// Ingests one ledger entry into the trigger plane, persisting any resulting receipts.
///
/// # Errors
///
/// Returns an error when evaluation or receipt persistence fails.
pub async fn ingest_ledger_entry(
    workspace: &WorkspacePath,
    entry: &LedgerEntry,
) -> Result<Vec<TriggerReceipt>> {
    ingest_event(workspace, &event_from_ledger_entry(entry)).await
}

/// Validates a trigger definition without persisting it.
///
/// # Errors
///
/// Returns a validation error when the trigger contract is incomplete.
pub fn validate_trigger_definition(trigger: &Trigger) -> Result<()> {
    if trigger.id.trim().is_empty() {
        return Err(WorkgraphError::ValidationError(
            "trigger id must not be empty".to_owned(),
        ));
    }
    if trigger.title.trim().is_empty() {
        return Err(WorkgraphError::ValidationError(
            "trigger title must not be empty".to_owned(),
        ));
    }
    if trigger.action_plans.is_empty() {
        return Err(WorkgraphError::ValidationError(format!(
            "trigger '{}' must include at least one action plan",
            trigger.id
        )));
    }
    validate_event_pattern(&trigger.id, &trigger.event_pattern)
}

/// Converts a ledger entry into the normalized event envelope used by the trigger plane.
#[must_use]
pub fn event_from_ledger_entry(entry: &LedgerEntry) -> EventEnvelope {
    let reference = format!("{}/{}", entry.primitive_type, entry.primitive_id);
    let event_name = format!(
        "{}.{}",
        entry.primitive_type,
        format!("{:?}", entry.op).to_lowercase()
    );
    EventEnvelope {
        id: entry.hash.clone(),
        source: EventSourceKind::Ledger,
        event_name: Some(event_name),
        provider: None,
        actor_id: Some(entry.actor.clone()),
        occurred_at: entry.ts,
        op: Some(entry.op),
        primitive_type: Some(entry.primitive_type.clone()),
        primitive_id: Some(entry.primitive_id.clone()),
        subject_reference: Some(reference),
        field_names: entry.fields_changed.clone(),
        payload_fields: BTreeMap::from([
            ("op".to_owned(), format!("{:?}", entry.op).to_lowercase()),
            ("primitive_type".to_owned(), entry.primitive_type.clone()),
            ("primitive_id".to_owned(), entry.primitive_id.clone()),
        ]),
    }
}

/// Evaluates all active triggers against a normalized event envelope.
///
/// # Errors
///
/// Returns an error when persisted triggers cannot be loaded or decoded.
pub async fn evaluate_event(
    workspace: &WorkspacePath,
    event: &EventEnvelope,
) -> Result<Vec<MatchedTrigger>> {
    let triggers = list_triggers(workspace).await?;
    let mut matches = Vec::new();
    for trigger in triggers {
        if trigger.status != TriggerStatus::Active {
            continue;
        }
        if !event_pattern_matches(&trigger.event_pattern, event) {
            continue;
        }
        matches.push(MatchedTrigger {
            trigger_id: trigger.id.clone(),
            title: trigger.title.clone(),
            event: event.clone(),
            dedup_key: dedup_key(&trigger.id, &event.id),
            action_outcomes: build_action_outcomes(workspace, &trigger, event).await?,
        });
    }
    Ok(matches)
}

/// Evaluates all active triggers against a ledger entry.
///
/// # Errors
///
/// Returns an error when persisted triggers cannot be loaded or decoded.
pub async fn evaluate_ledger_entry(
    workspace: &WorkspacePath,
    entry: &LedgerEntry,
) -> Result<Vec<MatchedTrigger>> {
    evaluate_event(workspace, &event_from_ledger_entry(entry)).await
}

async fn build_action_outcomes(
    workspace: &WorkspacePath,
    trigger: &Trigger,
    event: &EventEnvelope,
) -> Result<Vec<TriggerActionOutcome>> {
    let planning_actor = event.actor_id.clone().unwrap_or_else(system_actor);
    let mut action_outcomes = Vec::with_capacity(trigger.action_plans.len());
    for plan in &trigger.action_plans {
        let context = action_policy_context(trigger, event, plan);
        let decision = evaluate_policy(
            workspace,
            &planning_actor,
            PolicyAction::Create,
            TRIGGER_ACTION_PLAN_TYPE,
            &context,
        )
        .await?;
        let (decision, reason) = match decision {
            PolicyDecision::Allow => (TriggerPlanDecision::Allow, None),
            PolicyDecision::Deny => (
                TriggerPlanDecision::Deny,
                Some(format!(
                    "policy denied create of {TRIGGER_ACTION_PLAN_TYPE} for actor '{}'",
                    planning_actor
                )),
            ),
        };
        action_outcomes.push(TriggerActionOutcome {
            plan: plan.clone(),
            decision,
            reason,
        });
    }
    Ok(action_outcomes)
}

fn action_policy_context(
    trigger: &Trigger,
    event: &EventEnvelope,
    plan: &wg_types::TriggerActionPlan,
) -> PolicyContext {
    let mut fields = BTreeMap::new();
    fields.insert("trigger_id".to_owned(), Value::String(trigger.id.clone()));
    fields.insert(
        "event_source".to_owned(),
        Value::String(event.source.as_str().to_owned()),
    );
    if let Some(event_name) = &event.event_name {
        fields.insert("event_name".to_owned(), Value::String(event_name.clone()));
    }
    fields.insert("action_kind".to_owned(), Value::String(plan.kind.clone()));
    if let Some(target_reference) = &plan.target_reference {
        fields.insert(
            "target_reference".to_owned(),
            Value::String(target_reference.clone()),
        );
    }
    PolicyContext { fields }
}

async fn save_trigger_with_audit(
    workspace: &WorkspacePath,
    trigger: &Trigger,
    audit: AuditedWriteRequest,
) -> Result<LedgerEntry> {
    let primitive = trigger_to_primitive(trigger)?;
    let (_, ledger_entry) =
        write_primitive_audited_now(workspace, &Registry::builtins(), &primitive, audit).await?;
    Ok(ledger_entry)
}

async fn save_trigger_receipt_with_audit(
    workspace: &WorkspacePath,
    receipt: &TriggerReceipt,
    audit: AuditedWriteRequest,
) -> Result<LedgerEntry> {
    let primitive = trigger_receipt_to_primitive(receipt)?;
    let (_, ledger_entry) =
        write_primitive_audited_now(workspace, &Registry::builtins(), &primitive, audit).await?;
    Ok(ledger_entry)
}

fn validate_event_pattern(trigger_id: &str, pattern: &EventPattern) -> Result<()> {
    match pattern.source {
        EventSourceKind::Ledger => {
            if pattern.provider.is_some() {
                return Err(WorkgraphError::ValidationError(format!(
                    "trigger '{trigger_id}' ledger patterns may not set provider"
                )));
            }
            if pattern.event_name.is_none()
                && pattern.ops.is_empty()
                && pattern.primitive_types.is_empty()
                && pattern.primitive_id.is_none()
                && pattern.field_names.is_empty()
                && pattern.actor_id.is_none()
                && pattern.subject_reference.is_none()
                && pattern.payload_fields.is_empty()
            {
                return Err(WorkgraphError::ValidationError(format!(
                    "trigger '{trigger_id}' ledger pattern must constrain at least one matcher"
                )));
            }
        }
        EventSourceKind::Webhook => {
            if pattern.provider.as_deref().is_none() || pattern.event_name.as_deref().is_none() {
                return Err(WorkgraphError::ValidationError(format!(
                    "trigger '{trigger_id}' webhook patterns require provider and event_name"
                )));
            }
        }
        EventSourceKind::Internal => {
            if pattern.event_name.as_deref().is_none() {
                return Err(WorkgraphError::ValidationError(format!(
                    "trigger '{trigger_id}' internal patterns require event_name"
                )));
            }
        }
    }
    Ok(())
}

fn event_pattern_matches(pattern: &EventPattern, event: &EventEnvelope) -> bool {
    if pattern.source != event.source {
        return false;
    }
    if let Some(event_name) = &pattern.event_name {
        if event.event_name.as_ref() != Some(event_name) {
            return false;
        }
    }
    if let Some(provider) = &pattern.provider {
        if event.provider.as_ref() != Some(provider) {
            return false;
        }
    }
    if let Some(actor_id) = &pattern.actor_id {
        if event.actor_id.as_ref() != Some(actor_id) {
            return false;
        }
    }
    if let Some(subject_reference) = &pattern.subject_reference {
        if event.subject_reference.as_ref() != Some(subject_reference) {
            return false;
        }
    }
    if !pattern.ops.is_empty() && !event.op.is_some_and(|op| pattern.ops.contains(&op)) {
        return false;
    }
    if !pattern.primitive_types.is_empty()
        && event
            .primitive_type
            .as_ref()
            .is_none_or(|primitive_type| !pattern.primitive_types.contains(primitive_type))
    {
        return false;
    }
    if let Some(primitive_id) = &pattern.primitive_id {
        if event.primitive_id.as_ref() != Some(primitive_id) {
            return false;
        }
    }
    if !pattern.field_names.is_empty()
        && !pattern
            .field_names
            .iter()
            .all(|field_name| event.field_names.contains(field_name))
    {
        return false;
    }
    for (field_name, field_value) in &pattern.payload_fields {
        if event.payload_fields.get(field_name) != Some(field_value) {
            return false;
        }
    }
    true
}

fn trigger_to_primitive(trigger: &Trigger) -> Result<StoredPrimitive> {
    let mut extra_fields = BTreeMap::new();
    extra_fields.insert(
        "status".to_owned(),
        serde_yaml::to_value(trigger.status).map_err(encoding_error)?,
    );
    extra_fields.insert(
        "event_pattern".to_owned(),
        serde_yaml::to_value(&trigger.event_pattern).map_err(encoding_error)?,
    );
    if !trigger.action_plans.is_empty() {
        extra_fields.insert(
            "action_plans".to_owned(),
            serde_yaml::to_value(&trigger.action_plans).map_err(encoding_error)?,
        );
    }
    if let Some(subscription_state) = &trigger.subscription_state {
        extra_fields.insert(
            "subscription_state".to_owned(),
            serde_yaml::to_value(subscription_state).map_err(encoding_error)?,
        );
    }

    Ok(StoredPrimitive {
        frontmatter: PrimitiveFrontmatter {
            r#type: TRIGGER_TYPE.to_owned(),
            id: trigger.id.clone(),
            title: trigger.title.clone(),
            extra_fields,
        },
        body: String::new(),
    })
}

/// Decodes a stored primitive into a typed trigger.
///
/// # Errors
///
/// Returns an error when the stored primitive is not a valid trigger payload.
pub fn trigger_from_primitive(primitive: &StoredPrimitive) -> Result<Trigger> {
    if primitive.frontmatter.r#type != TRIGGER_TYPE {
        return Err(WorkgraphError::ValidationError(format!(
            "expected trigger primitive, found '{}'",
            primitive.frontmatter.r#type
        )));
    }

    let trigger = TriggerPrimitive {
        id: primitive.frontmatter.id.clone(),
        title: primitive.frontmatter.title.clone(),
        status: primitive
            .frontmatter
            .extra_fields
            .get("status")
            .map_or(Ok(TriggerStatus::Draft), parse_yaml_value)?,
        event_pattern: primitive
            .frontmatter
            .extra_fields
            .get("event_pattern")
            .map(parse_yaml_value)
            .transpose()?
            .ok_or_else(|| {
                WorkgraphError::ValidationError(format!(
                    "trigger '{}' is missing required event_pattern",
                    primitive.frontmatter.id
                ))
            })?,
        action_plans: primitive
            .frontmatter
            .extra_fields
            .get("action_plans")
            .map_or(Ok(Vec::new()), parse_yaml_value)?,
        subscription_state: primitive
            .frontmatter
            .extra_fields
            .get("subscription_state")
            .map_or(Ok(None), parse_optional_yaml_value)?,
    };
    validate_trigger_definition(&trigger)?;
    Ok(trigger)
}

fn trigger_receipt_to_primitive(receipt: &TriggerReceipt) -> Result<StoredPrimitive> {
    let mut extra_fields = BTreeMap::new();
    extra_fields.insert(
        "trigger_id".to_owned(),
        Value::String(receipt.trigger_id.clone()),
    );
    extra_fields.insert(
        "trigger_title".to_owned(),
        Value::String(receipt.trigger_title.clone()),
    );
    extra_fields.insert(
        "event_id".to_owned(),
        Value::String(receipt.event_id.clone()),
    );
    extra_fields.insert(
        "event_source".to_owned(),
        serde_yaml::to_value(receipt.event_source).map_err(encoding_error)?,
    );
    if let Some(event_name) = &receipt.event_name {
        extra_fields.insert("event_name".to_owned(), Value::String(event_name.clone()));
    }
    if let Some(provider) = &receipt.provider {
        extra_fields.insert("provider".to_owned(), Value::String(provider.clone()));
    }
    if let Some(actor_id) = &receipt.actor_id {
        extra_fields.insert("actor_id".to_owned(), Value::String(actor_id.to_string()));
    }
    if let Some(subject_reference) = &receipt.subject_reference {
        extra_fields.insert(
            "subject_reference".to_owned(),
            Value::String(subject_reference.clone()),
        );
    }
    extra_fields.insert(
        "occurred_at".to_owned(),
        Value::String(receipt.occurred_at.to_rfc3339()),
    );
    extra_fields.insert(
        "dedup_key".to_owned(),
        Value::String(receipt.dedup_key.clone()),
    );
    if !receipt.field_names.is_empty() {
        extra_fields.insert(
            "field_names".to_owned(),
            serde_yaml::to_value(&receipt.field_names).map_err(encoding_error)?,
        );
    }
    if !receipt.payload_fields.is_empty() {
        extra_fields.insert(
            "payload_fields".to_owned(),
            serde_yaml::to_value(&receipt.payload_fields).map_err(encoding_error)?,
        );
    }
    if !receipt.action_outcomes.is_empty() {
        extra_fields.insert(
            "action_outcomes".to_owned(),
            serde_yaml::to_value(&receipt.action_outcomes).map_err(encoding_error)?,
        );
    }

    Ok(StoredPrimitive {
        frontmatter: PrimitiveFrontmatter {
            r#type: TRIGGER_RECEIPT_TYPE.to_owned(),
            id: receipt.id.clone(),
            title: receipt.title.clone(),
            extra_fields,
        },
        body: String::new(),
    })
}

/// Decodes a stored primitive into a typed trigger receipt.
///
/// # Errors
///
/// Returns an error when the stored primitive is not a valid trigger receipt payload.
pub fn trigger_receipt_from_primitive(primitive: &StoredPrimitive) -> Result<TriggerReceipt> {
    if primitive.frontmatter.r#type != TRIGGER_RECEIPT_TYPE {
        return Err(WorkgraphError::ValidationError(format!(
            "expected trigger receipt primitive, found '{}'",
            primitive.frontmatter.r#type
        )));
    }

    Ok(TriggerReceiptPrimitive {
        id: primitive.frontmatter.id.clone(),
        title: primitive.frontmatter.title.clone(),
        trigger_id: required_string_field(primitive, "trigger_id")?,
        trigger_title: required_string_field(primitive, "trigger_title")?,
        event_id: required_string_field(primitive, "event_id")?,
        event_source: primitive
            .frontmatter
            .extra_fields
            .get("event_source")
            .map(parse_yaml_value)
            .transpose()?
            .ok_or_else(|| {
                WorkgraphError::ValidationError(format!(
                    "trigger receipt '{}' is missing required event_source",
                    primitive.frontmatter.id
                ))
            })?,
        event_name: primitive
            .frontmatter
            .extra_fields
            .get("event_name")
            .and_then(string_value)
            .map(str::to_owned),
        provider: primitive
            .frontmatter
            .extra_fields
            .get("provider")
            .and_then(string_value)
            .map(str::to_owned),
        actor_id: primitive
            .frontmatter
            .extra_fields
            .get("actor_id")
            .and_then(string_value)
            .map(ActorId::new),
        subject_reference: primitive
            .frontmatter
            .extra_fields
            .get("subject_reference")
            .and_then(string_value)
            .map(str::to_owned),
        occurred_at: primitive
            .frontmatter
            .extra_fields
            .get("occurred_at")
            .map(parse_datetime)
            .transpose()?
            .ok_or_else(|| {
                WorkgraphError::ValidationError(format!(
                    "trigger receipt '{}' is missing required occurred_at",
                    primitive.frontmatter.id
                ))
            })?,
        dedup_key: required_string_field(primitive, "dedup_key")?,
        field_names: primitive
            .frontmatter
            .extra_fields
            .get("field_names")
            .map_or(Ok(Vec::new()), parse_yaml_value)?,
        payload_fields: primitive
            .frontmatter
            .extra_fields
            .get("payload_fields")
            .map_or(Ok(BTreeMap::new()), parse_yaml_value)?,
        action_outcomes: primitive
            .frontmatter
            .extra_fields
            .get("action_outcomes")
            .map_or(Ok(Vec::new()), parse_yaml_value)?,
    })
}

fn parse_yaml_value<T>(value: &Value) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_yaml::from_value::<T>(value.clone()).map_err(encoding_error)
}

fn parse_optional_yaml_value<T>(value: &Value) -> Result<Option<T>>
where
    T: serde::de::DeserializeOwned,
{
    serde_yaml::from_value::<T>(value.clone())
        .map(Some)
        .map_err(encoding_error)
}

fn required_string_field(primitive: &StoredPrimitive, field_name: &str) -> Result<String> {
    primitive
        .frontmatter
        .extra_fields
        .get(field_name)
        .and_then(string_value)
        .map(str::to_owned)
        .ok_or_else(|| {
            WorkgraphError::ValidationError(format!(
                "{} '{}' is missing required {}",
                primitive.frontmatter.r#type, primitive.frontmatter.id, field_name
            ))
        })
}

fn string_value(value: &Value) -> Option<&str> {
    match value {
        Value::String(value) => Some(value.as_str()),
        Value::Tagged(tagged) => string_value(&tagged.value),
        Value::Null
        | Value::Bool(_)
        | Value::Number(_)
        | Value::Sequence(_)
        | Value::Mapping(_) => None,
    }
}

fn parse_datetime(value: &Value) -> Result<DateTime<Utc>> {
    match value {
        Value::String(value) => DateTime::parse_from_rfc3339(value)
            .map(|timestamp| timestamp.with_timezone(&Utc))
            .map_err(encoding_error),
        Value::Tagged(tagged) => parse_datetime(&tagged.value),
        Value::Null
        | Value::Bool(_)
        | Value::Number(_)
        | Value::Sequence(_)
        | Value::Mapping(_) => Err(WorkgraphError::ValidationError(
            "expected RFC3339 datetime string".to_owned(),
        )),
    }
}

pub(crate) fn dedup_key(trigger_id: &str, event_id: &str) -> String {
    format!("{trigger_id}::{event_id}")
}

pub(crate) fn trigger_receipt_id(trigger_id: &str, event_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(trigger_id.as_bytes());
    hasher.update(b"::");
    hasher.update(event_id.as_bytes());
    let digest = hasher.finalize();
    let hash = digest[..12]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    format!("{}-{hash}", slug_component(trigger_id))
}

fn encoding_error(error: impl std::fmt::Display) -> WorkgraphError {
    WorkgraphError::EncodingError(error.to_string())
}

fn system_actor() -> ActorId {
    ActorId::new("system:workgraph")
}

fn slug_component(input: &str) -> String {
    let slug = input
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>();
    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "trigger".to_owned()
    } else {
        trimmed.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use tempfile::tempdir;
    use wg_clock::MockClock;
    use wg_ledger::{LedgerEntryDraft, LedgerReader, LedgerWriter};
    use wg_paths::WorkspacePath;

    use chrono::{TimeZone, Utc};
    use wg_types::{
        ActorId, EventEnvelope, EventPattern, EventSourceKind, LedgerOp, TriggerActionPlan,
        TriggerPlanDecision, TriggerPrimitive, TriggerStatus,
    };

    use crate::{
        evaluate_event, evaluate_ledger_entry, event_from_ledger_entry, save_trigger,
        validate_trigger_definition,
    };

    fn ledger_trigger(id: &str) -> TriggerPrimitive {
        TriggerPrimitive {
            id: id.to_owned(),
            title: "React to thread completion".to_owned(),
            status: TriggerStatus::Active,
            event_pattern: EventPattern {
                source: EventSourceKind::Ledger,
                event_name: None,
                ops: vec![LedgerOp::Done],
                primitive_types: vec!["thread".to_owned()],
                primitive_id: None,
                field_names: vec!["evidence".to_owned()],
                provider: None,
                actor_id: None,
                subject_reference: None,
                payload_fields: BTreeMap::new(),
            },
            action_plans: vec![TriggerActionPlan {
                kind: "rebrief_actor".to_owned(),
                target_reference: Some("agent/cursor".to_owned()),
                instruction: "Refresh the actor brief".to_owned(),
            }],
            subscription_state: None,
        }
    }

    #[test]
    fn validation_rejects_incomplete_trigger_contracts() {
        let invalid = TriggerPrimitive {
            id: "bad".to_owned(),
            title: "Bad trigger".to_owned(),
            status: TriggerStatus::Active,
            event_pattern: EventPattern {
                source: EventSourceKind::Webhook,
                event_name: None,
                ops: Vec::new(),
                primitive_types: Vec::new(),
                primitive_id: None,
                field_names: Vec::new(),
                provider: None,
                actor_id: None,
                subject_reference: None,
                payload_fields: BTreeMap::new(),
            },
            action_plans: vec![TriggerActionPlan {
                kind: "notify".to_owned(),
                target_reference: None,
                instruction: "Notify someone".to_owned(),
            }],
            subscription_state: None,
        };

        let error = validate_trigger_definition(&invalid)
            .expect_err("incomplete webhook pattern should fail");
        assert!(error.to_string().contains("provider"));
    }

    #[tokio::test]
    async fn ledger_event_matching_returns_action_plans() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());
        save_trigger(&workspace, &ledger_trigger("trigger-1"))
            .await
            .expect("trigger should persist");

        let (entries, _) = LedgerReader::new(temp_dir.path().to_path_buf())
            .read_from(Default::default())
            .await
            .expect("ledger should be readable");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].op, LedgerOp::Create);

        let clock = MockClock::new(
            "2026-03-22T10:00:00Z"
                .parse()
                .expect("timestamp should parse"),
        );
        let writer = LedgerWriter::new(temp_dir.path(), clock);
        let entry = writer
            .append(LedgerEntryDraft::new(
                ActorId::new("agent:cursor"),
                LedgerOp::Done,
                "thread",
                "thread-1",
                vec!["evidence".to_owned()],
            ))
            .await
            .expect("ledger append should succeed");

        let matches = evaluate_ledger_entry(&workspace, &entry)
            .await
            .expect("matching should succeed");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].event.source, EventSourceKind::Ledger);
        assert_eq!(matches[0].action_outcomes.len(), 1);
        assert_eq!(matches[0].action_outcomes[0].plan.kind, "rebrief_actor");
        assert_eq!(
            matches[0].action_outcomes[0].decision,
            TriggerPlanDecision::Allow
        );
    }

    #[tokio::test]
    async fn webhook_and_internal_patterns_validate_but_do_not_match_ledger_events() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());
        let webhook = TriggerPrimitive {
            id: "webhook-trigger".to_owned(),
            title: "Webhook trigger".to_owned(),
            status: TriggerStatus::Active,
            event_pattern: EventPattern {
                source: EventSourceKind::Webhook,
                event_name: Some("pull_request.merged".to_owned()),
                ops: Vec::new(),
                primitive_types: Vec::new(),
                primitive_id: None,
                field_names: Vec::new(),
                provider: Some("github".to_owned()),
                actor_id: None,
                subject_reference: None,
                payload_fields: BTreeMap::new(),
            },
            action_plans: vec![TriggerActionPlan {
                kind: "create_thread".to_owned(),
                target_reference: Some("project/dealer-portal".to_owned()),
                instruction: "Create a follow-up thread".to_owned(),
            }],
            subscription_state: None,
        };
        save_trigger(&workspace, &webhook)
            .await
            .expect("webhook trigger should persist");

        let clock = MockClock::new(
            "2026-03-22T10:00:00Z"
                .parse()
                .expect("timestamp should parse"),
        );
        let writer = LedgerWriter::new(temp_dir.path(), clock);
        let entry = writer
            .append(LedgerEntryDraft::new(
                ActorId::new("agent:cursor"),
                LedgerOp::Done,
                "thread",
                "thread-1",
                vec!["evidence".to_owned()],
            ))
            .await
            .expect("ledger append should succeed");

        let matches = evaluate_ledger_entry(&workspace, &entry)
            .await
            .expect("matching should succeed");
        assert!(matches.is_empty());
    }

    #[test]
    fn event_from_ledger_entry_populates_normalized_fields() {
        let entry = wg_types::LedgerEntry {
            ts: Utc
                .with_ymd_and_hms(2026, 3, 22, 10, 0, 0)
                .single()
                .expect("timestamp should be valid"),
            actor: ActorId::new("agent:cursor"),
            op: LedgerOp::Done,
            primitive_type: "thread".to_owned(),
            primitive_id: "thread-1".to_owned(),
            fields_changed: vec!["status".to_owned(), "evidence".to_owned()],
            hash: "hash-123".to_owned(),
            prev_hash: None,
            note: None,
        };
        let event = event_from_ledger_entry(&entry);
        assert_eq!(event.id, "hash-123");
        assert_eq!(event.source, EventSourceKind::Ledger);
        assert_eq!(event.event_name.as_deref(), Some("thread.done"));
        assert_eq!(event.subject_reference.as_deref(), Some("thread/thread-1"));
        assert_eq!(
            event.payload_fields.get("primitive_type"),
            Some(&"thread".to_owned())
        );
    }

    #[tokio::test]
    async fn internal_event_matching_uses_event_envelope_fields() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());
        let trigger = TriggerPrimitive {
            id: "internal-trigger".to_owned(),
            title: "Internal trigger".to_owned(),
            status: TriggerStatus::Active,
            event_pattern: EventPattern {
                source: EventSourceKind::Internal,
                event_name: Some("checkpoint.saved".to_owned()),
                ops: Vec::new(),
                primitive_types: vec!["checkpoint".to_owned()],
                primitive_id: None,
                field_names: vec!["focus".to_owned()],
                provider: Some("signal-bus".to_owned()),
                actor_id: Some(ActorId::new("agent:cursor")),
                subject_reference: Some("checkpoint/checkpoint-1".to_owned()),
                payload_fields: BTreeMap::from([("focus".to_owned(), "Phase 3".to_owned())]),
            },
            action_plans: vec![TriggerActionPlan {
                kind: "rebrief_actor".to_owned(),
                target_reference: Some("agent/cursor".to_owned()),
                instruction: "Refresh trigger plane summary".to_owned(),
            }],
            subscription_state: None,
        };
        save_trigger(&workspace, &trigger)
            .await
            .expect("internal trigger should persist");

        let event = EventEnvelope {
            id: "internal-event-1".to_owned(),
            source: EventSourceKind::Internal,
            event_name: Some("checkpoint.saved".to_owned()),
            provider: Some("signal-bus".to_owned()),
            actor_id: Some(ActorId::new("agent:cursor")),
            occurred_at: Utc
                .with_ymd_and_hms(2026, 3, 22, 10, 0, 0)
                .single()
                .expect("timestamp should be valid"),
            op: None,
            primitive_type: Some("checkpoint".to_owned()),
            primitive_id: Some("checkpoint-1".to_owned()),
            subject_reference: Some("checkpoint/checkpoint-1".to_owned()),
            field_names: vec!["focus".to_owned()],
            payload_fields: BTreeMap::from([("focus".to_owned(), "Phase 3".to_owned())]),
        };

        let matches = evaluate_event(&workspace, &event)
            .await
            .expect("internal event should evaluate");
        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0].event.event_name.as_deref(),
            Some("checkpoint.saved")
        );
    }
}
