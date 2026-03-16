#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Shared orientation models for agent and human entry into a WorkGraph workspace.

use std::collections::BTreeMap;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

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

    use super::{BriefItem, BriefSection, ContextLens, RecentActivity, WorkspaceBrief};

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
            warnings: vec!["No policies recorded yet".to_owned()],
        };

        let json = serde_json::to_string_pretty(&brief).expect("brief should serialize");
        let decoded: WorkspaceBrief =
            serde_json::from_str(&json).expect("brief should deserialize");

        assert_eq!(decoded, brief);
        assert!(!decoded.is_empty());
    }
}
