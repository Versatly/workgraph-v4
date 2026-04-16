//! Shared helper types embedded by higher-order primitives.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::str::FromStr;

/// References an external system without copying the source of truth into WorkGraph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExternalRef {
    /// The source system or provider name, such as `github` or `linear`.
    pub provider: String,
    /// The record kind within the provider, such as `repo`, `issue`, or `email`.
    pub kind: String,
    /// The canonical URL or URI for the external record.
    pub url: String,
    /// The provider-specific identifier when it differs from the URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// Additional small metadata fields needed to orient an agent without calling the provider.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub metadata: BTreeMap<String, String>,
}

/// Stores a lightweight local snapshot of external state for orientation and querying.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CachedSnapshot {
    /// A human-readable summary of the external state at the time it was captured.
    pub summary: String,
    /// The system that produced the snapshot.
    pub source: String,
    /// The time the snapshot was last refreshed.
    pub refreshed_at: DateTime<Utc>,
    /// Additional flattened key/value attributes from the external source.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fields: BTreeMap<String, String>,
    /// Links back to the authoritative external records represented by this snapshot.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub external_refs: Vec<ExternalRef>,
}

/// Governance scope granted to a remote hosted or MCP credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RemoteAccessScope {
    /// Read-only access to orientation and query commands.
    #[default]
    Read,
    /// Operational coordination writes such as claiming threads or transitioning runs.
    Operate,
    /// Full administrative access, including broad create flows and trigger administration.
    Admin,
}

impl RemoteAccessScope {
    /// Returns the stable snake_case label for this access scope.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::Operate => "operate",
            Self::Admin => "admin",
        }
    }

    /// Returns true when this scope satisfies a requested minimum scope.
    #[must_use]
    pub const fn allows(self, required: Self) -> bool {
        self.rank() >= required.rank()
    }

    const fn rank(self) -> u8 {
        match self {
            Self::Read => 0,
            Self::Operate => 1,
            Self::Admin => 2,
        }
    }
}

impl std::fmt::Display for RemoteAccessScope {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl FromStr for RemoteAccessScope {
    type Err = String;

    fn from_str(input: &str) -> std::result::Result<Self, Self::Err> {
        match input {
            "read" => Ok(Self::Read),
            "operate" => Ok(Self::Operate),
            "admin" => Ok(Self::Admin),
            _ => Err(format!(
                "unsupported remote access scope '{input}'; expected one of read, operate, admin"
            )),
        }
    }
}

/// A remotely executable WorkGraph command request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteCommandRequest {
    /// Full CLI argument vector, including the binary name.
    pub args: Vec<String>,
    /// Optional actor override used to attribute mutations on the hosted server.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_id: Option<String>,
}

/// A remotely executed WorkGraph command response.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RemoteCommandResponse {
    /// Whether command execution succeeded.
    pub success: bool,
    /// The rendered human or JSON envelope returned by the command.
    pub rendered: String,
}

#[cfg(test)]
mod tests {
    use super::{
        CachedSnapshot, ExternalRef, RemoteAccessScope, RemoteCommandRequest, RemoteCommandResponse,
    };
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
    fn external_ref_roundtrips_through_json() {
        let reference = ExternalRef {
            provider: "github".into(),
            kind: "issue".into(),
            url: "https://github.com/Versatly/workgraph-v4/issues/42".into(),
            id: Some("42".into()),
            metadata: BTreeMap::from([
                ("repo".into(), "Versatly/workgraph-v4".into()),
                ("state".into(), "open".into()),
            ]),
        };

        roundtrip(&reference);
    }

    #[test]
    fn cached_snapshot_roundtrips_through_json() {
        let snapshot = CachedSnapshot {
            summary: "Client launch is on track".into(),
            source: "crm".into(),
            refreshed_at: Utc
                .with_ymd_and_hms(2026, 3, 14, 9, 30, 0)
                .single()
                .expect("valid timestamp"),
            fields: BTreeMap::from([
                ("health".into(), "green".into()),
                ("stage".into(), "implementation".into()),
            ]),
            external_refs: vec![ExternalRef {
                provider: "hubspot".into(),
                kind: "deal".into(),
                url: "https://example.com/deals/123".into(),
                id: Some("123".into()),
                metadata: BTreeMap::from([("pipeline".into(), "enterprise".into())]),
            }],
        };

        roundtrip(&snapshot);
    }

    #[test]
    fn remote_command_contract_roundtrips_through_json() {
        roundtrip(&RemoteCommandRequest {
            args: vec!["workgraph".into(), "--json".into(), "status".into()],
            actor_id: Some("agent:cursor".into()),
        });
        roundtrip(&RemoteCommandResponse {
            success: true,
            rendered: "{\"success\":true}".into(),
        });
    }

    #[test]
    fn remote_access_scope_roundtrips_and_orders() {
        roundtrip(&RemoteAccessScope::Operate);
        assert!(RemoteAccessScope::Admin.allows(RemoteAccessScope::Read));
        assert!(RemoteAccessScope::Operate.allows(RemoteAccessScope::Operate));
        assert!(!RemoteAccessScope::Read.allows(RemoteAccessScope::Admin));
        assert_eq!(
            "operate"
                .parse::<RemoteAccessScope>()
                .expect("scope should parse"),
            RemoteAccessScope::Operate
        );
    }
}
