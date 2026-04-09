//! Coordination, graph, and trigger contracts shared across WorkGraph crates.

use crate::{ActorId, ExternalRef, LedgerOp};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Declares whether an actor lineage is explicitly tracked or intentionally opaque.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LineageMode {
    /// Descendant actors are tracked as explicit first-class entities.
    Tracked,
    /// Descendant actors exist but are intentionally modeled as opaque.
    ///
    /// This preserves durable delegation meaning without forcing every runtime
    /// session, spawned worker, or internal subagent to become a first-class
    /// actor in the graph.
    Opaque,
}

/// Lifecycle of a mission primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MissionStatus {
    /// The mission shell has been created but not yet planned.
    Draft,
    /// The mission plan and milestone threads have been declared.
    Planned,
    /// The mission plan has been approved and can be started.
    Approved,
    /// The mission is actively in progress.
    Active,
    /// The mission is validating completion readiness.
    Validating,
    /// The mission is blocked on a dependency or policy gate.
    Blocked,
    /// The mission has completed successfully.
    Completed,
    /// The mission has been cancelled.
    Cancelled,
}

impl MissionStatus {
    /// Returns the stable serialized value used in frontmatter and JSON.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Planned => "planned",
            Self::Approved => "approved",
            Self::Active => "active",
            Self::Validating => "validating",
            Self::Blocked => "blocked",
            Self::Completed => "completed",
            Self::Cancelled => "cancelled",
        }
    }
}

/// Lifecycle of a trigger primitive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerStatus {
    /// The trigger is defined but not yet active.
    Draft,
    /// The trigger is active and should be evaluated.
    Active,
    /// The trigger is temporarily paused.
    Paused,
    /// The trigger is permanently disabled.
    Disabled,
}

impl TriggerStatus {
    /// Returns the stable serialized value used in frontmatter and JSON.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Disabled => "disabled",
        }
    }
}

/// Human or agent authored message type inside a thread conversation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageKind {
    /// Human-authored message.
    Human,
    /// Agent-authored message.
    Agent,
}

/// Immutable message recorded in a thread conversation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationMessage {
    /// Message timestamp.
    pub ts: DateTime<Utc>,
    /// Message author.
    pub actor: ActorId,
    /// Author kind.
    pub kind: MessageKind,
    /// Message body.
    pub text: String,
}

/// Exit criterion that must be satisfied before a thread may complete.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadExitCriterion {
    /// Stable criterion identifier scoped to the thread.
    pub id: String,
    /// Human-readable criterion title.
    pub title: String,
    /// Optional longer description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether the criterion must be satisfied before completion.
    #[serde(default = "default_true")]
    pub required: bool,
    /// Optional supporting target the criterion is expected to validate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
}

/// Evidence recorded against a thread.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EvidenceItem {
    /// Stable evidence identifier scoped to the thread.
    pub id: String,
    /// Human-readable evidence title.
    pub title: String,
    /// Optional supporting description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Optional reference to the source record or primitive.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// Exit criteria satisfied by this evidence item.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub satisfies: Vec<String>,
    /// Time the evidence was recorded, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub recorded_at: Option<DateTime<Utc>>,
    /// Optional source label such as `manual`, `run`, or `webhook`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
}

/// Durable follow-up action planned by a coordination primitive.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CoordinationAction {
    /// Stable action identifier scoped to the parent primitive.
    pub id: String,
    /// Human-readable action title.
    pub title: String,
    /// Action kind such as `notify`, `rebrief`, or `create_thread`.
    pub kind: String,
    /// Optional target primitive or system reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_reference: Option<String>,
    /// Optional action description or instruction.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Durable thread document model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadPrimitive {
    /// Stable thread identifier.
    pub id: String,
    /// Thread title.
    pub title: String,
    /// Thread lifecycle status.
    pub status: crate::ThreadStatus,
    /// Assigned actor responsible for the thread, when any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_actor: Option<ActorId>,
    /// Parent mission identifier, when scoped under a mission.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_mission_id: Option<String>,
    /// Exit criteria declared for the thread.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exit_criteria: Vec<ThreadExitCriterion>,
    /// Evidence recorded against the thread.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<EvidenceItem>,
    /// Planned actions to take while the thread is still open.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub update_actions: Vec<CoordinationAction>,
    /// Planned actions to take once the thread is complete.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub completion_actions: Vec<CoordinationAction>,
    /// Immutable conversation log.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub messages: Vec<ConversationMessage>,
}

