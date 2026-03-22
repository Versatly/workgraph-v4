#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Thread lifecycle and evidence-bearing coordination persistence for WorkGraph.

use std::collections::BTreeSet;

use chrono::Utc;
use serde_yaml::Value;
use wg_error::{Result, WorkgraphError};
use wg_paths::WorkspacePath;
use wg_store::{
    PrimitiveFrontmatter, StoredPrimitive, list_primitives, read_primitive, write_primitive,
};
use wg_types::{
    ActorId, ConversationMessage, CoordinationAction, EvidenceItem, MessageKind, Registry,
    ThreadExitCriterion, ThreadPrimitive, ThreadStatus,
};

const THREAD_TYPE: &str = "thread";

/// Typed thread model persisted by this crate.
pub type Thread = ThreadPrimitive;

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

/// Creates and persists a new draft thread.
///
/// # Errors
///
/// Returns an error when required fields are empty or persistence fails.
pub async fn create_thread(
    workspace: &WorkspacePath,
    id: &str,
    title: &str,
    parent_mission_id: Option<&str>,
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

    let thread = ThreadPrimitive {
        id: id.to_owned(),
        title: title.to_owned(),
        status: ThreadStatus::Draft,
        assigned_actor: None,
        parent_mission_id: parent_mission_id.map(str::to_owned),
        exit_criteria: Vec::new(),
        evidence: Vec::new(),
        update_actions: Vec::new(),
        completion_actions: Vec::new(),
        messages: Vec::new(),
    };
    save_thread(workspace, &thread).await?;
    Ok(thread)
}

/// Loads a persisted thread by identifier.
///
/// # Errors
///
/// Returns an error when the stored primitive cannot be loaded or decoded.
pub async fn load_thread(workspace: &WorkspacePath, thread_id: &str) -> Result<Thread> {
    let primitive = read_primitive(workspace, THREAD_TYPE, thread_id).await?;
    thread_from_primitive(&primitive)
}

/// Lists all persisted threads in deterministic path order.
///
/// # Errors
///
/// Returns an error when thread primitives cannot be loaded or decoded.
pub async fn list_threads(workspace: &WorkspacePath) -> Result<Vec<Thread>> {
    list_primitives(workspace, THREAD_TYPE)
        .await?
        .iter()
        .map(thread_from_primitive)
        .collect()
}

/// Transitions a thread from draft or blocked into a ready state.
///
/// # Errors
///
/// Returns an error when the transition is invalid or persistence fails.
pub async fn open_thread(workspace: &WorkspacePath, thread_id: &str) -> Result<Thread> {
    let mut thread = load_thread(workspace, thread_id).await?;
    match thread.status {
        ThreadStatus::Draft | ThreadStatus::Blocked => thread.status = ThreadStatus::Ready,
        ThreadStatus::Ready => {}
        _ => {
            return Err(WorkgraphError::ValidationError(format!(
                "thread '{thread_id}' cannot be opened from status {:?}",
                thread.status
            )));
        }
    }
    save_thread(workspace, &thread).await?;
    Ok(thread)
}

/// Claims a ready or waiting thread for an actor and marks it active.
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
        ThreadStatus::Ready | ThreadStatus::Waiting => {
            if let Some(existing) = &thread.assigned_actor {
                if existing != &actor {
                    return Err(WorkgraphError::ValidationError(format!(
                        "thread '{thread_id}' is already assigned to '{}'",
                        existing
                    )));
                }
            }
            thread.assigned_actor = Some(actor);
            thread.status = ThreadStatus::Active;
        }
        ThreadStatus::Active => {
            if thread.assigned_actor.as_ref() == Some(&actor) {
                return Ok(thread);
            }
            return Err(WorkgraphError::ValidationError(format!(
                "thread '{thread_id}' is already assigned to '{}'",
                thread
                    .assigned_actor
                    .as_ref()
                    .map_or("unknown", ActorId::as_str)
            )));
        }
        _ => {
            return Err(WorkgraphError::ValidationError(format!(
                "thread '{thread_id}' cannot be claimed from status {:?}",
                thread.status
            )));
        }
    }

    save_thread(workspace, &thread).await?;
    Ok(thread)
}

