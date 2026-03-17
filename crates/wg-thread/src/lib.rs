#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Thread lifecycle and conversation persistence for WorkGraph.
//!
//! Threads are stored as `thread` primitives using `wg-store`. Lifecycle
//! metadata is saved in YAML frontmatter while the conversation log is kept in
//! markdown body as a YAML code block.

use std::collections::BTreeMap;
use std::str::FromStr;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use wg_error::{Result, WorkgraphError};
use wg_paths::WorkspacePath;
use wg_store::{PrimitiveFrontmatter, StoredPrimitive, read_primitive, write_primitive};
use wg_types::{ActorId, Registry};

const THREAD_TYPE: &str = "thread";

/// Identifies a thread in compatibility APIs used by placeholder crates.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ThreadId(pub String);

impl ThreadId {
    /// Creates a new thread identifier.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

/// Simplified compatibility lifecycle state used by placeholder crates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThreadState {
    /// The thread exists but has not started execution.
    #[default]
    Draft,
    /// The thread is currently active.
    Active,
    /// The thread is no longer accepting new work.
    Closed,
}

/// Compatibility handle for placeholder crates.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ThreadHandle {
    /// Stable identifier for the thread.
    pub id: ThreadId,
    /// Current lifecycle state.
    pub state: ThreadState,
}

impl ThreadHandle {
    /// Returns a copy of the handle marked active.
    #[must_use]
    pub fn activate(mut self) -> Self {
        self.state = ThreadState::Active;
        self
    }
}

/// Status for thread lifecycle orchestration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ThreadStatus {
    /// Thread is being drafted and not yet open for claiming.
    Draft,
    /// Thread is open for assignment.
    Open,
    /// Thread has an explicit assignee.
    Claimed,
    /// Thread is blocked by dependency or policy gate.
    Blocked,
    /// Thread was completed.
    Completed,
    /// Thread was cancelled.
    Cancelled,
}

impl ThreadStatus {
    /// Returns the storage string used in frontmatter.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Open => "open",
            Self::Claimed => "claimed",
            Self::Blocked => "blocked",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl FromStr for ThreadStatus {
    type Err = String;

    fn from_str(input: &str) -> std::result::Result<Self, Self::Err> {
        match input {
            "draft" => Ok(Self::Draft),
            "open" => Ok(Self::Open),
            "claimed" => Ok(Self::Claimed),
            "blocked" => Ok(Self::Blocked),
            "completed" => Ok(Self::Completed),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(format!("unsupported thread status '{input}'")),
        }
    }
}

/// Message author class for conversation entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    /// Human-authored message.
    Human,
    /// Agent-authored message.
    Agent,
}

/// One immutable conversation entry in a thread log.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationMessage {
    /// Message timestamp.
    pub ts: DateTime<Utc>,
    /// Author identifier.
    pub actor: ActorId,
    /// Message author kind.
    pub kind: MessageKind,
    /// Message payload.
    pub text: String,
}

/// Domain model for a persisted thread primitive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Thread {
    /// Stable thread identifier.
    pub id: String,
    /// Human-readable thread title.
    pub title: String,
    /// Current thread status.
    pub status: ThreadStatus,
    /// Currently assigned actor, when claimed.
    pub assigned_actor: Option<ActorId>,
    /// Optional parent mission identifier.
    pub parent_mission: Option<String>,
    /// Ordered conversation log.
    pub messages: Vec<ConversationMessage>,
}

/// Creates and persists a new draft thread.
///
/// # Errors
///
/// Returns an error when required fields are empty or persistence fails.
pub async fn create_thread(
    workspace: &WorkspacePath,
    id: &str,
    title: &str,
    parent_mission: Option<&str>,
) -> Result<Thread> {
    if id.trim().is_empty() {
        return Err(WorkgraphError::ValidationError(
            "thread id must not be empty".to_owned(),
        ));
    }
    if title.trim().is_empty() {
        return Err(WorkgraphError::ValidationError(
            "thread title must not be empty".to_owned(),
        ));
    }

    let thread = Thread {
        id: id.to_owned(),
        title: title.to_owned(),
        status: ThreadStatus::Draft,
        assigned_actor: None,
        parent_mission: parent_mission.map(str::to_owned),
        messages: Vec::new(),
    };
    save_thread(workspace, &thread).await?;
    Ok(thread)
}

