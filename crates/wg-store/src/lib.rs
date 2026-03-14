#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Markdown-native primitive storage, validation, and querying for WorkGraph.
//!
//! This crate owns the filesystem-backed primitive store used by the WorkGraph
//! kernel. Primitives are persisted as Markdown documents with YAML
//! frontmatter, stored under pluralized per-type directories.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use tokio::fs;
use wg_encoding::{parse_frontmatter, write_frontmatter};
use wg_error::{Result, WorkgraphError};
use wg_fs::{atomic_write, ensure_dir, list_md_files};
use wg_paths::{StorePath, WorkspacePath};
use wg_types::Registry;

/// Typed YAML frontmatter stored at the top of a primitive Markdown document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrimitiveFrontmatter {
    /// The stable primitive type name stored as the `type` frontmatter field.
    #[serde(rename = "type")]
    pub r#type: String,
    /// The stable primitive identifier.
    pub id: String,
    /// The human-readable primitive title.
    pub title: String,
    /// Additional frontmatter fields preserved alongside the required fields.
    #[serde(flatten, default, skip_serializing_if = "BTreeMap::is_empty")]
    pub extra_fields: BTreeMap<String, Value>,
}

/// A primitive loaded from or ready to be written to the markdown store.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StoredPrimitive {
    /// The typed YAML frontmatter associated with the primitive.
    pub frontmatter: PrimitiveFrontmatter,
    /// The Markdown body stored after the closing frontmatter fence.
    pub body: String,
}

/// An exact-match scalar frontmatter filter expressed as `field=value`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FieldFilter {
    /// The frontmatter field name to compare.
    pub field: String,
    /// The exact scalar value to match after string conversion.
    pub value: String,
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

/// Loads all primitives of a type and filters them by exact scalar field values.
///
/// Filters are applied to the built-in `type`, `id`, and `title` fields as well
/// as any scalar values present in [`PrimitiveFrontmatter::extra_fields`].
/// Sequence and mapping values never match scalar filters.
///
/// # Errors
///
/// Returns any error produced while listing stored primitives of the requested
/// type.
pub async fn query_primitives(
    workspace: &WorkspacePath,
    primitive_type: &str,
    filters: &[FieldFilter],
) -> Result<Vec<StoredPrimitive>> {
    let primitives = list_primitives(workspace, primitive_type).await?;

    Ok(primitives
        .into_iter()
        .filter(|primitive| {
            filters
                .iter()
                .all(|filter| matches_filter(primitive, filter))
        })
        .collect())
}

