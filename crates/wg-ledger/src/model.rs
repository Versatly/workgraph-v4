//! Public models used by the append-only WorkGraph ledger.

use wg_types::{ActorId, LedgerOp};

/// A pending mutation that has not yet been timestamped or hash-linked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerEntryDraft {
    /// The actor who initiated the mutation.
    pub actor: ActorId,
    /// The operation captured by the ledger entry.
    pub op: LedgerOp,
    /// The primitive type affected by the mutation.
    pub primitive_type: String,
    /// The primitive identifier affected by the mutation.
    pub primitive_id: String,
    /// The fields changed by the mutation, when known.
    pub fields_changed: Vec<String>,
    /// Optional human-readable note describing the mutation.
    pub note: Option<String>,
}

impl LedgerEntryDraft {
    /// Creates a new ledger entry draft.
    #[must_use]
    pub fn new(
        actor: ActorId,
        op: LedgerOp,
        primitive_type: impl Into<String>,
        primitive_id: impl Into<String>,
        fields_changed: Vec<String>,
    ) -> Self {
        Self {
            actor,
            op,
            primitive_type: primitive_type.into(),
            primitive_id: primitive_id.into(),
            fields_changed,
            note: None,
        }
    }

    /// Attaches an optional descriptive note to the draft.
    #[must_use]
    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.note = Some(note.into());
        self
    }
}

/// A cursor that points to the next unread zero-based ledger line.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct LedgerCursor(usize);

impl LedgerCursor {
    /// Creates a cursor from a zero-based ledger line index.
    #[must_use]
    pub const fn new(line: usize) -> Self {
        Self(line)
    }

    /// Returns the zero-based ledger line index.
    #[must_use]
    pub const fn line(self) -> usize {
        self.0
    }
}
