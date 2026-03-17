//! Query and filter logic for stored primitives.

use serde_yaml::Value;
use wg_error::Result;
use wg_paths::WorkspacePath;

use crate::document::{FieldFilter, StoredPrimitive};
use crate::io::list_primitives;

/// Loads all primitives of a type and filters them by exact scalar field values.
///
/// Filters are applied to the built-in `type`, `id`, and `title` fields as well
/// as any scalar values present in [`crate::PrimitiveFrontmatter::extra_fields`].
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

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_yaml::Value;
    use tempfile::tempdir;
    use wg_paths::WorkspacePath;
    use wg_types::{FieldDefinition, PrimitiveType, Registry};

    use crate::document::{FieldFilter, PrimitiveFrontmatter, StoredPrimitive};
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

        let numeric_and_bool = super::query_primitives(
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

        let non_scalar = super::query_primitives(
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