/// Durable mission document model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissionMilestone {
    /// Stable milestone identifier scoped to the mission.
    pub id: String,
    /// Human-readable milestone title.
    pub title: String,
    /// Optional longer description.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Thread identifier auto-created for this milestone.
    pub thread_id: String,
}

/// Durable mission document model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MissionPrimitive {
    /// Stable mission identifier.
    pub id: String,
    /// Mission title.
    pub title: String,
    /// Mission lifecycle status.
    pub status: MissionStatus,
    /// Markdown objective for the mission.
    pub objective: String,
    /// Planned milestones that scope mission execution and thread creation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub milestones: Vec<MissionMilestone>,
    /// Child thread identifiers coordinated by this mission.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub thread_ids: Vec<String>,
    /// Runs related to this mission.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub run_ids: Vec<String>,
    /// Timestamp when the mission was approved.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub approved_at: Option<DateTime<Utc>>,
    /// Timestamp when active execution started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    /// Timestamp when validation started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub validated_at: Option<DateTime<Utc>>,
    /// Timestamp when completion was recorded.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub completed_at: Option<DateTime<Utc>>,
}

/// Durable run document model.
///
/// A run is one bounded execution attempt or work session on behalf of a
/// thread. It captures the durable coordination receipt for that attempt rather
/// than every raw runtime detail.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunPrimitive {
    /// Stable run identifier.
    pub id: String,
    /// Run title.
    pub title: String,
    /// Run lifecycle status.
    pub status: crate::RunStatus,
    /// Optional classification for the kind of bounded work attempt.
    ///
    /// Examples include `agent_pass`, `review`, `approval`, `call`, or
    /// `automation_job`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,
    /// Optional source that created or observed the run receipt.
    ///
    /// Examples include `manual`, `sdk`, `cursor`, `calendar_adapter`, or
    /// `salesforce_adapter`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Logical actor responsible for the run.
    ///
    /// This points at the durable accountable actor boundary rather than a
    /// transient runtime session identifier.
    pub actor_id: ActorId,
    /// Concrete tracked executor that performed the run, when different from
    /// the responsible actor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub executor_id: Option<ActorId>,
    /// Thread this run belongs to.
    pub thread_id: String,
    /// Mission the run contributes to, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mission_id: Option<String>,
    /// Parent run identifier for delegated execution, when any.
    ///
    /// This preserves delegation relationships without requiring WorkGraph to
    /// mirror an external orchestrator's full internal execution tree.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_run_id: Option<String>,
    /// Timestamp when execution started, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<DateTime<Utc>>,
    /// Timestamp when execution ended, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<DateTime<Utc>>,
    /// Optional human-readable summary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Links back to authoritative external records related to this run.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external_refs: Vec<ExternalRef>,
}

/// Event source supported by trigger patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EventSourceKind {
    /// The immutable local ledger stream.
    Ledger,
    /// Externally ingested webhook event.
    Webhook,
    /// Internal system event emitted by WorkGraph subsystems.
    Internal,
}

/// Trigger event matching contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EventPattern {
    /// Event source to match against.
    pub source: EventSourceKind,
    /// Optional stable event name for non-ledger sources.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_name: Option<String>,
    /// Optional ledger operations to match.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub ops: Vec<LedgerOp>,
    /// Optional primitive types in scope.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub primitive_types: Vec<String>,
    /// Optional concrete primitive identifier to match.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub primitive_id: Option<String>,
    /// Optional field names that must appear in the event payload.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub field_names: Vec<String>,
    /// Optional provider or emitter name for webhook/internal sources.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

/// Action plan yielded by a matched trigger.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerActionPlan {
    /// Plan kind such as `create_thread`, `rebrief_actor`, or `emit_signal`.
    pub kind: String,
    /// Optional target primitive or external reference.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_reference: Option<String>,
    /// Durable instruction attached to the plan.
    pub instruction: String,
}

