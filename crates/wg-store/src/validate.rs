//! Validation rules for stored WorkGraph primitives.

use std::collections::BTreeSet;

use serde_yaml::Value;
use wg_error::{Result, WorkgraphError};
use wg_types::{FieldDefinition, Registry};

use crate::document::{PrimitiveFrontmatter, StoredPrimitive};

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

    let known_fields = primitive_type
        .fields
        .iter()
        .map(|definition| definition.name.as_str())
        .collect::<BTreeSet<_>>();

    for definition in &primitive_type.fields {
        if !definition.required {
            if let Some(value) = primitive.frontmatter.extra_fields.get(&definition.name) {
                validate_field_shape(definition, value).map_err(|message| {
                    WorkgraphError::ValidationError(format!(
                        "primitive '{}' of type '{}' has invalid field '{}': {message}",
                        primitive.frontmatter.id, primitive.frontmatter.r#type, definition.name
                    ))
                })?;
            }
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

        if let Some(value) = primitive.frontmatter.extra_fields.get(&definition.name) {
            validate_field_shape(definition, value).map_err(|message| {
                WorkgraphError::ValidationError(format!(
                    "primitive '{}' of type '{}' has invalid field '{}': {message}",
                    primitive.frontmatter.id, primitive.frontmatter.r#type, definition.name
                ))
            })?;
        }
    }

    for field_name in primitive.frontmatter.extra_fields.keys() {
        if !known_fields.contains(field_name.as_str()) {
            return Err(WorkgraphError::ValidationError(format!(
                "primitive '{}' of type '{}' includes unknown field '{}'; inspect `workgraph schema {}` for the canonical contract",
                primitive.frontmatter.id, primitive.frontmatter.r#type, field_name, primitive.frontmatter.r#type
            )));
        }
    }

    Ok(())
}

/// Validates a primitive loaded from disk and rewrites validation failures as store errors.
pub(crate) fn validate_loaded_primitive(
    path: &std::path::Path,
    frontmatter: &PrimitiveFrontmatter,
) -> Result<()> {
    validate_reserved_fields(frontmatter).map_err(|error| match error {
        WorkgraphError::ValidationError(message) => WorkgraphError::StoreError(format!(
            "invalid primitive stored at '{}': {message}",
            path.display()
        )),
        other => other,
    })
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

fn validate_field_shape(definition: &FieldDefinition, value: &Value) -> std::result::Result<(), String> {
    if definition.repeated {
        let values = sequence_values(value)
            .ok_or_else(|| "expected a YAML sequence for a repeated field".to_owned())?;
        if definition.is_reference() {
            for item in values {
                validate_reference_value(item, definition)?;
            }
        }
        return Ok(());
    }

    if definition.is_reference() {
        return validate_reference_value(value, definition);
    }

    Ok(())
}

fn validate_reference_value(
    value: &Value,
    definition: &FieldDefinition,
) -> std::result::Result<(), String> {
    let reference = scalar_text(value)
        .ok_or_else(|| "expected a scalar reference value".to_owned())?;
    if reference.trim().is_empty() {
        return Err("reference value must not be empty".to_owned());
    }
    if reference.contains('/') {
        let (primitive_type, id) = reference
            .split_once('/')
            .ok_or_else(|| "reference must be in <type>/<id> form when typed".to_owned())?;
        if primitive_type.trim().is_empty() || id.trim().is_empty() {
            return Err("reference must be in <type>/<id> form".to_owned());
        }
        if !definition.reference_types.is_empty()
            && !definition.reference_types.iter().any(|allowed| allowed == primitive_type.trim())
        {
            return Err(format!(
                "expected one of [{}], got '{}'",
                definition.reference_types.join(", "),
                primitive_type.trim()
            ));
        }
    }
    Ok(())
}

fn sequence_values(value: &Value) -> Option<&Vec<Value>> {
    match value {
        Value::Sequence(values) => Some(values),
        Value::Tagged(tagged) => sequence_values(&tagged.value),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) | Value::Mapping(_) => {
            None
        }
    }
}

