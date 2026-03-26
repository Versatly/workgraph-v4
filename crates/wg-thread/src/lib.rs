#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Thread lifecycle and evidence-bearing coordination persistence for WorkGraph.

use std::collections::BTreeSet;

use wg_error::Result;
use wg_paths::WorkspacePath;
use wg_store::{AuditedWriteRequest, list_primitives, read_primitive};
use wg_types::{ActorId, CoordinationAction, EvidenceItem, ThreadExitCriterion, ThreadPrimitive};

mod codec;
mod mutation;
mod mutation_support;
mod render;

const THREAD_TYPE: &str = "thread";
const SYSTEM_ACTOR: &str = "system:workgraph";

/// Typed thread model persisted by this crate.
pub type Thread = ThreadPrimitive;

use codec::{thread_from_primitive, thread_to_primitive};

pub use mutation::ThreadMutationService;

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
    ThreadMutationService::new(workspace)
        .create_thread(id, title, parent_mission_id)
        .await
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
    ThreadMutationService::new(workspace)
        .open_thread(thread_id)
        .await
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
    ThreadMutationService::new(workspace)
        .claim_thread(thread_id, actor)
        .await
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
    ThreadMutationService::new(workspace)
        .add_exit_criterion(thread_id, criterion)
        .await
}

/// Records evidence against a thread.
///
/// # Errors
///
/// Returns an error when the evidence is invalid or persistence fails.
pub async fn add_evidence(
    workspace: &WorkspacePath,
    thread_id: &str,
    evidence: EvidenceItem,
) -> Result<Thread> {
    ThreadMutationService::new(workspace)
        .add_evidence(thread_id, evidence)
        .await
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
    ThreadMutationService::new(workspace)
        .add_update_action(thread_id, action)
        .await
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
    ThreadMutationService::new(workspace)
        .add_completion_action(thread_id, action)
        .await
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
    ThreadMutationService::new(workspace)
        .complete_thread(thread_id)
        .await
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
    ThreadMutationService::new(workspace)
        .add_message(thread_id, actor, text)
        .await
}

async fn save_thread_with_audit(
    workspace: &WorkspacePath,
    thread: &Thread,
    audit: AuditedWriteRequest,
) -> Result<()> {
    let primitive = thread_to_primitive(thread)?;
    wg_store::write_primitive_audited_now(
        workspace,
        &wg_types::Registry::builtins(),
        &primitive,
        audit,
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use wg_ledger::LedgerReader;
    use wg_paths::WorkspacePath;
    use wg_store::read_primitive;
    use wg_types::LedgerOp;
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

        let (entries, _) = LedgerReader::new(temp_dir.path().to_path_buf())
            .read_from(Default::default())
            .await
            .expect("ledger should be readable");
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].op, LedgerOp::Create);
        assert_eq!(entries[1].op, LedgerOp::Reopen);
        assert_eq!(entries[2].op, LedgerOp::Claim);
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
