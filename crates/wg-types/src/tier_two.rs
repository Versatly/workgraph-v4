//! Tier 2 cached company context primitives.

use crate::{CachedSnapshot, ExternalRef, NodeId};
use serde::{Deserialize, Serialize};

/// Captures top-level organization context and external links.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Org {
    /// The stable organization identifier.
    pub id: String,
    /// The organization display name.
    pub title: String,
    /// A concise orientation summary for new humans or agents.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Tags used for filtering and grouping.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Links to authoritative external systems.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external_refs: Vec<ExternalRef>,
    /// A lightweight cached view of important external state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<CachedSnapshot>,
}

/// Captures a team and its responsibilities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Team {
    /// The stable team identifier.
    pub id: String,
    /// The team display name.
    pub title: String,
    /// The owning organization identifier when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub org_id: Option<String>,
    /// The team's mission or responsibility summary.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mission: Option<String>,
    /// The people or agents associated with the team.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<String>,
    /// Tags used for filtering and grouping.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Links to authoritative external systems.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external_refs: Vec<ExternalRef>,
}

/// Captures a human collaborator profile.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Person {
    /// The stable person identifier.
    pub id: String,
    /// The person's display name.
    pub title: String,
    /// The preferred email address.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// The person's primary role.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// The team identifiers associated with the person.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub team_ids: Vec<String>,
    /// Tags used for filtering and grouping.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Links to authoritative external systems.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external_refs: Vec<ExternalRef>,
}

/// Captures an AI agent profile and runtime metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Agent {
    /// The stable agent identifier.
    pub id: String,
    /// The agent display name.
    pub title: String,
    /// A concise explanation of what the agent is good at.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// The runtime or adapter used to launch the agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime: Option<String>,
    /// The human or team responsible for the agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// The network node currently associated with the agent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<NodeId>,
    /// The capabilities advertised by the agent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<String>,
    /// Tags used for filtering and grouping.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Links to authoritative external systems.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external_refs: Vec<ExternalRef>,
}

/// Captures customer context and external account links.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Client {
    /// The stable client identifier.
    pub id: String,
    /// The client display name.
    pub title: String,
    /// A concise summary of the customer relationship.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// The primary account owner inside the company.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_owner: Option<String>,
    /// Tags used for filtering and grouping.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Links to authoritative external systems.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external_refs: Vec<ExternalRef>,
    /// A lightweight cached view of important external state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<CachedSnapshot>,
}

/// Captures project context and links to delivery systems.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    /// The stable project identifier.
    pub id: String,
    /// The project display name.
    pub title: String,
    /// The current project status.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    /// The client associated with the project.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    /// Teams currently working on the project.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub team_ids: Vec<String>,
    /// Tags used for filtering and grouping.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Links to authoritative external systems.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external_refs: Vec<ExternalRef>,
    /// A lightweight cached view of important external state.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<CachedSnapshot>,
}

#[cfg(test)]
mod tests {
    use super::{Agent, Client, Org, Person, Project, Team};
    use crate::{CachedSnapshot, ExternalRef, NodeId};
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

    fn reference() -> ExternalRef {
        ExternalRef {
            provider: "github".into(),
            kind: "repo".into(),
            url: "https://github.com/Versatly/workgraph-v4".into(),
            id: None,
            metadata: BTreeMap::from([("visibility".into(), "private".into())]),
        }
    }

    fn snapshot() -> CachedSnapshot {
        CachedSnapshot {
            summary: "Project health is green".into(),
            source: "linear".into(),
            refreshed_at: Utc
                .with_ymd_and_hms(2026, 3, 14, 12, 0, 0)
                .single()
                .expect("valid timestamp"),
            fields: BTreeMap::from([("priority".into(), "high".into())]),
            external_refs: vec![reference()],
        }
    }

    #[test]
    fn tier_two_models_roundtrip_through_json() {
        let org = Org {
            id: "versatly".into(),
            title: "Versatly".into(),
            summary: Some("AI-native software company".into()),
            tags: vec!["company".into()],
            external_refs: vec![reference()],
            snapshot: Some(snapshot()),
        };
        let team = Team {
            id: "platform".into(),
            title: "Platform".into(),
            org_id: Some("versatly".into()),
            mission: Some("Build the WorkGraph kernel".into()),
            members: vec!["pedro".into(), "clawdious".into()],
            tags: vec!["engineering".into()],
            external_refs: vec![reference()],
        };
        let person = Person {
            id: "pedro".into(),
            title: "Pedro".into(),
            email: Some("pedro@example.com".into()),
            role: Some("Founder".into()),
            team_ids: vec!["platform".into()],
            tags: vec!["leadership".into()],
            external_refs: vec![reference()],
        };
        let agent = Agent {
            id: "clawdious".into(),
            title: "Clawdious".into(),
            description: Some("Background coding agent".into()),
            runtime: Some("cursor".into()),
            owner: Some("platform".into()),
            node_id: Some(NodeId::new("node-a")),
            capabilities: vec!["coding".into(), "review".into()],
            tags: vec!["ai".into()],
            external_refs: vec![reference()],
        };
        let client = Client {
            id: "acme".into(),
            title: "Acme Corp".into(),
            summary: Some("Enterprise launch customer".into()),
            account_owner: Some("pedro".into()),
            tags: vec!["enterprise".into()],
            external_refs: vec![reference()],
            snapshot: Some(snapshot()),
        };
        let project = Project {
            id: "workgraph-v4".into(),
            title: "WorkGraph v4".into(),
            status: Some("active".into()),
            client_id: Some("acme".into()),
            team_ids: vec!["platform".into()],
            tags: vec!["priority".into()],
            external_refs: vec![reference()],
            snapshot: Some(snapshot()),
        };

        roundtrip(&org);
        roundtrip(&team);
        roundtrip(&person);
        roundtrip(&agent);
        roundtrip(&client);
        roundtrip(&project);
    }
}
