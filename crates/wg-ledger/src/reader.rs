//! Cursor-based reading for immutable JSONL ledger files.

use std::path::PathBuf;

use wg_error::Result;
use wg_paths::WorkspacePath;
use wg_types::LedgerEntry;

use crate::model::LedgerCursor;
use crate::storage::read_entries_from_path;

/// Reads ledger entries from a workspace ledger file.
#[derive(Debug, Clone)]
pub struct LedgerReader {
    root: WorkspacePath,
}

impl LedgerReader {
    /// Builds a reader for the given workspace root.
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: WorkspacePath::new(root),
        }
    }

    /// Reads all entries starting at the provided cursor and returns the next cursor.
    ///
    /// # Errors
    ///
    /// Returns an error when the ledger cannot be read or decoded.
    pub async fn read_from(
        &self,
        cursor: LedgerCursor,
    ) -> Result<(Vec<LedgerEntry>, LedgerCursor)> {
        let entries = read_entries_from_path(self.root.ledger_path().as_path()).await?;
        let start = cursor.line().min(entries.len());
        let next_cursor = LedgerCursor::new(entries.len());

        Ok((entries.into_iter().skip(start).collect(), next_cursor))
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use wg_clock::MockClock;
    use wg_types::{ActorId, LedgerOp};

    use crate::{LedgerCursor, LedgerEntryDraft, LedgerReader, LedgerWriter, verify_chain};

    fn draft(op: LedgerOp, primitive_id: &str, fields_changed: &[&str]) -> LedgerEntryDraft {
        LedgerEntryDraft::new(
            ActorId::new("pedro"),
            op,
            "decision",
            primitive_id,
            fields_changed
                .iter()
                .map(|field| (*field).to_owned())
                .collect(),
        )
    }

    #[tokio::test]
    async fn empty_ledger_reads_and_verifies_cleanly() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let reader = LedgerReader::new(temp_dir.path());

        let (entries, cursor) = reader
            .read_from(LedgerCursor::default())
            .await
            .expect("reading a missing ledger should succeed");

        assert!(entries.is_empty());
        assert_eq!(cursor, LedgerCursor::new(0));

        verify_chain(temp_dir.path())
            .await
            .expect("missing ledger should be treated as valid and empty");
    }

    #[tokio::test]
    async fn reader_cursor_advances_with_existing_entries() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let clock = MockClock::new(
            "2026-03-14T12:00:00Z"
                .parse()
                .expect("test timestamp should parse as RFC 3339"),
        );
        let writer = LedgerWriter::new(temp_dir.path(), clock.clone());

        let first = writer
            .append(draft(LedgerOp::Create, "rust-for-workgraph-v4", &["title"]))
            .await
            .expect("first append should succeed");

        clock.set(
            "2026-03-14T12:05:00Z"
                .parse()
                .expect("test timestamp should parse as RFC 3339"),
        );

        let second = writer
            .append(draft(
                LedgerOp::Update,
                "rust-for-workgraph-v4",
                &["status", "decided_at"],
            ))
            .await
            .expect("second append should succeed");

        let reader = LedgerReader::new(temp_dir.path());
        let (entries, next_cursor) = reader
            .read_from(LedgerCursor::default())
            .await
            .expect("reading the full ledger should succeed");
        let (tail, tail_cursor) = reader
            .read_from(LedgerCursor::new(1))
            .await
            .expect("reading from a non-zero cursor should succeed");
        let (empty_tail, end_cursor) = reader
            .read_from(LedgerCursor::new(99))
            .await
            .expect("reading past the end should succeed");

        assert_eq!(first.prev_hash, None);
        assert_eq!(second.prev_hash.as_deref(), Some(first.hash.as_str()));
        assert_ne!(first.hash, second.hash);
        assert_eq!(entries, vec![first.clone(), second.clone()]);
        assert_eq!(next_cursor, LedgerCursor::new(2));
        assert_eq!(tail, vec![second]);
        assert_eq!(tail_cursor, LedgerCursor::new(2));
        assert!(empty_tail.is_empty());
        assert_eq!(end_cursor, LedgerCursor::new(2));
    }
}
