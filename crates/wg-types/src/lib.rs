#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Shared serializable data models for WorkGraph primitives, status machines, and registry state.

mod config;
mod coordination;
mod identity;
mod ledger;
mod registry;
mod status;
mod support;
mod tier_one;
mod tier_two;

pub use config::WorkgraphConfig;
pub use coordination::{
    CheckpointPrimitive, ConversationMessage, CoordinationAction, EventEnvelope, EventPattern,
    EventSourceKind, EvidenceItem, GraphEdgeKind, GraphEdgeReference, GraphEdgeSource, LineageMode,
    MessageKind, MissionMilestone, MissionPrimitive, MissionStatus, RunPrimitive,
    ThreadExitCriterion, ThreadPrimitive, TriggerActionOutcome, TriggerActionPlan,
    TriggerPlanDecision, TriggerPrimitive, TriggerReceiptPrimitive, TriggerStatus,
    TriggerSubscriptionState,
};
pub use identity::{ActorId, NodeId, WorkspaceId};
pub use ledger::{LedgerEntry, LedgerOp};
pub use registry::{FieldDefinition, PrimitiveType, Registry};
pub use status::{RunStatus, ThreadStatus};
pub use support::{CachedSnapshot, ExternalRef};
pub use tier_one::{Decision, Lesson, Pattern, Policy, Relationship, StrategicNote};
pub use tier_two::{Agent, Client, Org, Person, Project, Team};
