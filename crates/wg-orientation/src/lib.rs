#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Shared orientation models for agent and human entry into a WorkGraph workspace.

use std::collections::BTreeMap;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

mod brief_runtime;
mod mutation;
mod runtime;
mod runtime_support;
mod status_runtime;

pub use mutation::CheckpointMutationService;
pub use runtime::{ActorBrief, WorkspaceStatus, brief, checkpoint, status};

/// The perspective or slice of context requested for a workspace brief.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum ContextLens {
    /// A broad workspace orientation across key entity and knowledge types.
    #[default]
    Workspace,
    /// A lens emphasizing active delivery context such as clients and projects.
    Delivery,
    /// A lens emphasizing governance, policy, patterns, and lessons.
    Policy,
    /// A lens emphasizing agents and collaboration surfaces.
    Agents,
}

impl ContextLens {
    /// Returns the stable string identifier used in CLI and serialized outputs.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Workspace => "workspace",
            Self::Delivery => "delivery",
            Self::Policy => "policy",
            Self::Agents => "agents",
        }
    }
}

impl FromStr for ContextLens {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "workspace" => Ok(Self::Workspace),
            "delivery" => Ok(Self::Delivery),
            "policy" => Ok(Self::Policy),
            "agents" => Ok(Self::Agents),
            _ => Err(format!(
                "unsupported context lens '{input}'; expected one of workspace, delivery, policy, agents"
            )),
        }
    }
}

/// A single brief item surfaced to a human or agent during orientation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BriefItem {
    /// The item category, such as `org`, `client`, `project`, or `decision`.
    pub kind: String,
    /// A stable reference for the item, when one exists.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference: Option<String>,
    /// The human-readable title or label.
    pub title: String,
    /// Additional supporting detail or summary text.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// A grouped section inside a workspace brief.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BriefSection {
    /// The stable section key.
    pub key: String,
    /// The human-readable section title.
    pub title: String,
    /// A one-line summary of the section contents.
    pub summary: String,
    /// The concrete items included in the section.
    #[serde(default)]
    pub items: Vec<BriefItem>,
}

/// A compact summary of recent immutable workspace activity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecentActivity {
    /// The timestamp of the recorded activity.
    pub ts: String,
    /// The actor that initiated the activity.
    pub actor: String,
    /// The operation that occurred.
    pub op: String,
    /// The affected primitive reference.
    pub reference: String,
}

/// A typed graph hygiene issue detected during graph construction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphIssue {
    /// Primitive that declared the unresolved reference.
    pub source_reference: String,
    /// Unresolved target reference or target primitive reference.
    pub target_reference: String,
    /// Intended edge kind.
    pub kind: String,
    /// Provenance of the reference.
    pub provenance: String,
    /// Human-readable resolution failure.
    pub reason: String,
}

/// Graph orphan node detected during graph construction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphOrphan {
    /// Orphan primitive reference in `type/id` form.
    pub reference: String,
}

/// Unsatisfied evidence contract for a thread.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThreadEvidenceGap {
    /// Thread reference in `type/id` form.
    pub thread_reference: String,
    /// Required exit criteria that remain unsatisfied.
    #[serde(default)]
    pub missing_criteria: Vec<String>,
}

/// Health and replay summary for one trigger subscription.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerHealth {
    /// Trigger reference in `type/id` form.
    pub trigger_reference: String,
    /// Trigger lifecycle status.
    pub status: String,
    /// Most recent evaluated event id, when any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_event_id: Option<String>,
    /// Most recent receipt id, when any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_receipt_id: Option<String>,
    /// Most recent event timestamp, when any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_evaluated_at: Option<String>,
    /// Most recent match timestamp, when any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_matched_at: Option<String>,
}

/// Summary of durable planned actions emitted by trigger receipts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerPlannedActionSummary {
    /// Number of pending/allowed plans across receipts.
    pub pending_count: usize,
    /// Number of policy-suppressed plans across receipts.
    pub suppressed_count: usize,
}

/// Recent durable trigger receipt surfaced for orientation and status output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TriggerReceiptSummary {
    /// Receipt reference in `type/id` form.
    pub receipt_reference: String,
    /// Trigger reference in `type/id` form.
    pub trigger_reference: String,
    /// Event source for the matched event.
    pub event_source: String,
    /// Event name when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub event_name: Option<String>,
    /// Subject reference when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject_reference: Option<String>,
    /// Receipt timestamp.
    pub occurred_at: String,
    /// Count of allowed/pending plans in this receipt.
    pub pending_plans: usize,
    /// Count of suppressed plans in this receipt.
    pub suppressed_plans: usize,
}