/// Appends a structured exit criterion to a thread.
///
/// # Errors
///
/// Returns an error when the criterion identifier is empty, duplicates an
/// existing criterion, or persistence fails.
pub async fn add_exit_criterion(
    workspace: &WorkspacePath,
    thread_id: &str,
    criterion: ThreadExitCriterion,
) -> Result<Thread> {
    if criterion.id.trim().is_empty() {
        return Err(WorkgraphError::ValidationError(
            "thread exit criterion id must not be empty".to_owned(),
        ));
    }
    let mut thread = load_thread(workspace, thread_id).await?;
    if thread
        .exit_criteria
        .iter()
        .any(|existing| existing.id == criterion.id)
    {
        return Err(WorkgraphError::ValidationError(format!(
            "thread '{thread_id}' already contains exit criterion '{}'",
            criterion.id
        )));
    }
    thread.exit_criteria.push(criterion);
    save_thread(workspace, &thread).await?;
    Ok(thread)
}

/// Records evidence against a thread.
///
/// # Errors
///
/// Returns an error when the evidence is invalid or persistence fails.
pub async fn add_evidence(
    workspace: &WorkspacePath,
    thread_id: &str,
    mut evidence: EvidenceItem,
) -> Result<Thread> {
    if evidence.id.trim().is_empty() {
        return Err(WorkgraphError::ValidationError(
            "thread evidence id must not be empty".to_owned(),
        ));
    }
    let mut thread = load_thread(workspace, thread_id).await?;
    if thread
        .evidence
        .iter()
        .any(|existing| existing.id == evidence.id)
    {
        return Err(WorkgraphError::ValidationError(format!(
            "thread '{thread_id}' already contains evidence '{}'",
            evidence.id
        )));
    }
    let known_criteria = thread
        .exit_criteria
        .iter()
        .map(|criterion| criterion.id.as_str())
        .collect::<BTreeSet<_>>();
    for criterion_id in &evidence.satisfies {
        if !known_criteria.contains(criterion_id.as_str()) {
            return Err(WorkgraphError::ValidationError(format!(
                "thread '{thread_id}' evidence '{}' references unknown criterion '{}'",
                evidence.id, criterion_id
            )));
        }
    }
    if evidence.recorded_at.is_none() {
        evidence.recorded_at = Some(Utc::now());
    }
    thread.evidence.push(evidence);
    save_thread(workspace, &thread).await?;
    Ok(thread)
}

/// Appends a planned update action to a thread.
///
/// # Errors
///
/// Returns an error when the action identifier is invalid or persistence fails.
pub async fn add_update_action(
    workspace: &WorkspacePath,
    thread_id: &str,
    action: CoordinationAction,
) -> Result<Thread> {
    add_action(workspace, thread_id, action, ActionList::Update).await
}

/// Appends a planned completion action to a thread.
///
/// # Errors
///
/// Returns an error when the action identifier is invalid or persistence fails.
pub async fn add_completion_action(
    workspace: &WorkspacePath,
    thread_id: &str,
    action: CoordinationAction,
) -> Result<Thread> {
    add_action(workspace, thread_id, action, ActionList::Completion).await
}

/// Returns unsatisfied required exit criterion identifiers for the thread.
#[must_use]
pub fn unsatisfied_exit_criteria(thread: &Thread) -> Vec<String> {
    let satisfied = thread
        .evidence
        .iter()
        .flat_map(|item| item.satisfies.iter().cloned())
        .collect::<BTreeSet<_>>();
    thread
        .exit_criteria
        .iter()
        .filter(|criterion| criterion.required && !satisfied.contains(&criterion.id))
        .map(|criterion| criterion.id.clone())
        .collect()
}

