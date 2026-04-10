use std::collections::BTreeMap;

use chrono::Utc;
use serde_yaml::Value;
use wg_error::{Result, WorkgraphError};
use wg_paths::WorkspacePath;
use wg_policy::{PolicyAction, PolicyContext, PolicyDecision, evaluate as evaluate_policy};
use wg_store::{
    AuditedWriteRequest, PrimitiveFrontmatter, StoredPrimitive, read_primitive,
    write_primitive_audited_now,
};
use wg_trigger::ingest_ledger_entry;
use wg_types::{ActorId, FieldDefinition, LedgerOp, PrimitiveType, Registry};

/// Domain mutation service for checkpoint persistence.
///
/// Checkpoints are coordination primitives even though they are created from the
/// orientation surface. This service keeps checkpoint semantics above raw store
/// persistence so future hook execution stays aligned with the rest of the
/// coordination families.
#[derive(Debug, Clone, Copy)]
pub struct CheckpointMutationService<'a> {
    workspace: &'a WorkspacePath,
}

impl<'a> CheckpointMutationService<'a> {
    /// Creates a new checkpoint mutation service for a workspace.
    #[must_use]
    pub fn new(workspace: &'a WorkspacePath) -> Self {
        Self { workspace }
    }

    /// Saves a checkpoint primitive for the current work focus.
    ///
    /// # Errors
    ///
    /// Returns an error when checkpoint persistence fails.
    pub async fn checkpoint(self, working_on: &str, focus: &str) -> Result<StoredPrimitive> {
        if working_on.trim().is_empty() {
            return Err(WorkgraphError::ValidationError(
                "checkpoint working_on must not be empty".to_owned(),
            ));
        }
        if focus.trim().is_empty() {
            return Err(WorkgraphError::ValidationError(
                "checkpoint focus must not be empty".to_owned(),
            ));
        }

        let id = format!(
            "{}-{}",
            slugify(working_on),
            Utc::now().format("%Y%m%d%H%M%S")
        );
        let title = format!("Checkpoint: {}", working_on.trim());
        let created_at = Utc::now().to_rfc3339();
        let primitive = StoredPrimitive {
            frontmatter: PrimitiveFrontmatter {
                r#type: "checkpoint".to_owned(),
                id: id.clone(),
                title,
                extra_fields: BTreeMap::from([
                    (
                        "working_on".to_owned(),
                        Value::String(working_on.trim().to_owned()),
                    ),
                    ("focus".to_owned(), Value::String(focus.trim().to_owned())),
                    ("created_at".to_owned(), Value::String(created_at)),
                ]),
            },
            body: format!("## Working on\n{working_on}\n\n## Focus\n{focus}\n"),
        };
        let audit = AuditedWriteRequest::new(ActorId::new("system:workgraph"), LedgerOp::Create)
            .with_note(format!("Saved checkpoint '{}'", id));

        self.authorize(&id, &audit).await?;
        let (_, ledger_entry) = write_primitive_audited_now(
            self.workspace,
            &checkpoint_registry(),
            &primitive,
            audit.clone(),
        )
        .await?;
        self.after_mutation(&primitive, &audit, &ledger_entry)
            .await?;
        read_primitive(self.workspace, "checkpoint", &id).await
    }

    async fn authorize(self, checkpoint_id: &str, audit: &AuditedWriteRequest) -> Result<()> {
        let decision = evaluate_policy(
            self.workspace,
            &audit.actor,
            PolicyAction::Create,
            "checkpoint",
            &PolicyContext::default(),
        )
        .await?;
        if decision == PolicyDecision::Deny {
            return Err(WorkgraphError::ValidationError(format!(
                "policy denied create of checkpoint/{checkpoint_id} for actor '{}'",
                audit.actor
            )));
        }
        Ok(())
    }

    async fn after_mutation(
        self,
        _primitive: &StoredPrimitive,
        _audit: &AuditedWriteRequest,
        ledger_entry: &wg_types::LedgerEntry,
    ) -> Result<()> {
        ingest_ledger_entry(self.workspace, ledger_entry).await?;
        Ok(())
    }
}

pub(crate) fn checkpoint_registry() -> Registry {
    let mut registry = Registry::builtins();
    if registry.get_type("checkpoint").is_none() {
        registry.types.push(PrimitiveType::new(
            "checkpoint",
            "checkpoints",
            "Saved orientation checkpoint",
            vec![
                FieldDefinition::new("id", "string", "Stable checkpoint identifier", true, false),
                FieldDefinition::new("title", "string", "Checkpoint title", true, false),
                FieldDefinition::new("working_on", "string", "Current work item", true, false),
                FieldDefinition::new("focus", "string", "Current focus", true, false),
                FieldDefinition::new(
                    "created_at",
                    "datetime",
                    "Checkpoint timestamp",
                    true,
                    false,
                ),
            ],
        ));
    }
    registry
}

fn slugify(input: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;

    for character in input.chars() {
        let lower = character.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            slug.push(lower);
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }

    let trimmed = slug.trim_matches('-');
    if trimmed.is_empty() {
        "checkpoint".to_owned()
    } else {
        trimmed.to_owned()
    }
}