/// Validates a primitive against required WorkGraph storage rules and registry metadata.
///
/// Validation ensures the primitive has a non-empty type, identifier, and
/// title, that its type is registered, that reserved field names are not
/// duplicated inside `extra_fields`, and that every required registry-defined
/// field is present.
///
/// Required fields named `type`, `id`, `title`, and `body` are validated
/// against the canonical primitive fields. All other required fields must be
/// present in [`PrimitiveFrontmatter::extra_fields`].
///
/// # Errors
///
/// Returns [`wg_error::WorkgraphError::ValidationError`] when any validation
/// rule fails.
pub fn validate_primitive(registry: &Registry, primitive: &StoredPrimitive) -> Result<()> {
    validate_reserved_fields(&primitive.frontmatter)?;

    let primitive_type = registry
        .get_type(&primitive.frontmatter.r#type)
        .ok_or_else(|| {
            WorkgraphError::ValidationError(format!(
                "primitive type '{}' is not registered",
                primitive.frontmatter.r#type
            ))
        })?;

    for definition in &primitive_type.fields {
        if !definition.required {
            continue;
        }

        let is_present = match definition.name.as_str() {
            "type" => has_text(&primitive.frontmatter.r#type),
            "id" => has_text(&primitive.frontmatter.id),
            "title" => has_text(&primitive.frontmatter.title),
            "body" => has_text(&primitive.body),
            field_name => primitive
                .frontmatter
                .extra_fields
                .get(field_name)
                .is_some_and(value_is_present),
        };

        if !is_present {
            return Err(WorkgraphError::ValidationError(format!(
                "primitive '{}' of type '{}' is missing required field '{}'",
                primitive.frontmatter.id, primitive.frontmatter.r#type, definition.name
            )));
        }
    }

    Ok(())
}

fn validate_reserved_fields(frontmatter: &PrimitiveFrontmatter) -> Result<()> {
    if !has_text(&frontmatter.r#type) {
        return Err(WorkgraphError::ValidationError(
            "primitive type must not be empty".to_owned(),
        ));
    }

    if !has_text(&frontmatter.id) {
        return Err(WorkgraphError::ValidationError(
            "primitive id must not be empty".to_owned(),
        ));
    }

    if !has_text(&frontmatter.title) {
        return Err(WorkgraphError::ValidationError(
            "primitive title must not be empty".to_owned(),
        ));
    }

    for reserved in ["type", "id", "title"] {
        if frontmatter.extra_fields.contains_key(reserved) {
            return Err(WorkgraphError::ValidationError(format!(
                "primitive '{}' duplicates reserved field '{}'",
                frontmatter.id, reserved
            )));
        }
    }

    Ok(())
}

fn matches_filter(primitive: &StoredPrimitive, filter: &FieldFilter) -> bool {
    match filter.field.as_str() {
        "type" => primitive.frontmatter.r#type == filter.value,
        "id" => primitive.frontmatter.id == filter.value,
        "title" => primitive.frontmatter.title == filter.value,
        field_name => primitive
            .frontmatter
            .extra_fields
            .get(field_name)
            .and_then(scalar_value)
            .is_some_and(|value| value == filter.value),
    }
}

fn scalar_value(value: &Value) -> Option<String> {
    match value {
        Value::Bool(value) => Some(value.to_string()),
        Value::Number(value) => Some(value.to_string()),
        Value::String(value) => Some(value.clone()),
        Value::Tagged(tagged) => scalar_value(&tagged.value),
        Value::Null | Value::Sequence(_) | Value::Mapping(_) => None,
    }
}

fn value_is_present(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Bool(_) | Value::Number(_) => true,
        Value::String(value) => has_text(value),
        Value::Sequence(values) => !values.is_empty(),
        Value::Mapping(values) => !values.is_empty(),
        Value::Tagged(tagged) => value_is_present(&tagged.value),
    }
}

fn has_text(value: &str) -> bool {
    !value.trim().is_empty()
}

async fn load_primitive_file(
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

fn validate_loaded_primitive(path: &Path, frontmatter: &PrimitiveFrontmatter) -> Result<()> {
    validate_reserved_fields(frontmatter).map_err(|error| match error {
        WorkgraphError::ValidationError(message) => WorkgraphError::StoreError(format!(
            "invalid primitive stored at '{}': {message}",
            path.display()
        )),
        other => other,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        FieldFilter, PrimitiveFrontmatter, StoredPrimitive, list_primitives, query_primitives,
        read_primitive, validate_primitive, write_primitive,
    };
    use std::collections::BTreeMap;

    use serde_yaml::Value;
    use tempfile::tempdir;
    use tokio::fs;
    use wg_error::WorkgraphError;
    use wg_paths::WorkspacePath;
    use wg_types::{FieldDefinition, PrimitiveType, Registry};

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

    fn playbook_registry() -> Registry {
        Registry::new(vec![PrimitiveType::new(
            "playbook",
            "playbooks",
            "Executable team process",
            vec![
                FieldDefinition::new("id", "string", "Stable identifier", true, false),
                FieldDefinition::new("title", "string", "Human title", true, false),
                FieldDefinition::new("body", "string", "Markdown body", true, false),
            ],
        )])
    }

    #[test]
    fn validate_primitive_rejects_missing_required_registry_field() {
        let primitive = StoredPrimitive {
            frontmatter: PrimitiveFrontmatter {
                r#type: "decision".to_owned(),
                id: "missing-status".to_owned(),
                title: "Missing status".to_owned(),
                extra_fields: BTreeMap::new(),
            },
            body: "## Context\n\nNeeds a status field.\n".to_owned(),
        };

        let error = validate_primitive(&decision_registry(), &primitive)
            .expect_err("status should be required");

        match error {
            WorkgraphError::ValidationError(message) => {
                assert!(message.contains("status"));
                assert!(message.contains("missing-status"));
            }
            other => panic!("expected validation error, got {other:?}"),
        }
    }

    #[test]
    fn validate_primitive_rejects_unregistered_types() {
        let primitive = decision_primitive("unknown-type", "Unknown type", "draft", []);

        let error = validate_primitive(&Registry::default(), &primitive)
            .expect_err("unregistered type should fail");

        assert!(matches!(error, WorkgraphError::ValidationError(_)));
        assert!(error.to_string().contains("not registered"));
    }

    #[test]
    fn validate_primitive_checks_required_body_fields() {
        let primitive = StoredPrimitive {
            frontmatter: PrimitiveFrontmatter {
                r#type: "playbook".to_owned(),
                id: "empty-body".to_owned(),
                title: "Empty body".to_owned(),
                extra_fields: BTreeMap::new(),
            },
            body: "   ".to_owned(),
        };

        let error = validate_primitive(&playbook_registry(), &primitive)
            .expect_err("required body should be enforced");

        assert!(matches!(error, WorkgraphError::ValidationError(_)));
        assert!(error.to_string().contains("body"));
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

        let stored_path = write_primitive(&workspace, &registry, &primitive)
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

        write_primitive(&workspace, &registry, &primitive)
            .await
            .expect("primitive should write successfully");

        let loaded = read_primitive(&workspace, "decision", "capture-rationale")
            .await
            .expect("written primitive should roundtrip");

        assert_eq!(loaded, primitive);
    }

    #[tokio::test]
    async fn list_primitives_returns_all_primitives_in_path_order() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());
        let registry = decision_registry();

        write_primitive(
            &workspace,
            &registry,
            &decision_primitive("zeta-choice", "Zeta choice", "accepted", []),
        )
        .await
        .expect("zeta primitive should write");
        write_primitive(
            &workspace,
            &registry,
            &decision_primitive("alpha-choice", "Alpha choice", "draft", []),
        )
        .await
        .expect("alpha primitive should write");

        let primitives = list_primitives(&workspace, "decision")
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

        let primitives = list_primitives(&workspace, "decision")
            .await
            .expect("missing directories should behave like empty stores");

        assert!(primitives.is_empty());
    }

    #[tokio::test]
    async fn query_primitives_applies_exact_scalar_filters() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());
        let registry = decision_registry();

        write_primitive(
            &workspace,
            &registry,
            &decision_primitive(
                "alpha",
                "Alpha",
                "accepted",
                [
                    ("priority", Value::Number(2.into())),
                    ("active", Value::Bool(true)),
                    (
                        "tags",
                        Value::Sequence(vec![Value::String("ops".to_owned())]),
                    ),
                ],
            ),
        )
        .await
        .expect("alpha primitive should write");
        write_primitive(
            &workspace,
            &registry,
            &decision_primitive(
                "beta",
                "Beta",
                "draft",
                [
                    ("priority", Value::Number(1.into())),
                    ("active", Value::Bool(false)),
                ],
            ),
        )
        .await
        .expect("beta primitive should write");

        let accepted = query_primitives(
            &workspace,
            "decision",
            &[FieldFilter {
                field: "status".to_owned(),
                value: "accepted".to_owned(),
            }],
        )
        .await
        .expect("status query should succeed");
        assert_eq!(accepted.len(), 1);
        assert_eq!(accepted[0].frontmatter.id, "alpha");

        let numeric_and_bool = query_primitives(
            &workspace,
            "decision",
            &[
                FieldFilter {
                    field: "priority".to_owned(),
                    value: "2".to_owned(),
                },
                FieldFilter {
                    field: "active".to_owned(),
                    value: "true".to_owned(),
                },
            ],
        )
        .await
        .expect("numeric and boolean query should succeed");
        assert_eq!(numeric_and_bool.len(), 1);
        assert_eq!(numeric_and_bool[0].frontmatter.id, "alpha");

        let non_scalar = query_primitives(
            &workspace,
            "decision",
            &[FieldFilter {
                field: "tags".to_owned(),
                value: "ops".to_owned(),
            }],
        )
        .await
        .expect("non-scalar query should still execute");
        assert!(non_scalar.is_empty());
    }
}
