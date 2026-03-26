#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Durable trigger parsing, validation, and ledger-event matching for WorkGraph.

use serde_yaml::Value;
use wg_error::{Result, WorkgraphError};
use wg_paths::WorkspacePath;
use wg_store::{
    AuditedWriteRequest, PrimitiveFrontmatter, StoredPrimitive, list_primitives, read_primitive,
    write_primitive_audited_now,
};
use wg_types::{
    ActorId, EventPattern, EventSourceKind, LedgerEntry, LedgerOp, Registry, TriggerActionPlan,
    TriggerPrimitive, TriggerStatus,
};

mod mutation;

const TRIGGER_TYPE: &str = "trigger";
const SYSTEM_ACTOR: &str = "system:workgraph";

/// Typed trigger model persisted by this crate.
pub type Trigger = TriggerPrimitive;

pub use mutation::TriggerMutationService;

/// Result of matching a trigger against a concrete event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchedTrigger {
    /// Matched trigger identifier.
    pub trigger_id: String,
    /// Matched trigger title.
    pub title: String,
    /// Action plans emitted by the trigger.
    pub action_plans: Vec<TriggerActionPlan>,
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

/// Evaluates all active triggers against a ledger entry.
///
/// # Errors
///
/// Returns an error when persisted triggers cannot be loaded or decoded.
pub async fn evaluate_ledger_entry(
    workspace: &WorkspacePath,
    entry: &LedgerEntry,
) -> Result<Vec<MatchedTrigger>> {
    let triggers = list_triggers(workspace).await?;
    Ok(triggers
        .into_iter()
        .filter(|trigger| {
            trigger.status == TriggerStatus::Active
                && trigger.event_pattern.source == EventSourceKind::Ledger
                && ledger_pattern_matches(&trigger.event_pattern, entry)
        })
        .map(|trigger| MatchedTrigger {
            trigger_id: trigger.id,
            title: trigger.title,
            action_plans: trigger.action_plans,
        })
        .collect())
}

async fn save_trigger_with_audit(
    workspace: &WorkspacePath,
    trigger: &Trigger,
    audit: AuditedWriteRequest,
) -> Result<()> {
    let primitive = trigger_to_primitive(trigger)?;
    write_primitive_audited_now(workspace, &Registry::builtins(), &primitive, audit).await?;
    Ok(())
}

fn validate_event_pattern(trigger_id: &str, pattern: &EventPattern) -> Result<()> {
    match pattern.source {
        EventSourceKind::Ledger => {
            if pattern.event_name.is_some() || pattern.provider.is_some() {
                return Err(WorkgraphError::ValidationError(format!(
                    "trigger '{trigger_id}' ledger patterns may not set provider or event_name"
                )));
            }
            if pattern.ops.is_empty()
                && pattern.primitive_types.is_empty()
                && pattern.primitive_id.is_none()
                && pattern.field_names.is_empty()
            {
                return Err(WorkgraphError::ValidationError(format!(
                    "trigger '{trigger_id}' ledger pattern must constrain at least one of ops, primitive_types, primitive_id, or field_names"
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

fn ledger_pattern_matches(pattern: &EventPattern, entry: &LedgerEntry) -> bool {
    if !pattern.ops.is_empty() && !pattern.ops.contains(&entry.op) {
        return false;
    }
    if !pattern.primitive_types.is_empty()
        && !pattern.primitive_types.contains(&entry.primitive_type)
    {
        return false;
    }
    if let Some(primitive_id) = &pattern.primitive_id {
        if primitive_id != &entry.primitive_id {
            return false;
        }
    }
    if !pattern.field_names.is_empty()
        && !pattern
            .field_names
            .iter()
            .all(|field_name| entry.fields_changed.contains(field_name))
    {
        return false;
    }
    true
}

fn trigger_to_primitive(trigger: &Trigger) -> Result<StoredPrimitive> {
    let mut extra_fields = std::collections::BTreeMap::new();
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

fn trigger_from_primitive(primitive: &StoredPrimitive) -> Result<Trigger> {
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
    };
    validate_trigger_definition(&trigger)?;
    Ok(trigger)
}

fn parse_yaml_value<T>(value: &Value) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_yaml::from_value::<T>(value.clone()).map_err(encoding_error)
}

fn encoding_error(error: impl std::fmt::Display) -> WorkgraphError {
    WorkgraphError::EncodingError(error.to_string())
}

fn system_actor() -> ActorId {
    ActorId::new(SYSTEM_ACTOR)
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use wg_clock::MockClock;
    use wg_ledger::{LedgerEntryDraft, LedgerReader, LedgerWriter};
    use wg_paths::WorkspacePath;
    use wg_types::{
        ActorId, EventPattern, EventSourceKind, LedgerOp, TriggerActionPlan, TriggerPrimitive,
        TriggerStatus,
    };

    use crate::{evaluate_ledger_entry, save_trigger, validate_trigger_definition};

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
            },
            action_plans: vec![TriggerActionPlan {
                kind: "rebrief_actor".to_owned(),
                target_reference: Some("agent/cursor".to_owned()),
                instruction: "Refresh the actor brief".to_owned(),
            }],
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
            },
            action_plans: vec![TriggerActionPlan {
                kind: "notify".to_owned(),
                target_reference: None,
                instruction: "Notify someone".to_owned(),
            }],
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
        assert_eq!(entries[0].op, LedgerOp::Update);

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
        assert_eq!(matches[0].action_plans.len(), 1);
        assert_eq!(matches[0].action_plans[0].kind, "rebrief_actor");
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
            },
            action_plans: vec![TriggerActionPlan {
                kind: "create_thread".to_owned(),
                target_reference: Some("project/dealer-portal".to_owned()),
                instruction: "Create a follow-up thread".to_owned(),
            }],
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
}
