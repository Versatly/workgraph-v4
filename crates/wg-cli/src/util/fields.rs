//! Helpers for parsing and transforming field values supplied through the CLI.

use std::collections::BTreeMap;

use serde_yaml::Value;
use wg_types::PrimitiveType;

use crate::args::KeyValueInput;

/// Parses a `key=value` string into a typed CLI field input.
///
/// # Errors
///
/// Returns an error string when the input does not contain `=` or has an empty key.
pub fn parse_key_value_input(input: &str) -> Result<KeyValueInput, String> {
    let (key, value) = input
        .split_once('=')
        .ok_or_else(|| "expected key=value".to_owned())?;

    if key.trim().is_empty() {
        return Err("field key must not be empty".to_owned());
    }

    Ok(KeyValueInput {
        key: key.trim().to_owned(),
        value: value.to_owned(),
    })
}

/// Splits parsed field arguments into markdown body content and extra frontmatter.
#[must_use]
pub fn split_body_and_frontmatter(
    primitive_type: Option<&PrimitiveType>,
    fields: &[KeyValueInput],
) -> (String, BTreeMap<String, Value>) {
    let mut body = String::new();
    let mut extra_fields = BTreeMap::new();

    for field in fields {
        if field.key == "body" {
            body = field.value.clone();
        } else {
            let parsed = primitive_type
                .and_then(|primitive_type| primitive_type.field(&field.key))
                .map_or_else(
                    || parse_scalar_value(&field.value),
                    |definition| parse_field_value(definition, &field.value),
                );
            extra_fields.insert(field.key.clone(), parsed);
        }
    }

    (body, extra_fields)
}

/// Converts a free-form CLI input string into a scalar YAML value when possible.
#[must_use]
pub fn parse_scalar_value(input: &str) -> Value {
    if let Ok(value) = input.parse::<i64>() {
        return Value::Number(value.into());
    }

    if let Ok(value) = input.parse::<f64>() {
        return serde_yaml::to_value(value).unwrap_or_else(|_| Value::String(input.to_owned()));
    }

    match input {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        _ => Value::String(input.to_owned()),
    }
}

/// Parses a field value according to the registry field contract when available.
#[must_use]
pub fn parse_field_value(definition: &wg_types::FieldDefinition, input: &str) -> Value {
    let trimmed = input.trim();
    if definition.repeated {
        return parse_repeated_value(trimmed);
    }
    if definition.field_type == "object" || definition.field_type == "object[]" {
        return parse_yaml_or_string(trimmed);
    }
    parse_scalar_value(trimmed)
}

fn parse_repeated_value(input: &str) -> Value {
    if input.is_empty() {
        return Value::Sequence(Vec::new());
    }
    if input.starts_with('[') {
        return parse_yaml_or_string(input);
    }
    Value::Sequence(
        input
            .split(',')
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| Value::String(value.to_owned()))
            .collect(),
    )
}

fn parse_yaml_or_string(input: &str) -> Value {
    serde_yaml::from_str::<Value>(input).unwrap_or_else(|_| Value::String(input.to_owned()))
}
