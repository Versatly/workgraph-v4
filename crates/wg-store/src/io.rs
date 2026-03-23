//! Filesystem IO operations for stored markdown primitives.

use std::path::Path;

use tokio::fs;
use wg_clock::{Clock, RealClock};
use wg_encoding::{parse_frontmatter, write_frontmatter};
use wg_error::{Result, WorkgraphError};
use wg_fs::{atomic_write, ensure_dir, list_md_files};
use wg_ledger::{LedgerEntryDraft, LedgerWriter};
use wg_paths::{StorePath, WorkspacePath};
use wg_types::{ActorId, LedgerEntry, LedgerOp, Registry};

use crate::document::{PrimitiveFrontmatter, StoredPrimitive};
use crate::validate::{validate_loaded_primitive, validate_primitive};

/// Metadata required to persist a primitive write into the immutable ledger.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuditedWriteRequest {
    /// Actor responsible for the mutation.
    pub actor: ActorId,
    /// Operation captured in the ledger.
    pub op: LedgerOp,
    /// Optional human-readable note describing the mutation.
    pub note: Option<String>,
}

impl AuditedWriteRequest {
    /// Creates a new audited write request.
    #[must_use]
    pub fn new(actor: ActorId, op: LedgerOp) -> Self {
        Self {
            actor,
            op,
            note: None,
        }
    }

