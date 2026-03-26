//! Append logic for immutable JSONL ledger files.

use std::path::PathBuf;

use wg_clock::Clock;
use wg_error::Result;
use wg_paths::WorkspacePath;
use wg_types::LedgerEntry;

use crate::hash::compute_entry_hash;
use crate::model::LedgerEntryDraft;
use crate::storage::{append_entry_line, read_entries_from_path};
use crate::verify::verify_entries;

/// Appends immutable entries to a workspace ledger file.
pub struct LedgerWriter<C> {
    root: WorkspacePath,
    clock: C,
}

impl<C> LedgerWriter<C>
where
    C: Clock,
{
    /// Builds a writer for the given workspace root and clock.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>, clock: C) -> Self {
        Self {
            root: WorkspacePath::new(root),
            clock,
        }
    }

    /// Appends a stamped, hash-linked entry to the ledger and returns it.
    ///
    /// # Errors
    ///
    /// Returns an error when the existing chain is invalid, the new entry
    /// cannot be hashed, or the JSONL append fails.
    pub async fn append(&self, draft: LedgerEntryDraft) -> Result<LedgerEntry> {
        let ledger_path = self.root.ledger_path();
        let existing_entries = read_entries_from_path(ledger_path.as_path()).await?;
        verify_entries(&existing_entries)?;

        let prev_hash = existing_entries.last().map(|entry| entry.hash.clone());
        let mut entry = LedgerEntry {
            ts: self.clock.now(),
            actor: draft.actor,
            op: draft.op,
            primitive_type: draft.primitive_type,
            primitive_id: draft.primitive_id,
            fields_changed: draft.fields_changed,
            hash: String::new(),
            prev_hash,
            note: draft.note,
        };
        entry.hash = compute_entry_hash(&entry)?;

        append_entry_line(ledger_path.as_path(), &entry).await?;
        Ok(entry)
    }
}
