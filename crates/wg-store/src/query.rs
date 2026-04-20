//! Query and filter logic for stored primitives.

use serde_yaml::Value;
use wg_error::{Result, WorkgraphError};
use wg_paths::WorkspacePath;
use wg_types::{FieldQueryBehavior, Registry};

use crate::document::{FieldFilter, StoredPrimitive};
use crate::io::list_primitives;

/// Loads all primitives of a type and filters them by registry-defined field semantics.
///
/// Filters are applied to the built-in `type`, `id`, and `title` fields as well as
/// registry-defined extra fields. Scalar fields use exact equality, repeated fields
/// may support containment matching, and opaque fields are rejected as unsupported.
///
/// # Errors
///
/// Returns any error produced while listing stored primitives of the requested
/// type, or a validation-style error when a filter references an unknown or non-queryable field.
pub async fn query_primitives(
    workspace: &WorkspacePath,
    registry: &Registry,
    primitive_type: &str,
    filters: &[FieldFilter],
) -> Result<Vec<StoredPrimitive>> {
    let primitive_definition = registry.get_type(primitive_type).ok_or_else(|| {
        WorkgraphError::ValidationError(format!(
            "primitive type '{}' is not registered",
            primitive_type
        ))
    })?;
    validate_filters(primitive_definition, filters)?;
    let primitives = list_primitives(workspace, primitive_type).await?;

    Ok(primitives
        .into_iter()
        .filter(|primitive| {
            filters
                .iter()
                .all(|filter| matches_filter(primitive, primitive_definition, filter))
        })
        .collect())
}

fn validate_filters(
    primitive_definition: &wg_types::PrimitiveType,
    filters: &[FieldFilter],
) -> Result<()> {
    for filter in filters {
        match filter.field.as_str() {
            "type" | "id" | "title" => continue,
            field_name => {
                let Some(definition) = primitive_definition.field(field_name) else {
                    return Err(WorkgraphError::ValidationError(format!(
                        "field '{}' is not part of the '{}' schema; inspect `workgraph schema {}` for valid filters",
                        field_name, primitive_definition.name, primitive_definition.name
                    )));
                };
                if definition.query_behavior == FieldQueryBehavior::Opaque {
                    return Err(WorkgraphError::ValidationError(format!(
                        "field '{}' on '{}' does not support direct query matching",
                        field_name, primitive_definition.name
                    )));
                }
            }
        }
    }
    Ok(())
}

fn matches_filter(
    primitive: &StoredPrimitive,
    primitive_definition: &wg_types::PrimitiveType,
    filter: &FieldFilter,
) -> bool {
    match filter.field.as_str() {
        "type" => primitive.frontmatter.r#type == filter.value,
        "id" => primitive.frontmatter.id == filter.value,
        "title" => primitive.frontmatter.title == filter.value,
        field_name => {
            let Some(definition) = primitive_definition.field(field_name) else {
                return false;
            };
            let Some(value) = primitive.frontmatter.extra_fields.get(field_name) else {
                return false;
            };
            match definition.query_behavior {
                FieldQueryBehavior::Exact => {
                    scalar_value(value).is_some_and(|entry| entry == filter.value)
                }
                FieldQueryBehavior::Contains => repeated_values(value)
                    .iter()
                    .any(|entry| entry == &filter.value),
                FieldQueryBehavior::Opaque => false,
            }
        }
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

fn repeated_values(value: &Value) -> Vec<String> {
    match value {
        Value::String(value) => vec![value.clone()],
        Value::Sequence(values) => values.iter().filter_map(scalar_value).collect(),
        Value::Tagged(tagged) => repeated_values(&tagged.value),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::Mapping(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_yaml::Value;
    use tempfile::tempdir;
    use wg_paths::WorkspacePath;
    use wg_types::{FieldDefinition, FieldQueryBehavior, PrimitiveType, Registry};

    use crate::document::{FieldFilter, FilterOperator, PrimitiveFrontmatter, StoredPrimitive};
    use crate::io::write_primitive;

    fn decision_registry() -> Registry {
        Registry::new(vec![PrimitiveType::new(
            "decision",
            "decisions",
            "Captured rationale",
            vec![
                FieldDefinition::new("id", "string", "Stable identifier", true, false),
                FieldDefinition::new("title", "string", "Human title", true, false),
                FieldDefinition::new("status", "string", "Decision status", true, false),
                FieldDefinition::new("priority", "number", "Priority", false, false),
                FieldDefinition::new("active", "boolean", "Active", false, false),
                FieldDefinition::new("tags", "string[]", "Tags", false, true),
                FieldDefinition::new("snapshot", "object", "Snapshot", false, false)
                    .with_query_behavior(FieldQueryBehavior::Opaque),
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

        let accepted = super::query_primitives(
            &workspace,
            &registry,
            "decision",
            &[FieldFilter {
                field: "status".to_owned(),
                value: "accepted".to_owned(),
                operator: FilterOperator::Exact,
            }],
        )
        .await
        .expect("status query should succeed");
        assert_eq!(accepted.len(), 1);
        assert_eq!(accepted[0].frontmatter.id, "alpha");

        let numeric_and_bool = super::query_primitives(
            &workspace,
            &registry,
            "decision",
            &[
                FieldFilter {
                    field: "priority".to_owned(),
                    value: "2".to_owned(),
                    operator: FilterOperator::Exact,
                },
                FieldFilter {
                    field: "active".to_owned(),
                    value: "true".to_owned(),
                    operator: FilterOperator::Exact,
                },
            ],
        )
        .await
        .expect("numeric and boolean query should succeed");
        assert_eq!(numeric_and_bool.len(), 1);
        assert_eq!(numeric_and_bool[0].frontmatter.id, "alpha");

        let non_scalar = super::query_primitives(
            &workspace,
            &registry,
            "decision",
            &[FieldFilter {
                field: "tags".to_owned(),
                value: "ops".to_owned(),
                operator: FilterOperator::Exact,
            }],
        )
        .await
        .expect("contains query should succeed");
        assert_eq!(non_scalar.len(), 1);
        assert_eq!(non_scalar[0].frontmatter.id, "alpha");
    }

    #[tokio::test]
    async fn query_primitives_rejects_unknown_and_opaque_filters() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());
        let registry = decision_registry();
        write_primitive(
            &workspace,
            &registry,
            &decision_primitive("alpha", "Alpha", "accepted", []),
        )
        .await
        .expect("alpha primitive should write");

        let unknown_error = super::query_primitives(
            &workspace,
            &registry,
            "decision",
            &[FieldFilter {
                field: "unknown".to_owned(),
                value: "x".to_owned(),
                operator: FilterOperator::Exact,
            }],
        )
        .await
        .expect_err("unknown fields should fail");
        assert!(
            unknown_error
                .to_string()
                .contains("not part of the 'decision' schema")
        );

        let opaque_error = super::query_primitives(
            &workspace,
            &registry,
            "decision",
            &[FieldFilter {
                field: "snapshot".to_owned(),
                value: "x".to_owned(),
                operator: FilterOperator::Exact,
            }],
        )
        .await
        .expect_err("opaque fields should fail");
        assert!(
            opaque_error
                .to_string()
                .contains("does not support direct query matching")
        );
    }
}
