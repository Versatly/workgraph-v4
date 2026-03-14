//! Tier 1 primitives that exist uniquely inside WorkGraph.

use crate::{ActorId, ExternalRef};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Captures a decision, its rationale, and its outcome.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decision {
    /// The stable decision identifier.
    pub id: String,
    /// The decision title.
    pub title: String,
    /// The current decision status.
    pub status: String,
    /// The primary actor who decided, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decided_by: Option<ActorId>,
    /// The time the decision was finalized, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decided_at: Option<DateTime<Utc>>,
    /// Other participants in the discussion.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub participants: Vec<String>,
    /// Background context that motivated the decision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// Why the final choice was made.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rationale: Option<String>,
    /// Important consequences of the decision.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub consequences: Vec<String>,
    /// Tags used for filtering and grouping.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Links to authoritative external systems.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external_refs: Vec<ExternalRef>,
}

/// Captures a repeatable operating pattern or playbook.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pattern {
    /// The stable pattern identifier.
    pub id: String,
    /// The pattern title.
    pub title: String,
    /// A concise summary of the pattern.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Guidance on when the pattern should be applied.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub when_to_use: Option<String>,
    /// The repeatable steps that make up the pattern.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub steps: Vec<String>,
    /// Known edge cases or exceptions.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exceptions: Vec<String>,
    /// Tags used for filtering and grouping.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Links to authoritative external systems.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external_refs: Vec<ExternalRef>,
}

/// Captures a lesson learned from prior work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lesson {
    /// The stable lesson identifier.
    pub id: String,
    /// The lesson title.
    pub title: String,
    /// A concise summary of the lesson.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// The experience or event that produced the lesson.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub learned_from: Option<String>,
    /// Contexts where the lesson applies.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub applies_to: Vec<String>,
    /// Tags used for filtering and grouping.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Links to authoritative external systems.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external_refs: Vec<ExternalRef>,
}

/// Captures a normative rule and its operating scope.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Policy {
    /// The stable policy identifier.
    pub id: String,
    /// The policy title.
    pub title: String,
    /// The scope where the policy applies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<String>,
    /// The rule or directive that must be followed.
    pub rule: String,
    /// How the policy is enforced.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enforcement: Option<String>,
    /// Known exceptions to the policy.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exceptions: Vec<String>,
    /// Tags used for filtering and grouping.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Links to authoritative external systems.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external_refs: Vec<ExternalRef>,
}

/// Captures a contextual relationship between two entities.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relationship {
    /// The stable relationship identifier.
    pub id: String,
    /// The relationship title.
    pub title: String,
    /// The source entity identifier.
    pub from: String,
    /// The target entity identifier.
    pub to: String,
    /// The nature of the relationship.
    pub nature: String,
    /// Additional context about the relationship.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    /// Tags used for filtering and grouping.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Links to authoritative external systems.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external_refs: Vec<ExternalRef>,
}

/// Captures long-term strategic context that should persist over time.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StrategicNote {
    /// The stable strategic note identifier.
    pub id: String,
    /// The note title.
    pub title: String,
    /// A concise summary of the strategic note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// The planning horizon for the note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub horizon: Option<String>,
    /// The long-form content of the strategic note.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
    /// Tags used for filtering and grouping.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Links to authoritative external systems.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external_refs: Vec<ExternalRef>,
}

#[cfg(test)]
mod tests {
    use super::{Decision, Lesson, Pattern, Policy, Relationship, StrategicNote};
    use crate::{ActorId, ExternalRef};
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
            kind: "pull_request".into(),
            url: "https://github.com/Versatly/workgraph-v4/pull/7".into(),
            external_id: Some("7".into()),
            metadata: BTreeMap::from([("repo".into(), "Versatly/workgraph-v4".into())]),
        }
    }

    #[test]
    fn tier_one_models_roundtrip_through_json() {
        let decision = Decision {
            id: "rust-for-workgraph-v4".into(),
            title: "Rust for WorkGraph v4".into(),
            status: "decided".into(),
            decided_by: Some(ActorId::new("pedro")),
            decided_at: Some(
                Utc.with_ymd_and_hms(2026, 3, 13, 18, 0, 0)
                    .single()
                    .expect("valid timestamp"),
            ),
            participants: vec!["pedro".into(), "clawdious".into()],
            context: Some("TypeScript hit scale limits".into()),
            rationale: Some("Need a single binary with strong typing".into()),
            consequences: vec!["Three month rewrite".into()],
            tags: vec!["architecture".into()],
            external_refs: vec![reference()],
        };
        let pattern = Pattern {
            id: "ship-small-prs".into(),
            title: "Ship small PRs".into(),
            summary: Some("Keep reviewable changes small".into()),
            when_to_use: Some("All routine product work".into()),
            steps: vec!["Plan".into(), "Implement".into(), "Verify".into()],
            exceptions: vec!["Emergency hotfixes".into()],
            tags: vec!["delivery".into()],
            external_refs: vec![reference()],
        };
        let lesson = Lesson {
            id: "tests-catch-schema-drift".into(),
            title: "Tests catch schema drift".into(),
            summary: Some("Roundtrip tests protect frontmatter models".into()),
            learned_from: Some("Prior migration bugs".into()),
            applies_to: vec!["registry".into(), "store".into()],
            tags: vec!["quality".into()],
            external_refs: vec![reference()],
        };
        let policy = Policy {
            id: "always-write-doc-comments".into(),
            title: "Document public APIs".into(),
            scope: Some("Rust crates".into()),
            rule: "Every public item must include a doc comment.".into(),
            enforcement: Some("CI linting and code review".into()),
            exceptions: vec![],
            tags: vec!["engineering".into()],
            external_refs: vec![reference()],
        };
        let relationship = Relationship {
            id: "pedro-owns-acme".into(),
            title: "Pedro owns Acme account".into(),
            from: "pedro".into(),
            to: "acme".into(),
            nature: "account_owner".into(),
            context: Some("Primary escalation point".into()),
            tags: vec!["client".into()],
            external_refs: vec![reference()],
        };
        let strategic_note = StrategicNote {
            id: "ai-native-ops".into(),
            title: "AI-native ops advantage".into(),
            summary: Some("Operational leverage comes from shared context".into()),
            horizon: Some("3-5 years".into()),
            body: Some("WorkGraph should become the company nervous system.".into()),
            tags: vec!["strategy".into()],
            external_refs: vec![reference()],
        };

        roundtrip(&decision);
        roundtrip(&pattern);
        roundtrip(&lesson);
        roundtrip(&policy);
        roundtrip(&relationship);
        roundtrip(&strategic_note);
    }
}