/// Durable trigger document model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerPrimitive {
    /// Stable trigger identifier.
    pub id: String,
    /// Trigger title.
    pub title: String,
    /// Trigger lifecycle status.
    pub status: TriggerStatus,
    /// Event pattern to match.
    pub event_pattern: EventPattern,
    /// Action plans emitted when the pattern matches.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub action_plans: Vec<TriggerActionPlan>,
}

/// Durable checkpoint document model.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CheckpointPrimitive {
    /// Stable checkpoint identifier.
    pub id: String,
    /// Checkpoint title.
    pub title: String,
    /// Current work item.
    pub working_on: String,
    /// Current focus.
    pub focus: String,
    /// Actor owning the checkpoint, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<ActorId>,
    /// Timestamp the checkpoint was created.
    pub created_at: DateTime<Utc>,
}

/// Typed graph edge emitted by the graph builder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphEdgeKind {
    /// Loose content or wiki reference.
    Reference,
    /// Explicit semantic relationship.
    Relationship,
    /// Assignment or ownership edge.
    Assignment,
    /// Mission/thread/run structural containment edge.
    Containment,
    /// Evidence support edge.
    Evidence,
    /// Trigger dependency or action-target edge.
    Trigger,
}

/// Provenance of a typed graph edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GraphEdgeSource {
    /// Edge came from a wiki-link in markdown or frontmatter text.
    WikiLink,
    /// Edge came from an explicit structured field.
    Field,
    /// Edge came from a relationship primitive.
    RelationshipPrimitive,
    /// Edge came from a thread evidence record.
    EvidenceRecord,
    /// Edge came from a trigger definition.
    TriggerRule,
}

/// Serializable edge reference carrying endpoints, kind, and provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphEdgeReference {
    /// Source primitive reference in `type/id` form.
    pub source_reference: String,
    /// Target primitive reference in `type/id` form.
    pub target_reference: String,
    /// Semantic edge kind.
    pub kind: GraphEdgeKind,
    /// Provenance of the edge.
    pub provenance: GraphEdgeSource,
}

const fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::{
        CheckpointPrimitive, ConversationMessage, CoordinationAction, EventPattern,
        EventSourceKind, EvidenceItem, GraphEdgeKind, GraphEdgeReference, GraphEdgeSource,
        LineageMode, MessageKind, MissionMilestone, MissionPrimitive, MissionStatus, RunPrimitive,
        ThreadExitCriterion, ThreadPrimitive, TriggerActionPlan, TriggerPrimitive, TriggerStatus,
    };
    use crate::{ActorId, ExternalRef, RunStatus, ThreadStatus};
    use chrono::{TimeZone, Utc};
    use std::collections::BTreeMap;

    fn roundtrip<T>(value: &T)
    where
        T: serde::Serialize + for<'de> serde::Deserialize<'de> + PartialEq + std::fmt::Debug,
    {
        let json = serde_json::to_string_pretty(value).expect("value should serialize");
        let decoded: T = serde_json::from_str(&json).expect("value should deserialize");
        assert_eq!(&decoded, value);
    }

    #[test]
    fn coordination_models_roundtrip_through_json() {
        let thread = ThreadPrimitive {
            id: "thread-1".into(),
            title: "Validate dealer portal migration".into(),
            status: ThreadStatus::Active,
            assigned_actor: Some(ActorId::new("agent:cursor")),
            parent_mission_id: Some("mission-1".into()),
            exit_criteria: vec![ThreadExitCriterion {
                id: "criterion-1".into(),
                title: "Verification complete".into(),
                description: Some("Need evidence from external verifier".into()),
                required: true,
                reference: Some("project/dealer-portal".into()),
            }],
            evidence: vec![EvidenceItem {
                id: "evidence-1".into(),
                title: "Verifier report".into(),
                description: None,
                reference: Some("decision/deployment-signoff".into()),
                satisfies: vec!["criterion-1".into()],
                recorded_at: Some(
                    Utc.with_ymd_and_hms(2026, 3, 22, 9, 0, 0)
                        .single()
                        .expect("valid timestamp"),
                ),
                source: Some("run".into()),
            }],
            update_actions: vec![CoordinationAction {
                id: "update-1".into(),
                title: "Notify mission owner".into(),
                kind: "notify".into(),
                target_reference: Some("person/pedro".into()),
                description: None,
            }],
            completion_actions: vec![CoordinationAction {
                id: "complete-1".into(),
                title: "Rebrief assignee".into(),
                kind: "rebrief_actor".into(),
                target_reference: Some("agent:cursor".into()),
                description: Some("Send final result summary".into()),
            }],
            messages: vec![ConversationMessage {
                ts: Utc
                    .with_ymd_and_hms(2026, 3, 22, 8, 0, 0)
                    .single()
                    .expect("valid timestamp"),
                actor: ActorId::new("agent:cursor"),
                kind: MessageKind::Agent,
                text: "I am on it.".into(),
            }],
        };
        let mission = MissionPrimitive {
            id: "mission-1".into(),
            title: "Dealer portal launch".into(),
            status: MissionStatus::Active,
            objective: "Ship the migration safely.".into(),
            milestones: vec![MissionMilestone {
                id: "m1".into(),
                title: "External verification".into(),
                description: Some("Gather external signoff evidence".into()),
                thread_id: "mission-1-m1".into(),
            }],
            thread_ids: vec!["thread-1".into()],
            run_ids: vec!["run-1".into()],
            approved_at: Some(
                Utc.with_ymd_and_hms(2026, 3, 22, 7, 45, 0)
                    .single()
                    .expect("valid timestamp"),
            ),
            started_at: Some(
                Utc.with_ymd_and_hms(2026, 3, 22, 8, 10, 0)
                    .single()
                    .expect("valid timestamp"),
            ),
            validated_at: None,
            completed_at: None,
        };
        let run = RunPrimitive {
            id: "run-1".into(),
            title: "Cursor investigation run".into(),
            status: RunStatus::Running,
            kind: Some("agent_pass".into()),
            source: Some("cursor".into()),
            actor_id: ActorId::new("agent:cursor"),
            executor_id: Some(ActorId::new("agent:cursor/subtask")),
            thread_id: "thread-1".into(),
            mission_id: Some("mission-1".into()),
            parent_run_id: None,
            started_at: Some(
                Utc.with_ymd_and_hms(2026, 3, 22, 8, 12, 0)
                    .single()
                    .expect("valid timestamp"),
            ),
            ended_at: None,
            summary: Some("Collecting external verification evidence".into()),
            external_refs: vec![ExternalRef {
                provider: "cursor".into(),
                kind: "session".into(),
                url: "cursor://sessions/abc123".into(),
                id: Some("abc123".into()),
                metadata: BTreeMap::from([("workspace".into(), "workgraph-v4".into())]),
            }],
        };
        let trigger = TriggerPrimitive {
            id: "trigger-1".into(),
            title: "React to completed deployments".into(),
            status: TriggerStatus::Active,
            event_pattern: EventPattern {
                source: EventSourceKind::Ledger,
                event_name: None,
                ops: vec![crate::LedgerOp::Done],
                primitive_types: vec!["thread".into()],
                primitive_id: None,
                field_names: vec!["evidence".into()],
                provider: None,
            },
            action_plans: vec![TriggerActionPlan {
                kind: "rebrief_actor".into(),
                target_reference: Some("agent:cursor".into()),
                instruction: "Refresh the active brief with new evidence.".into(),
            }],
        };
        let checkpoint = CheckpointPrimitive {
            id: "checkpoint-1".into(),
            title: "Checkpoint: Dealer portal".into(),
            working_on: "Dealer portal migration".into(),
            focus: "Evidence collection".into(),
            actor_id: Some(ActorId::new("agent:cursor")),
            created_at: Utc
                .with_ymd_and_hms(2026, 3, 22, 10, 0, 0)
                .single()
                .expect("valid timestamp"),
        };
        let edge = GraphEdgeReference {
            source_reference: "mission/mission-1".into(),
            target_reference: "thread/thread-1".into(),
            kind: GraphEdgeKind::Containment,
            provenance: GraphEdgeSource::Field,
        };

        roundtrip(&thread);
        roundtrip(&mission);
        roundtrip(&run);
        roundtrip(&trigger);
        roundtrip(&checkpoint);
        roundtrip(&edge);
        roundtrip(&LineageMode::Opaque);
    }
}