    /// Attaches a descriptive note to the audited write.
    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

/// Reads a single primitive from the markdown store by type and identifier.
///
/// The primitive is expected at `<workspace>/<pluralized-type>/<id>.md`.
///
/// # Errors
///
/// Returns an error when the file cannot be read, the Markdown frontmatter
/// cannot be parsed, or the frontmatter does not match the requested type and
/// identifier.
pub async fn read_primitive(
    workspace: &WorkspacePath,
    primitive_type: &str,
    id: &str,
) -> Result<StoredPrimitive> {
    let path = workspace.primitive_path(primitive_type, id);
    load_primitive_file(path.as_path(), Some(primitive_type), Some(id)).await
}

/// Validates and atomically writes a primitive to the markdown store.
///
/// The destination path is resolved from the primitive's type and identifier
/// using [`WorkspacePath`]. Parent directories are created automatically.
///
/// # Errors
///
/// Returns a validation error when the primitive is invalid for the provided
/// registry, an encoding error when frontmatter serialization fails, or an I/O
/// error when persistence fails.
pub async fn write_primitive(
    workspace: &WorkspacePath,
    registry: &Registry,
    primitive: &StoredPrimitive,
) -> Result<StorePath> {
    validate_primitive(registry, primitive)?;

    let directory = workspace.type_dir(&primitive.frontmatter.r#type);
    ensure_dir(directory.as_path()).await?;

    let path = workspace.primitive_path(&primitive.frontmatter.r#type, &primitive.frontmatter.id);
    let document = write_frontmatter(&primitive.frontmatter, &primitive.body)
        .map_err(|error| WorkgraphError::EncodingError(error.to_string()))?;

    atomic_write(path.as_path(), document.as_bytes()).await?;
    Ok(path)
}

/// Validates, writes, and appends a corresponding immutable ledger entry.
///
/// # Errors
///
/// Returns an error when primitive validation, persistence, or ledger append
/// fails.
pub async fn write_primitive_audited<C>(
    workspace: &WorkspacePath,
    registry: &Registry,
    primitive: &StoredPrimitive,
    audit: AuditedWriteRequest,
    clock: C,
) -> Result<(StorePath, LedgerEntry)>
where
    C: Clock,
{
    let path = write_primitive(workspace, registry, primitive).await?;
    let fields_changed = changed_fields(primitive);
    let mut draft = LedgerEntryDraft::new(
        audit.actor,
        audit.op,
        primitive.frontmatter.r#type.clone(),
        primitive.frontmatter.id.clone(),
        fields_changed,
    );
    if let Some(note) = audit.note {
        draft = draft.with_note(note);
    }

    let writer = LedgerWriter::new(workspace.as_path().to_path_buf(), clock);
    let ledger_entry = writer.append(draft).await?;
    Ok((path, ledger_entry))
}

/// Validates, writes, and appends a corresponding immutable ledger entry using the real clock.
///
/// # Errors
///
/// Returns an error when primitive validation, persistence, or ledger append
/// fails.
pub async fn write_primitive_audited_now(
    workspace: &WorkspacePath,
    registry: &Registry,
    primitive: &StoredPrimitive,
    audit: AuditedWriteRequest,
) -> Result<(StorePath, LedgerEntry)> {
    write_primitive_audited(workspace, registry, primitive, audit, RealClock::new()).await
}

/// Lists all stored primitives for a given type.
///
/// Files are loaded from the pluralized type directory and returned in path
/// order for deterministic behavior.
///
/// # Errors
///
/// Returns an error when a stored Markdown file cannot be read or parsed. A
/// missing type directory is treated as an empty result set.
pub async fn list_primitives(
    workspace: &WorkspacePath,
    primitive_type: &str,
) -> Result<Vec<StoredPrimitive>> {
    let directory = workspace.type_dir(primitive_type);
    let paths = match list_md_files(directory.as_path()).await {
        Ok(paths) => paths,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(error.into()),
    };

    let mut primitives = Vec::with_capacity(paths.len());
    for path in paths {
        let id = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or_else(|| {
                WorkgraphError::StoreError(format!(
                    "primitive file path '{}' is missing a valid UTF-8 stem",
                    path.display()
                ))
            })?;

        primitives.push(load_primitive_file(&path, Some(primitive_type), Some(id)).await?);
    }

    Ok(primitives)
}

pub(crate) async fn load_primitive_file(
    path: &Path,
    expected_type: Option<&str>,
    expected_id: Option<&str>,
) -> Result<StoredPrimitive> {
    let document = fs::read_to_string(path).await?;
    let parsed = parse_frontmatter::<PrimitiveFrontmatter>(&document)
        .map_err(|error| WorkgraphError::EncodingError(error.to_string()))?;

    if let Some(expected_type) = expected_type {
        if parsed.frontmatter.r#type != expected_type {
            return Err(WorkgraphError::StoreError(format!(
                "primitive at '{}' declared type '{}' but was loaded as '{}'",
                path.display(),
                parsed.frontmatter.r#type,
                expected_type
            )));
        }
    }

    if let Some(expected_id) = expected_id {
        if parsed.frontmatter.id != expected_id {
            return Err(WorkgraphError::StoreError(format!(
                "primitive at '{}' declared id '{}' but was loaded as '{}'",
                path.display(),
                parsed.frontmatter.id,
                expected_id
            )));
        }
    }

    validate_loaded_primitive(path, &parsed.frontmatter)?;

    Ok(StoredPrimitive {
        frontmatter: parsed.frontmatter,
        body: parsed.body,
    })
}

fn changed_fields(primitive: &StoredPrimitive) -> Vec<String> {
    let mut fields = vec!["id".to_owned(), "title".to_owned(), "type".to_owned()];
    fields.extend(primitive.frontmatter.extra_fields.keys().cloned());
    if !primitive.body.trim().is_empty() {
        fields.push("body".to_owned());
    }
    fields.sort();
    fields.dedup();
    fields
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_yaml::Value;
    use tempfile::tempdir;
    use tokio::fs;
    use wg_ledger::verify_chain;
    use wg_paths::WorkspacePath;
    use wg_types::{ActorId, FieldDefinition, LedgerOp, PrimitiveType, Registry};

    use crate::document::{PrimitiveFrontmatter, StoredPrimitive};

    fn decision_registry() -> Registry {
        Registry::new(vec![PrimitiveType::new(
            "decision",
            "decisions",
            "Captured rationale",
            vec![
                FieldDefinition::new("id", "string", "Stable identifier", true, false),
                FieldDefinition::new("title", "string", "Human title", true, false),
                FieldDefinition::new("status", "string", "Decision status", true, false),
            ],
        )])
    }

    fn decision_primitive(
        id: &str,
        title: &str,
        status: &str,
        extra_fields: impl IntoIterator<Item = (&'static str, Value)>,
    ) -> StoredPrimitive {
        let mut fields = BTreeMap::new();
        fields.insert("status".to_owned(), Value::String(status.to_owned()));
        for (key, value) in extra_fields {
            fields.insert(key.to_owned(), value);
        }

        StoredPrimitive {
            frontmatter: PrimitiveFrontmatter {
                r#type: "decision".to_owned(),
                id: id.to_owned(),
                title: title.to_owned(),
                extra_fields: fields,
            },
            body: format!("## Context\n\nPrimitive {id}\n"),
        }
    }

    #[tokio::test]
    async fn write_primitive_uses_pluralized_directory_layout() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());
        let registry = decision_registry();
        let primitive = decision_primitive(
            "rust-for-workgraph-v4",
            "Rust for WorkGraph v4",
            "decided",
            [],
        );

        let stored_path = super::write_primitive(&workspace, &registry, &primitive)
            .await
            .expect("primitive should write successfully");

        assert_eq!(
            stored_path.as_path(),
            workspace
                .primitive_path("decision", "rust-for-workgraph-v4")
                .as_path()
        );
        assert!(workspace.type_dir("decision").as_path().is_dir());

        let document = fs::read_to_string(stored_path.as_path())
            .await
            .expect("written document should be readable");
        assert!(document.starts_with("---\n"));
        assert!(document.contains("type: decision\n"));
        assert!(document.contains("id: rust-for-workgraph-v4\n"));
        assert!(document.contains("title: Rust for WorkGraph v4\n"));
        assert!(document.contains("status: decided\n"));
        assert!(document.ends_with("## Context\n\nPrimitive rust-for-workgraph-v4\n"));
    }

    #[tokio::test]
    async fn read_primitive_roundtrips_written_documents() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());
        let registry = decision_registry();
        let primitive = decision_primitive(
            "capture-rationale",
            "Capture rationale",
            "proposed",
            [("owner", Value::String("pedro".to_owned()))],
        );