/// Transitions a thread from draft/blocked to open.
///
/// # Errors
///
/// Returns an error when the transition is invalid or persistence fails.
pub async fn open_thread(workspace: &WorkspacePath, thread_id: &str) -> Result<Thread> {
    let mut thread = load_thread(workspace, thread_id).await?;
    match thread.status {
        ThreadStatus::Draft | ThreadStatus::Blocked => thread.status = ThreadStatus::Open,
        ThreadStatus::Open => {}
        _ => {
            return Err(WorkgraphError::ValidationError(format!(
                "thread '{thread_id}' cannot be opened from status '{}'",
                thread.status.as_str()
            )));
        }
    }
    save_thread(workspace, &thread).await?;
    Ok(thread)
}

/// Claims an open thread for an actor.
///
/// # Errors
///
/// Returns an error when the thread is in an incompatible status, is already
/// claimed by another actor, or persistence fails.
pub async fn claim_thread(
    workspace: &WorkspacePath,
    thread_id: &str,
    actor: ActorId,
) -> Result<Thread> {
    let mut thread = load_thread(workspace, thread_id).await?;

    match thread.status {
        ThreadStatus::Open => {
            thread.status = ThreadStatus::Claimed;
            thread.assigned_actor = Some(actor);
        }
        ThreadStatus::Claimed => {
            if thread.assigned_actor.as_ref() == Some(&actor) {
                return Ok(thread);
            }
            return Err(WorkgraphError::ValidationError(format!(
                "thread '{thread_id}' is already claimed by '{}'",
                thread
                    .assigned_actor
                    .as_ref()
                    .map_or("unknown", ActorId::as_str)
            )));
        }
        _ => {
            return Err(WorkgraphError::ValidationError(format!(
                "thread '{thread_id}' cannot be claimed from status '{}'",
                thread.status.as_str()
            )));
        }
    }

    save_thread(workspace, &thread).await?;
    Ok(thread)
}

/// Marks a thread as completed.
///
/// # Errors
///
/// Returns an error when the transition is invalid or persistence fails.
pub async fn complete_thread(workspace: &WorkspacePath, thread_id: &str) -> Result<Thread> {
    let mut thread = load_thread(workspace, thread_id).await?;
    match thread.status {
        ThreadStatus::Open | ThreadStatus::Claimed | ThreadStatus::Blocked => {
            thread.status = ThreadStatus::Completed;
        }
        ThreadStatus::Completed => {}
        _ => {
            return Err(WorkgraphError::ValidationError(format!(
                "thread '{thread_id}' cannot be completed from status '{}'",
                thread.status.as_str()
            )));
        }
    }

    save_thread(workspace, &thread).await?;
    Ok(thread)
}

/// Appends a conversation message to a thread.
///
/// # Errors
///
/// Returns an error when the text is empty, the thread is terminal, or
/// persistence fails.
pub async fn add_message(
    workspace: &WorkspacePath,
    thread_id: &str,
    actor: ActorId,
    text: &str,
) -> Result<Thread> {
    if text.trim().is_empty() {
        return Err(WorkgraphError::ValidationError(
            "thread messages must not be empty".to_owned(),
        ));
    }

    let mut thread = load_thread(workspace, thread_id).await?;
    if matches!(thread.status, ThreadStatus::Completed | ThreadStatus::Cancelled) {
        return Err(WorkgraphError::ValidationError(format!(
            "thread '{thread_id}' is terminal and cannot receive new messages"
        )));
    }

    thread.messages.push(ConversationMessage {
        ts: Utc::now(),
        kind: infer_message_kind(&actor),
        actor,
        text: text.to_owned(),
    });

    save_thread(workspace, &thread).await?;
    Ok(thread)
}

