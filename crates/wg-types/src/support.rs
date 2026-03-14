//! Shared helper types embedded by higher-order primitives.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

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
    pub external_id: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::{CachedSnapshot, ExternalRef};
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
            external_id: Some("42".into()),
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
                external_id: Some("123".into()),
                metadata: BTreeMap::from([("pipeline".into(), "enterprise".into())]),
            }],
        };

        roundtrip(&snapshot);
    }
}