/// A structured, reusable workspace brief suitable for humans, agents, and future MCP resources.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceBrief {
    /// The selected lens used to shape the brief.
    pub lens: ContextLens,
    /// The stable workspace identifier.
    pub workspace_id: String,
    /// The human-readable workspace name.
    pub workspace_name: String,
    /// The filesystem root of the workspace.
    pub workspace_root: String,
    /// The configured default actor, when present.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_actor_id: Option<String>,
    /// Key primitive counts across the workspace.
    #[serde(default)]
    pub type_counts: BTreeMap<String, usize>,
    /// The primary orientation sections in this brief.
    #[serde(default)]
    pub sections: Vec<BriefSection>,
    /// Recent immutable ledger activity.
    #[serde(default)]
    pub recent_activity: Vec<RecentActivity>,
    /// Trigger subscription health summaries.
    #[serde(default)]
    pub trigger_health: Vec<TriggerHealth>,
    /// Recent durable trigger receipts.
    #[serde(default)]
    pub trigger_receipts: Vec<TriggerReceiptSummary>,
    /// Aggregate trigger-planned action summary.
    pub trigger_planned_actions: TriggerPlannedActionSummary,
    /// Warnings or gaps an entering agent should notice immediately.
    #[serde(default)]
    pub warnings: Vec<String>,
}

impl WorkspaceBrief {
    /// Returns true when the brief contains no sections and no recent activity.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.sections.is_empty() && self.recent_activity.is_empty()
    }
}

/// Single line of compact orientation output used by lightweight placeholder crates.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct StatusLine(pub String);

/// Minimal briefing container retained for placeholder crates that need a tiny orientation type.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Briefing {
    /// Heading that describes the briefing.
    pub heading: String,
    /// Individual status lines.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub items: Vec<StatusLine>,
}

impl Briefing {
    /// Returns true when the briefing has no items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::{
        BriefItem, BriefSection, ContextLens, RecentActivity, TriggerHealth,
        TriggerPlannedActionSummary, TriggerReceiptSummary, WorkspaceBrief,
    };

    #[test]
    fn context_lens_parses_supported_values() {
        assert_eq!(
            "workspace".parse::<ContextLens>().unwrap(),
            ContextLens::Workspace
        );
        assert_eq!(
            "delivery".parse::<ContextLens>().unwrap(),
            ContextLens::Delivery
        );
        assert!("unknown".parse::<ContextLens>().is_err());
    }

    #[test]
    fn workspace_brief_roundtrips_through_json() {
        let brief = WorkspaceBrief {
            lens: ContextLens::Workspace,
            workspace_id: "versatly".to_owned(),
            workspace_name: "Versatly".to_owned(),
            workspace_root: "/workspace".to_owned(),
            default_actor_id: Some("cli".to_owned()),
            type_counts: BTreeMap::from([("org".to_owned(), 1)]),
            sections: vec![BriefSection {
                key: "orgs".to_owned(),
                title: "Organizations".to_owned(),
                summary: "1 organization".to_owned(),
                items: vec![BriefItem {
                    kind: "org".to_owned(),
                    reference: Some("org/versatly".to_owned()),
                    title: "Versatly".to_owned(),
                    detail: Some("AI-native company".to_owned()),
                }],
            }],
            recent_activity: vec![RecentActivity {
                ts: "2026-03-15T16:37:24Z".to_owned(),
                actor: "cli".to_owned(),
                op: "create".to_owned(),
                reference: "org/versatly".to_owned(),
            }],
            trigger_health: vec![TriggerHealth {
                trigger_reference: "trigger/demo".to_owned(),
                status: "active".to_owned(),
                last_event_id: Some("event-1".to_owned()),
                last_receipt_id: Some("trigger_receipt/demo".to_owned()),
                last_evaluated_at: Some("2026-03-15T16:37:24Z".to_owned()),
                last_matched_at: Some("2026-03-15T16:37:24Z".to_owned()),
            }],
            trigger_receipts: vec![TriggerReceiptSummary {
                receipt_reference: "trigger_receipt/demo".to_owned(),
                trigger_reference: "trigger/demo".to_owned(),
                event_source: "ledger".to_owned(),
                event_name: Some("thread.done".to_owned()),
                subject_reference: Some("thread/thread-1".to_owned()),
                occurred_at: "2026-03-15T16:37:24Z".to_owned(),
                pending_plans: 1,
                suppressed_plans: 0,
            }],
            trigger_planned_actions: TriggerPlannedActionSummary {
                pending_count: 1,
                suppressed_count: 0,
            },
            warnings: vec!["No policies recorded yet".to_owned()],
        };

        let json = serde_json::to_string_pretty(&brief).expect("brief should serialize");
        let decoded: WorkspaceBrief =
            serde_json::from_str(&json).expect("brief should deserialize");

        assert_eq!(decoded, brief);
        assert!(!decoded.is_empty());
    }
}