async fn load_thread(workspace: &WorkspacePath, thread_id: &str) -> Result<Thread> {
    let primitive = read_primitive(workspace, THREAD_TYPE, thread_id).await?;
    thread_from_primitive(&primitive)
}

async fn save_thread(workspace: &WorkspacePath, thread: &Thread) -> Result<()> {
    let primitive = thread_to_primitive(thread)?;
    write_primitive(workspace, &Registry::builtins(), &primitive).await?;
    Ok(())
}

fn thread_from_primitive(primitive: &StoredPrimitive) -> Result<Thread> {
    if primitive.frontmatter.r#type != THREAD_TYPE {
        return Err(WorkgraphError::ValidationError(format!(
            "expected thread primitive, found '{}'",
            primitive.frontmatter.r#type
        )));
    }

    let status = primitive
        .frontmatter
        .extra_fields
        .get("status")
        .and_then(string_value)
        .map_or(Ok(ThreadStatus::Draft), |value| {
            ThreadStatus::from_str(value).map_err(WorkgraphError::ValidationError)
        })?;

    Ok(Thread {
        id: primitive.frontmatter.id.clone(),
        title: primitive.frontmatter.title.clone(),
        status,
        assigned_actor: primitive
            .frontmatter
            .extra_fields
            .get("assigned_actor")
            .and_then(string_value)
            .map(ActorId::new),
        parent_mission: primitive
            .frontmatter
            .extra_fields
            .get("parent_mission")
            .and_then(string_value)
            .map(str::to_owned),
        messages: parse_conversation_body(&primitive.body)?,
    })
}

fn thread_to_primitive(thread: &Thread) -> Result<StoredPrimitive> {
    let mut extra_fields = BTreeMap::new();
    extra_fields.insert(
        "status".to_owned(),
        Value::String(thread.status.as_str().to_owned()),
    );
    if let Some(actor) = &thread.assigned_actor {
        extra_fields.insert("assigned_actor".to_owned(), Value::String(actor.to_string()));
    }
    if let Some(parent_mission) = &thread.parent_mission {
        extra_fields.insert(
            "parent_mission".to_owned(),
            Value::String(parent_mission.clone()),
        );
    }

    Ok(StoredPrimitive {
        frontmatter: PrimitiveFrontmatter {
            r#type: THREAD_TYPE.to_owned(),
            id: thread.id.clone(),
            title: thread.title.clone(),
            extra_fields,
        },
        body: render_conversation_body(&thread.messages)?,
    })
}

fn parse_conversation_body(body: &str) -> Result<Vec<ConversationMessage>> {
    if body.trim().is_empty() {
        return Ok(Vec::new());
    }

    let Some(opening) = body.find("```yaml") else {
        return Err(WorkgraphError::ValidationError(
            "thread conversation body must include a ```yaml code fence".to_owned(),
        ));
    };

    let after_opening = &body[opening + "```yaml".len()..];
    let after_newline = after_opening.strip_prefix('\n').unwrap_or(after_opening);
    let Some(closing) = after_newline.find("\n```") else {
        return Err(WorkgraphError::ValidationError(
            "thread conversation body is missing closing ``` fence".to_owned(),
        ));
    };
    let yaml = &after_newline[..closing];

    if yaml.trim().is_empty() {
        return Ok(Vec::new());
    }

    serde_yaml::from_str::<Vec<ConversationMessage>>(yaml).map_err(|error| {
        WorkgraphError::EncodingError(format!("failed to parse thread conversation body: {error}"))
    })
}

fn render_conversation_body(messages: &[ConversationMessage]) -> Result<String> {
    let yaml = serde_yaml::to_string(messages).map_err(|error| {
        WorkgraphError::EncodingError(format!(
            "failed to serialize thread conversation messages: {error}"
        ))
    })?;
    let yaml = yaml
        .strip_prefix("---\n")
        .or_else(|| yaml.strip_prefix("---\r\n"))
        .unwrap_or(yaml.as_str());
    let trailing_newline = if yaml.ends_with('\n') { "" } else { "\n" };

    Ok(format!(
        "## Conversation\n\n```yaml\n{yaml}{trailing_newline}```\n"
    ))
}

