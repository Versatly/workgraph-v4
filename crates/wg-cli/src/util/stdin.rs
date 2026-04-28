//! Helpers for parsing stdin payloads used by CLI pipeline flows.

use anyhow::{Context, bail};
use std::io::Read as _;

use crate::args::KeyValueInput;

/// Parsed `workgraph create --stdin` payload.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CreateStdinPayload {
    /// Optional title supplied by stdin.
    pub title: Option<String>,
    /// Additional key/value fields supplied by stdin.
    pub fields: Vec<KeyValueInput>,
}

/// Reads and parses a `workgraph create --stdin` JSON payload.
///
/// Expected shape:
/// `{ "title": "...", "fields": { "key": "value" } }`
///
/// # Errors
///
/// Returns an error when stdin cannot be read, contains invalid UTF-8 JSON,
/// or fields cannot be encoded into YAML-compatible values.
pub fn parse_create_stdin_payload() -> anyhow::Result<CreateStdinPayload> {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .context("failed to read stdin payload")?;
    if input.trim().is_empty() {
        bail!("stdin payload is empty");
    }

    let raw: serde_json::Value =
        serde_json::from_str(input.trim()).context("stdin payload must be valid JSON")?;
    let object = raw
        .as_object()
        .ok_or_else(|| anyhow::anyhow!("stdin payload must be a JSON object"))?;

    let title = match object.get("title") {
        Some(serde_json::Value::String(value)) if !value.trim().is_empty() => {
            Some(value.trim().to_owned())
        }
        Some(serde_json::Value::Null) | None => None,
        Some(_) => bail!("stdin payload field 'title' must be a string"),
    };

    let mut fields = Vec::new();
    if let Some(fields_value) = object.get("fields") {
        let fields_object = fields_value
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("stdin payload field 'fields' must be an object"))?;
        for (key, value) in fields_object {
            let scalar = json_value_to_yaml_string(key, value)?;
            fields.push(KeyValueInput {
                key: key.clone(),
                value: scalar,
            });
        }
    }

    for (key, value) in object {
        if key == "title" || key == "fields" {
            continue;
        }
        let scalar = json_value_to_yaml_string(key, value)?;
        if let Some(existing) = fields.iter_mut().find(|field| field.key == *key) {
            existing.value = scalar;
        } else {
            fields.push(KeyValueInput {
                key: key.clone(),
                value: scalar,
            });
        }
    }

    Ok(CreateStdinPayload { title, fields })
}

fn json_value_to_yaml_string(key: &str, value: &serde_json::Value) -> anyhow::Result<String> {
    match value {
        serde_json::Value::Null => Ok(String::new()),
        serde_json::Value::Bool(value) => Ok(value.to_string()),
        serde_json::Value::Number(value) => Ok(value.to_string()),
        serde_json::Value::String(value) => Ok(value.clone()),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => serde_yaml::to_string(value)
            .map(|encoded| encoded.trim().to_owned())
            .with_context(|| format!("stdin payload field '{key}' could not be encoded")),
    }
}

/// Merges CLI field flags and stdin fields (stdin entries override duplicate keys).
#[must_use]
pub fn merge_fields(
    cli_fields: &[KeyValueInput],
    stdin_fields: &[KeyValueInput],
) -> Vec<KeyValueInput> {
    let mut merged = cli_fields.to_vec();
    for stdin_field in stdin_fields {
        if let Some(existing) = merged.iter_mut().find(|field| field.key == stdin_field.key) {
            *existing = stdin_field.clone();
        } else {
            merged.push(stdin_field.clone());
        }
    }
    merged
}
