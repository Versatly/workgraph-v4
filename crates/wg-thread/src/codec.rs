use serde_yaml::Value;
use wg_error::{Result, WorkgraphError};
use wg_store::{PrimitiveFrontmatter, StoredPrimitive};
use wg_types::{ActorId, ThreadPrimitive, ThreadStatus};

use crate::{THREAD_TYPE, Thread, render::parse_conversation_messages};

pub(crate) fn thread_to_primitive(thread: &Thread) -> Result<StoredPrimitive> {
    let mut extra_fields = std::collections::BTreeMap::new();
    extra_fields.insert(
        "status".to_owned(),
        serde_yaml::to_value(thread.status).map_err(encoding_error)?,
    );
    if let Some(actor) = &thread.assigned_actor {
        extra_fields.insert(
            "assigned_actor".to_owned(),
            Value::String(actor.to_string()),
        );
    }
    if let Some(parent_mission_id) = &thread.parent_mission_id {
        extra_fields.insert(
            "parent_mission_id".to_owned(),
            Value::String(parent_mission_id.clone()),
        );
    }
    if !thread.exit_criteria.is_empty() {
        extra_fields.insert(
            "exit_criteria".to_owned(),
            serde_yaml::to_value(&thread.exit_criteria).map_err(encoding_error)?,
        );
    }
    if !thread.evidence.is_empty() {
        extra_fields.insert(
            "evidence".to_owned(),
            serde_yaml::to_value(&thread.evidence).map_err(encoding_error)?,
        );
    }
    if !thread.update_actions.is_empty() {
        extra_fields.insert(
            "update_actions".to_owned(),
            serde_yaml::to_value(&thread.update_actions).map_err(encoding_error)?,
        );
    }
    if !thread.completion_actions.is_empty() {
        extra_fields.insert(
            "completion_actions".to_owned(),
            serde_yaml::to_value(&thread.completion_actions).map_err(encoding_error)?,
        );
    }

    Ok(StoredPrimitive {
        frontmatter: PrimitiveFrontmatter {
            r#type: THREAD_TYPE.to_owned(),
            id: thread.id.clone(),
            title: thread.title.clone(),
            extra_fields,
        },
        body: crate::render::render_thread_body(thread)?,
    })
}

pub(crate) fn thread_from_primitive(primitive: &StoredPrimitive) -> Result<Thread> {
    if primitive.frontmatter.r#type != THREAD_TYPE {
        return Err(WorkgraphError::ValidationError(format!(
            "expected thread primitive, found '{}'",
            primitive.frontmatter.r#type
        )));
    }

    Ok(ThreadPrimitive {
        id: primitive.frontmatter.id.clone(),
        title: primitive.frontmatter.title.clone(),
        status: primitive
            .frontmatter
            .extra_fields
            .get("status")
            .map_or(Ok(ThreadStatus::Draft), parse_yaml_value)?,
        assigned_actor: primitive
            .frontmatter
            .extra_fields
            .get("assigned_actor")
            .and_then(string_value)
            .map(ActorId::new),
        parent_mission_id: primitive
            .frontmatter
            .extra_fields
            .get("parent_mission_id")
            .and_then(string_value)
            .map(str::to_owned),
        exit_criteria: primitive
            .frontmatter
            .extra_fields
            .get("exit_criteria")
            .map_or(Ok(Vec::new()), parse_yaml_value)?,
        evidence: primitive
            .frontmatter
            .extra_fields
            .get("evidence")
            .map_or(Ok(Vec::new()), parse_yaml_value)?,
        update_actions: primitive
            .frontmatter
            .extra_fields
            .get("update_actions")
            .map_or(Ok(Vec::new()), parse_yaml_value)?,
        completion_actions: primitive
            .frontmatter
            .extra_fields
            .get("completion_actions")
            .map_or(Ok(Vec::new()), parse_yaml_value)?,
        messages: parse_conversation_messages(&primitive.body)?,
    })
}

pub(crate) fn parse_yaml_value<T>(value: &Value) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_yaml::from_value::<T>(value.clone()).map_err(encoding_error)
}

pub(crate) fn string_value(value: &Value) -> Option<&str> {
    match value {
        Value::String(value) => Some(value.as_str()),
        Value::Tagged(tagged) => string_value(&tagged.value),
        Value::Null
        | Value::Bool(_)
        | Value::Number(_)
        | Value::Sequence(_)
        | Value::Mapping(_) => None,
    }
}

pub(crate) fn encoding_error(error: impl std::fmt::Display) -> WorkgraphError {
    WorkgraphError::EncodingError(error.to_string())
}