fn infer_message_kind(actor: &ActorId) -> MessageKind {
    let value = actor.as_str();
    if value.starts_with("agent:") || value.starts_with("agent/") {
        MessageKind::Agent
    } else {
        MessageKind::Human
    }
}

fn string_value(value: &Value) -> Option<&str> {
    match value {
        Value::String(value) => Some(value.as_str()),
        Value::Tagged(tagged) => string_value(&tagged.value),
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::Sequence(_) | Value::Mapping(_) => {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use wg_paths::WorkspacePath;
    use wg_store::read_primitive;
    use wg_types::ActorId;

    use crate::{
        MessageKind, ThreadStatus, add_message, claim_thread, complete_thread, create_thread,
        open_thread,
    };

    #[tokio::test]
    async fn thread_lifecycle_persists_status_transitions() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());

        create_thread(&workspace, "coord-1", "Coordinate launch", Some("mission-a"))
            .await
            .expect("thread should be created");
        let opened = open_thread(&workspace, "coord-1")
            .await
            .expect("thread should open");
        assert_eq!(opened.status, ThreadStatus::Open);

        let claimed = claim_thread(&workspace, "coord-1", ActorId::new("pedro"))
            .await
            .expect("thread should be claimed");
        assert_eq!(claimed.status, ThreadStatus::Claimed);
        assert_eq!(claimed.assigned_actor.as_ref().map(ActorId::as_str), Some("pedro"));

        let completed = complete_thread(&workspace, "coord-1")
            .await
            .expect("thread should complete");
        assert_eq!(completed.status, ThreadStatus::Completed);

        let stored = read_primitive(&workspace, "thread", "coord-1")
            .await
            .expect("thread primitive should be readable");
        assert_eq!(
            stored
                .frontmatter
                .extra_fields
                .get("status")
                .expect("status field should be present"),
            &serde_yaml::Value::String("completed".to_owned())
        );
    }

    #[tokio::test]
    async fn claim_rejects_competing_actor() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());

        create_thread(&workspace, "coord-2", "Coordinate support", None)
            .await
            .expect("thread should be created");
        open_thread(&workspace, "coord-2")
            .await
            .expect("thread should open");
        claim_thread(&workspace, "coord-2", ActorId::new("pedro"))
            .await
            .expect("first claim should succeed");

        let error = claim_thread(&workspace, "coord-2", ActorId::new("ana"))
            .await
            .expect_err("second claim should fail");
        assert_eq!(error.code(), "validation_error");
        assert!(error.to_string().contains("already claimed"));
    }

    #[tokio::test]
    async fn add_message_serializes_conversation_log_in_markdown_body() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());

        create_thread(&workspace, "coord-3", "Coordinate migration", None)
            .await
            .expect("thread should be created");
        open_thread(&workspace, "coord-3")
            .await
            .expect("thread should open");

        let updated = add_message(
            &workspace,
            "coord-3",
            ActorId::new("agent:cursor"),
            "I will take the migration checklist.",
        )
        .await
        .expect("message should be added");
        assert_eq!(updated.messages.len(), 1);
        assert_eq!(updated.messages[0].kind, MessageKind::Agent);

        let stored = read_primitive(&workspace, "thread", "coord-3")
            .await
            .expect("thread primitive should be readable");
        assert!(stored.body.contains("## Conversation"));
        assert!(stored.body.contains("```yaml"));
        assert!(stored.body.contains("I will take the migration checklist."));
    }

    #[tokio::test]
    async fn terminal_threads_reject_new_messages() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());

        create_thread(&workspace, "coord-4", "Coordinate QA", None)
            .await
            .expect("thread should be created");
        open_thread(&workspace, "coord-4")
            .await
            .expect("thread should open");
        complete_thread(&workspace, "coord-4")
            .await
            .expect("thread should complete");

        let error = add_message(&workspace, "coord-4", ActorId::new("pedro"), "hello")
            .await
            .expect_err("completed thread should reject messages");
        assert_eq!(error.code(), "validation_error");
        assert!(error.to_string().contains("terminal"));
    }
}
