//! Ledger data types shared by mutation-producing crates.

use crate::ActorId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Enumerates the durable mutation operations captured in the ledger.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LedgerOp {
    /// Creates a new primitive or runtime artifact.
    Create,
    /// Updates an existing primitive or runtime artifact.
    Update,
    /// Deletes a primitive from active storage.
    Delete,
    /// Claims work ownership.
    Claim,
    /// Releases an existing claim or lock.
    Release,
    /// Starts execution or active work.
    Start,
    /// Marks work as completed successfully.
    Done,
    /// Cancels work before completion.
    Cancel,
    /// Reopens previously completed or cancelled work.
    Reopen,
    /// Assigns work to a human or agent.
    Assign,
    /// Removes an assignee from work.
    Unassign,
}

/// Represents one immutable, hash-linked ledger record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerEntry {
    /// The time the mutation was recorded.
    pub ts: DateTime<Utc>,
    /// The actor who initiated the mutation.
    pub actor: ActorId,
    /// The operation captured by this entry.
    pub op: LedgerOp,
    /// The primitive type affected by the mutation.
    pub primitive_type: String,
    /// The primitive identifier affected by the mutation.
    pub primitive_id: String,
    /// The fields changed by the mutation, when known.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields_changed: Vec<String>,
    /// The hash for this ledger entry.
    pub hash: String,
    /// The previous hash in the immutable chain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev_hash: Option<String>,
    /// An optional human-readable note about the mutation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::{LedgerEntry, LedgerOp};
    use crate::ActorId;
    use chrono::{TimeZone, Utc};

    #[test]
    fn ledger_entry_roundtrips_through_json() {
        let entry = LedgerEntry {
            ts: Utc
                .with_ymd_and_hms(2026, 3, 14, 10, 15, 0)
                .single()
                .expect("valid timestamp"),
            actor: ActorId::new("pedro"),
            op: LedgerOp::Done,
            primitive_type: "decision".into(),
            primitive_id: "rust-for-workgraph-v4".into(),
            fields_changed: vec!["status".into(), "decided_at".into()],
            hash: "abc123".into(),
            prev_hash: Some("prev999".into()),
            note: Some("Approved after architecture review".into()),
        };

        let json = serde_json::to_string_pretty(&entry).expect("entry should serialize");
        let decoded: LedgerEntry = serde_json::from_str(&json).expect("entry should deserialize");

        assert_eq!(decoded, entry);
        assert!(json.contains("\"done\""));
    }
}
