//! CLI-facing serialization helpers for projecting typed coordination models back to primitives.

use anyhow::Context;
use std::collections::BTreeMap;
use wg_dispatch::Run;
use wg_mission::Mission;
use wg_store::{PrimitiveFrontmatter, StoredPrimitive};
use wg_thread::Thread;
use wg_trigger::Trigger;
use wg_types::{CheckpointPrimitive, Registry};

/// Converts a typed thread into a stored primitive using the builtin registry contracts.
///
/// # Errors
///
/// Returns an error when thread serialization fails.
pub fn thread_to_stored(thread: &Thread) -> anyhow::Result<StoredPrimitive> {
    let primitive = serde_yaml::to_value(thread).context("failed to encode thread")?;
    struct_value_to_stored("thread", &thread.id, &thread.title, primitive)
}

/// Converts a typed mission into a stored primitive using the builtin registry contracts.
///
/// # Errors
///
/// Returns an error when mission serialization fails.
pub fn mission_to_stored(mission: &Mission) -> anyhow::Result<StoredPrimitive> {
    let primitive = serde_yaml::to_value(mission).context("failed to encode mission")?;
    struct_value_to_stored("mission", &mission.id, &mission.title, primitive)
}

/// Converts a typed run into a stored primitive using the builtin registry contracts.
///
/// # Errors
///
/// Returns an error when run serialization fails.
pub fn run_to_stored(run: &Run) -> anyhow::Result<StoredPrimitive> {
    let primitive = serde_yaml::to_value(run).context("failed to encode run")?;
    struct_value_to_stored("run", &run.id, &run.title, primitive)
}

/// Converts a typed trigger into a stored primitive using the builtin registry contracts.
///
/// # Errors
///
/// Returns an error when trigger serialization fails.
pub fn trigger_to_stored(trigger: &Trigger) -> anyhow::Result<StoredPrimitive> {
    let primitive = serde_yaml::to_value(trigger).context("failed to encode trigger")?;
    struct_value_to_stored("trigger", &trigger.id, &trigger.title, primitive)
}

/// Converts a typed checkpoint into a stored primitive using the builtin registry contracts.
///
/// # Errors
///
/// Returns an error when checkpoint serialization fails.
pub fn checkpoint_to_stored(checkpoint: &CheckpointPrimitive) -> anyhow::Result<StoredPrimitive> {
    let primitive = serde_yaml::to_value(checkpoint).context("failed to encode checkpoint")?;
    struct_value_to_stored("checkpoint", &checkpoint.id, &checkpoint.title, primitive)
}

fn struct_value_to_stored(
    primitive_type: &str,
    id: &str,
    title: &str,
    primitive: serde_yaml::Value,
) -> anyhow::Result<StoredPrimitive> {
    let mut value = match primitive {
        serde_yaml::Value::Mapping(mapping) => mapping,
        _ => anyhow::bail!("expected mapping when serializing {primitive_type}/{id}"),
    };

    let body = extract_body(primitive_type, &mut value);
    let extra_fields = mapping_to_extra_fields(value)?;

    let stored = StoredPrimitive {
        frontmatter: PrimitiveFrontmatter {
            r#type: primitive_type.to_owned(),
            id: id.to_owned(),
            title: title.to_owned(),
            extra_fields,
        },
        body,
    };
    wg_store::validate_primitive(&Registry::builtins(), &stored)
        .with_context(|| format!("failed to validate serialized {primitive_type}/{id}"))?;
    Ok(stored)
}

fn extract_body(primitive_type: &str, value: &mut serde_yaml::Mapping) -> String {
    let body_key = serde_yaml::Value::String(match primitive_type {
        "mission" => "objective".to_owned(),
        "run" => "summary".to_owned(),
        _ => "body".to_owned(),
    });

    match value.remove(&body_key) {
        Some(serde_yaml::Value::String(body)) => body,
        Some(serde_yaml::Value::Null) | None => String::new(),
        Some(other) => serde_yaml::to_string(&other)
            .map(|text| text.trim().to_owned())
            .unwrap_or_default(),
    }
}

fn mapping_to_extra_fields(
    mapping: serde_yaml::Mapping,
) -> anyhow::Result<BTreeMap<String, serde_yaml::Value>> {
    let mut extra_fields = BTreeMap::new();
    for (key, value) in mapping {
        let Some(key) = key.as_str() else {
            anyhow::bail!("expected string frontmatter field name");
        };
        if matches!(key, "id" | "title") {
            continue;
        }
        extra_fields.insert(key.to_owned(), value);
    }
    Ok(extra_fields)
}