        super::write_primitive(&workspace, &registry, &primitive)
            .await
            .expect("primitive should write successfully");

        let loaded = super::read_primitive(&workspace, "decision", "capture-rationale")
            .await
            .expect("written primitive should roundtrip");

        assert_eq!(loaded, primitive);
    }

    #[tokio::test]
    async fn list_primitives_returns_all_primitives_in_path_order() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());
        let registry = decision_registry();

        super::write_primitive(
            &workspace,
            &registry,
            &decision_primitive("zeta-choice", "Zeta choice", "accepted", []),
        )
        .await
        .expect("zeta primitive should write");
        super::write_primitive(
            &workspace,
            &registry,
            &decision_primitive("alpha-choice", "Alpha choice", "draft", []),
        )
        .await
        .expect("alpha primitive should write");

        let primitives = super::list_primitives(&workspace, "decision")
            .await
            .expect("listing decisions should succeed");

        let ids: Vec<_> = primitives
            .iter()
            .map(|primitive| primitive.frontmatter.id.as_str())
            .collect();
        assert_eq!(ids, vec!["alpha-choice", "zeta-choice"]);
    }

    #[tokio::test]
    async fn list_primitives_returns_empty_when_type_directory_is_missing() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());

        let primitives = super::list_primitives(&workspace, "decision")
            .await
            .expect("missing directories should behave like empty stores");

        assert!(primitives.is_empty());
    }

    #[tokio::test]
    async fn audited_write_appends_ledger_entry_and_verifies_chain() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());
        let registry = decision_registry();
        let primitive = decision_primitive("audited", "Audited decision", "proposed", []);

        let (_, ledger_entry) = super::write_primitive_audited_now(
            &workspace,
            &registry,
            &primitive,
            super::AuditedWriteRequest::new(ActorId::new("pedro"), LedgerOp::Create)
                .with_note("Created during test"),
        )
        .await
        .expect("audited write should succeed");

        assert_eq!(ledger_entry.actor.as_str(), "pedro");
        assert_eq!(ledger_entry.op, LedgerOp::Create);
        assert_eq!(ledger_entry.primitive_type, "decision");
        assert_eq!(ledger_entry.primitive_id, "audited");
        assert!(ledger_entry.fields_changed.contains(&"status".to_owned()));
        assert_eq!(ledger_entry.note.as_deref(), Some("Created during test"));

        verify_chain(temp_dir.path())
            .await
            .expect("audited write should keep the ledger chain valid");
    }
}