fn scalar_text(value: &Value) -> Option<&str> {
    match value {
        Value::String(value) => Some(value.as_str()),
        Value::Tagged(tagged) => scalar_text(&tagged.value),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::Sequence(_) | Value::Mapping(_) => {
            None
        }
    }
}

fn has_text(value: &str) -> bool {
    !value.trim().is_empty()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_yaml::Value;
    use wg_error::WorkgraphError;
    use wg_types::{FieldDefinition, PrimitiveType, Registry};

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

    fn person_registry() -> Registry {
        Registry::new(vec![PrimitiveType::new(
            "person",
            "people",
            "Tracked human actor profile.",
            vec![
                FieldDefinition::new("id", "string", "Stable identifier", true, false),
                FieldDefinition::new("title", "string", "Human title", true, false),
                FieldDefinition::new("team_ids", "string[]", "Associated teams", false, true)
                    .with_reference_targets(["team"], wg_types::GraphEdgeKind::Assignment),
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

        let error = super::validate_primitive(&decision_registry(), &primitive)
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
    fn validate_primitive_rejects_unknown_fields() {
        let primitive = StoredPrimitive {
            frontmatter: PrimitiveFrontmatter {
                r#type: "person".to_owned(),
                id: "pedro".to_owned(),
                title: "Pedro".to_owned(),
                extra_fields: BTreeMap::from([(
                    "nickname".to_owned(),
                    Value::String("ped".to_owned()),
                )]),
            },
            body: String::new(),
        };

        let error = super::validate_primitive(&person_registry(), &primitive)
            .expect_err("unknown fields should fail");
        match error {
            WorkgraphError::ValidationError(message) => {
                assert!(message.contains("unknown field 'nickname'"));
                assert!(message.contains("workgraph schema person"));
            }
            other => panic!("expected validation error, got {other:?}"),
        }
    }

    #[test]
    fn validate_primitive_rejects_non_sequence_for_repeated_reference_field() {
        let primitive = StoredPrimitive {
            frontmatter: PrimitiveFrontmatter {
                r#type: "person".to_owned(),
                id: "pedro".to_owned(),
                title: "Pedro".to_owned(),
                extra_fields: BTreeMap::from([(
                    "team_ids".to_owned(),
                    Value::String("team/platform".to_owned()),
                )]),
            },
            body: String::new(),
        };

        let error = super::validate_primitive(&person_registry(), &primitive)
            .expect_err("non-sequence team_ids should fail");
        match error {
            WorkgraphError::ValidationError(message) => {
                assert!(message.contains("expected a YAML sequence"));
            }
            other => panic!("expected validation error, got {other:?}"),
        }
    }

    #[test]
    fn validate_primitive_accepts_typed_repeated_references() {
        let primitive = StoredPrimitive {
            frontmatter: PrimitiveFrontmatter {
                r#type: "person".to_owned(),
                id: "pedro".to_owned(),
                title: "Pedro".to_owned(),
                extra_fields: BTreeMap::from([(
                    "team_ids".to_owned(),
                    Value::Sequence(vec![Value::String("team/platform".to_owned())]),
                )]),
            },
            body: String::new(),
        };

        super::validate_primitive(&person_registry(), &primitive)
            .expect("typed repeated references should validate");
    }

    #[test]
    fn validate_primitive_rejects_unregistered_types() {
        let primitive = StoredPrimitive {
            frontmatter: PrimitiveFrontmatter {
                r#type: "decision".to_owned(),
                id: "unknown-type".to_owned(),
                title: "Unknown type".to_owned(),
                extra_fields: BTreeMap::from([(
                    "status".to_owned(),
                    serde_yaml::Value::String("draft".to_owned()),
                )]),
            },
            body: "## Context\n\nUnknown type.\n".to_owned(),
        };

        let error = super::validate_primitive(&Registry::default(), &primitive)
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

        let error = super::validate_primitive(&playbook_registry(), &primitive)
            .expect_err("required body should be enforced");

        assert!(matches!(error, WorkgraphError::ValidationError(_)));
        assert!(error.to_string().contains("body"));
    }
}