/// Marks a thread as done once every required exit criterion is satisfied.
///
/// # Errors
///
/// Returns an error when required criteria remain unsatisfied, the transition is
/// invalid, or persistence fails.
pub async fn complete_thread(workspace: &WorkspacePath, thread_id: &str) -> Result<Thread> {
    let mut thread = load_thread(workspace, thread_id).await?;
    let missing = unsatisfied_exit_criteria(&thread);
    if !missing.is_empty() {
        return Err(WorkgraphError::ValidationError(format!(
            "thread '{thread_id}' cannot complete; unsatisfied exit criteria: {}",
            missing.join(", ")
        )));
    }

    match thread.status {
        ThreadStatus::Active
        | ThreadStatus::Waiting
        | ThreadStatus::Blocked
        | ThreadStatus::Done => {
            thread.status = ThreadStatus::Done;
        }
        _ => {
            return Err(WorkgraphError::ValidationError(format!(
                "thread '{thread_id}' cannot be completed from status {:?}",
                thread.status
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
    if matches!(thread.status, ThreadStatus::Done | ThreadStatus::Cancelled) {
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

async fn add_action(
    workspace: &WorkspacePath,
    thread_id: &str,
    action: CoordinationAction,
    list: ActionList,
) -> Result<Thread> {
    if action.id.trim().is_empty() {
        return Err(WorkgraphError::ValidationError(
            "coordination action id must not be empty".to_owned(),
        ));
    }
    let mut thread = load_thread(workspace, thread_id).await?;
    let actions = match list {
        ActionList::Update => &mut thread.update_actions,
        ActionList::Completion => &mut thread.completion_actions,
    };
    if actions.iter().any(|existing| existing.id == action.id) {
        return Err(WorkgraphError::ValidationError(format!(
            "thread '{thread_id}' already contains action '{}'",
            action.id
        )));
    }
    actions.push(action);
    save_thread(workspace, &thread).await?;
    Ok(thread)
}

async fn save_thread(workspace: &WorkspacePath, thread: &Thread) -> Result<()> {
    let primitive = thread_to_primitive(thread)?;
    write_primitive(workspace, &Registry::builtins(), &primitive).await?;
    Ok(())
}

fn thread_to_primitive(thread: &Thread) -> Result<StoredPrimitive> {
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
        body: render_thread_body(thread)?,
    })
}

fn thread_from_primitive(primitive: &StoredPrimitive) -> Result<Thread> {
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

fn render_thread_body(thread: &Thread) -> Result<String> {
    let mut rendered = String::new();

    rendered.push_str("## Exit Criteria\n");
    if thread.exit_criteria.is_empty() {
        rendered.push_str("None recorded.\n");
    } else {
        for criterion in &thread.exit_criteria {
            let required = if criterion.required {
                "required"
            } else {
                "optional"
            };
            rendered.push_str(&format!(
                "- {} [{}] ({})\n",
                criterion.title, criterion.id, required
            ));
            if let Some(description) = &criterion.description {
                rendered.push_str(&format!("  {}\n", description.trim()));
            }
            if let Some(reference) = &criterion.reference {
                rendered.push_str(&format!("  reference: {}\n", reference));
            }
        }
    }

    rendered.push_str("\n## Evidence\n");
    if thread.evidence.is_empty() {
        rendered.push_str("None recorded.\n");
    } else {
        for evidence in &thread.evidence {
            rendered.push_str(&format!("- {} ({})\n", evidence.title, evidence.id));
            if !evidence.satisfies.is_empty() {
                rendered.push_str(&format!("  satisfies: {}\n", evidence.satisfies.join(", ")));
            }
            if let Some(reference) = &evidence.reference {
                rendered.push_str(&format!("  reference: {}\n", reference));
            }
            if let Some(source) = &evidence.source {
                rendered.push_str(&format!("  source: {}\n", source));
            }
        }
    }

    rendered.push_str("\n## Update Actions\n");
    render_actions(&mut rendered, &thread.update_actions);
    rendered.push_str("\n## Completion Actions\n");
    render_actions(&mut rendered, &thread.completion_actions);

    let yaml = serde_yaml::to_string(&thread.messages).map_err(encoding_error)?;
    let yaml = yaml
        .strip_prefix("---\n")
        .or_else(|| yaml.strip_prefix("---\r\n"))
        .unwrap_or(yaml.as_str());
    let trailing_newline = if yaml.ends_with('\n') { "" } else { "\n" };
    rendered.push_str("\n## Conversation\n\n```yaml\n");
    rendered.push_str(yaml);
    rendered.push_str(trailing_newline);
    rendered.push_str("```\n");

    Ok(rendered)
}

fn render_actions(rendered: &mut String, actions: &[CoordinationAction]) {
    if actions.is_empty() {
        rendered.push_str("None planned.\n");
        return;
    }

    for action in actions {
        rendered.push_str(&format!(
            "- {} ({}) [{}]\n",
            action.title, action.id, action.kind
        ));
        if let Some(target_reference) = &action.target_reference {
            rendered.push_str(&format!("  target: {}\n", target_reference));
        }
        if let Some(description) = &action.description {
            rendered.push_str(&format!("  {}\n", description.trim()));
        }
    }
}

fn parse_conversation_messages(body: &str) -> Result<Vec<ConversationMessage>> {
    let Some(opening) = body.find("```yaml") else {
        return Ok(Vec::new());
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

    serde_yaml::from_str::<Vec<ConversationMessage>>(yaml).map_err(encoding_error)
}

fn parse_yaml_value<T>(value: &Value) -> Result<T>
where
    T: serde::de::DeserializeOwned,
{
    serde_yaml::from_value::<T>(value.clone()).map_err(encoding_error)
}

fn string_value(value: &Value) -> Option<&str> {
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

fn infer_message_kind(actor: &ActorId) -> MessageKind {
    let value = actor.as_str();
    if value.starts_with("agent:") || value.starts_with("agent/") {
        MessageKind::Agent
    } else {
        MessageKind::Human
    }
}

fn encoding_error(error: impl std::fmt::Display) -> WorkgraphError {
    WorkgraphError::EncodingError(error.to_string())
}

enum ActionList {
    Update,
    Completion,
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use wg_paths::WorkspacePath;
    use wg_store::read_primitive;
    use wg_types::{ActorId, CoordinationAction, EvidenceItem, ThreadExitCriterion, ThreadStatus};

    use crate::{
        add_completion_action, add_evidence, add_exit_criterion, add_message, add_update_action,
        claim_thread, complete_thread, create_thread, load_thread, open_thread,
        unsatisfied_exit_criteria,
    };

    #[tokio::test]
    async fn thread_lifecycle_persists_status_transitions() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());

        create_thread(
            &workspace,
            "coord-1",
            "Coordinate launch",
            Some("mission-a"),
        )
        .await
        .expect("thread should be created");
        let opened = open_thread(&workspace, "coord-1")
            .await
            .expect("thread should open");
        assert_eq!(opened.status, ThreadStatus::Ready);

        let claimed = claim_thread(&workspace, "coord-1", ActorId::new("pedro"))
            .await
            .expect("thread should be claimed");
        assert_eq!(claimed.status, ThreadStatus::Active);
        assert_eq!(
            claimed.assigned_actor.as_ref().map(ActorId::as_str),
            Some("pedro")
        );

        let stored = read_primitive(&workspace, "thread", "coord-1")
            .await
            .expect("thread primitive should be readable");
        assert_eq!(
            stored
                .frontmatter
                .extra_fields
                .get("status")
                .expect("status field should be present"),
            &serde_yaml::to_value(ThreadStatus::Active).expect("status should serialize"),
        );
    }

    #[tokio::test]
    async fn completion_requires_evidence_for_required_criteria() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());

        create_thread(&workspace, "coord-2", "Evidence-bearing thread", None)
            .await
            .expect("thread should be created");
        open_thread(&workspace, "coord-2")
            .await
            .expect("thread should open");
        claim_thread(&workspace, "coord-2", ActorId::new("agent:cursor"))
            .await
            .expect("thread should be claimed");
        add_exit_criterion(
            &workspace,
            "coord-2",
            ThreadExitCriterion {
                id: "criterion-1".into(),
                title: "Verification complete".into(),
                description: None,
                required: true,
                reference: Some("project/dealer-portal".into()),
            },
        )
        .await
        .expect("criterion should be added");

        let error = complete_thread(&workspace, "coord-2")
            .await
            .expect_err("thread should reject completion without evidence");
        assert!(error.to_string().contains("criterion-1"));

        add_evidence(
            &workspace,
            "coord-2",
            EvidenceItem {
                id: "evidence-1".into(),
                title: "Verifier output".into(),
                description: None,
                reference: Some("decision/signoff".into()),
                satisfies: vec!["criterion-1".into()],
                recorded_at: None,
                source: Some("run".into()),
            },
        )
        .await
        .expect("evidence should be added");

        let completed = complete_thread(&workspace, "coord-2")
            .await
            .expect("thread should complete once evidence is present");
        assert_eq!(completed.status, ThreadStatus::Done);
        assert!(unsatisfied_exit_criteria(&completed).is_empty());
    }

    #[tokio::test]
    async fn add_message_serializes_human_readable_sections_and_conversation_log() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());

        create_thread(&workspace, "coord-3", "Conversation thread", None)
            .await
            .expect("thread should be created");
        open_thread(&workspace, "coord-3")
            .await
            .expect("thread should open");
        add_update_action(
            &workspace,
            "coord-3",
            CoordinationAction {
                id: "update-1".into(),
                title: "Notify owner".into(),
                kind: "notify".into(),
                target_reference: Some("person/pedro".into()),
                description: None,
            },
        )
        .await
        .expect("update action should be added");
        add_completion_action(
            &workspace,
            "coord-3",
            CoordinationAction {
                id: "complete-1".into(),
                title: "Create follow-up".into(),
                kind: "create_thread".into(),
                target_reference: Some("project/dealer-portal".into()),
                description: Some("Open a verification follow-up thread".into()),
            },
        )
        .await
        .expect("completion action should be added");
        add_message(
            &workspace,
            "coord-3",
            ActorId::new("agent:cursor"),
            "Investigating now.",
        )
        .await
        .expect("message should append");

        let stored = read_primitive(&workspace, "thread", "coord-3")
            .await
            .expect("thread primitive should be readable");
        assert!(stored.body.contains("## Exit Criteria"));
        assert!(stored.body.contains("## Update Actions"));
        assert!(stored.body.contains("## Completion Actions"));
        assert!(stored.body.contains("## Conversation"));
        assert!(stored.body.contains("Investigating now."));

        let loaded = load_thread(&workspace, "coord-3")
            .await
            .expect("thread should roundtrip");
        assert_eq!(loaded.messages.len(), 1);
    }

    #[tokio::test]
    async fn terminal_threads_reject_new_messages() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let workspace = WorkspacePath::new(temp_dir.path());

        create_thread(&workspace, "coord-4", "Terminal thread", None)
            .await
            .expect("thread should be created");
        open_thread(&workspace, "coord-4")
            .await
            .expect("thread should open");
        claim_thread(&workspace, "coord-4", ActorId::new("pedro"))
            .await
            .expect("thread should be claimed");
        complete_thread(&workspace, "coord-4")
            .await
            .expect("thread should complete without required criteria");

        let error = add_message(&workspace, "coord-4", ActorId::new("pedro"), "Should fail")
            .await
            .expect_err("completed thread should reject messages");
        assert!(error.to_string().contains("terminal"));
    }
}
