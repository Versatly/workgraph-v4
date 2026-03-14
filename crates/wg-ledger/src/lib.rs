#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Append-only JSONL ledger storage with SHA-256 hash chaining.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use wg_clock::Clock;
use wg_error::{Result, WorkgraphError};
use wg_fs::ensure_dir;
use wg_paths::WorkspacePath;
use wg_types::{ActorId, LedgerEntry, LedgerOp};

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
        }
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
            note: None,
        };
        entry.hash = compute_entry_hash(&entry)?;

        append_entry_line(ledger_path.as_path(), &entry).await?;
        Ok(entry)
    }
}

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

/// Recomputes and validates the full hash chain for a workspace ledger.
pub async fn verify_chain(root: impl Into<PathBuf>) -> Result<()> {
    let root = WorkspacePath::new(root);
    let entries = read_entries_from_path(root.ledger_path().as_path()).await?;
    verify_entries(&entries)
}

fn verify_entries(entries: &[LedgerEntry]) -> Result<()> {
    let mut expected_prev_hash: Option<String> = None;

    for (line_index, entry) in entries.iter().enumerate() {
        if entry.prev_hash.as_ref() != expected_prev_hash.as_ref() {
            return Err(WorkgraphError::LedgerError(format!(
                "ledger chain broken at line {}: expected prev_hash {:?}, found {:?}",
                line_index + 1,
                expected_prev_hash,
                entry.prev_hash
            )));
        }

        let expected_hash = compute_entry_hash(entry)?;
        if entry.hash != expected_hash {
            return Err(WorkgraphError::LedgerError(format!(
                "ledger hash mismatch at line {}: expected {}, found {}",
                line_index + 1,
                expected_hash,
                entry.hash
            )));
        }

        expected_prev_hash = Some(entry.hash.clone());
    }

    Ok(())
}

fn compute_entry_hash(entry: &LedgerEntry) -> Result<String> {
    let hash_material = serde_json::json!({
        "actor": &entry.actor,
        "fields_changed": &entry.fields_changed,
        "note": &entry.note,
        "op": entry.op,
        "prev_hash": &entry.prev_hash,
        "primitive_id": &entry.primitive_id,
        "primitive_type": &entry.primitive_type,
        "ts": &entry.ts,
    });

    let encoded = serde_json::to_vec(&hash_material).map_err(|error| {
        WorkgraphError::EncodingError(format!("failed to serialize ledger hash material: {error}"))
    })?;

    Ok(format!("{:x}", Sha256::digest(encoded)))
}

async fn read_entries_from_path(path: &Path) -> Result<Vec<LedgerEntry>> {
    match fs::read_to_string(path).await {
        Ok(contents) => parse_entries(&contents),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error.into()),
    }
}

fn parse_entries(contents: &str) -> Result<Vec<LedgerEntry>> {
    let mut entries = Vec::new();

    for (line_index, line) in contents.lines().enumerate() {
        if line.trim().is_empty() {
            return Err(WorkgraphError::LedgerError(format!(
                "ledger line {} is empty",
                line_index + 1
            )));
        }

        let entry = serde_json::from_str::<LedgerEntry>(line).map_err(|error| {
            WorkgraphError::EncodingError(format!(
                "failed to parse ledger line {}: {error}",
                line_index + 1
            ))
        })?;
        entries.push(entry);
    }

    Ok(entries)
}

async fn append_entry_line(path: &Path, entry: &LedgerEntry) -> Result<()> {
    if let Some(parent) = path.parent() {
        ensure_dir(parent).await?;
    }

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .await?;
    let line = serde_json::to_vec(entry).map_err(|error| {
        WorkgraphError::EncodingError(format!("failed to encode ledger entry as JSON: {error}"))
    })?;

    file.write_all(&line).await?;
    file.write_all(b"\n").await?;
    file.sync_all().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        LedgerCursor, LedgerEntryDraft, LedgerReader, LedgerWriter, compute_entry_hash,
        verify_chain,
    };
    use tempfile::tempdir;
    use tokio::fs;
    use wg_clock::MockClock;
    use wg_types::{ActorId, LedgerEntry, LedgerOp};

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

    async fn read_ledger_lines(root: &std::path::Path) -> Vec<String> {
        let contents = fs::read_to_string(root.join(".workgraph").join("ledger.jsonl"))
            .await
            .expect("ledger file should be readable");
        contents.lines().map(str::to_owned).collect()
    }

    async fn write_ledger_lines(root: &std::path::Path, lines: &[String]) {
        let mut contents = lines.join("\n");
        if !contents.is_empty() {
            contents.push('\n');
        }

        fs::write(root.join(".workgraph").join("ledger.jsonl"), contents)
            .await
            .expect("ledger file should be writable");
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
    async fn append_creates_hash_linked_entries_and_reader_cursor_advances() {
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
        assert_eq!(tail, vec![second.clone()]);
        assert_eq!(tail_cursor, LedgerCursor::new(2));
        assert!(empty_tail.is_empty());
        assert_eq!(end_cursor, LedgerCursor::new(2));

        let lines = read_ledger_lines(temp_dir.path()).await;
        assert_eq!(lines.len(), 2);

        verify_chain(temp_dir.path())
            .await
            .expect("freshly written ledger should verify");
    }

    #[tokio::test]
    async fn verify_chain_detects_payload_tampering() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let clock = MockClock::new(
            "2026-03-14T12:00:00Z"
                .parse()
                .expect("test timestamp should parse as RFC 3339"),
        );
        let writer = LedgerWriter::new(temp_dir.path(), clock.clone());

        writer
            .append(draft(LedgerOp::Create, "rust-for-workgraph-v4", &["title"]))
            .await
            .expect("first append should succeed");
        clock.set(
            "2026-03-14T12:05:00Z"
                .parse()
                .expect("test timestamp should parse as RFC 3339"),
        );
        writer
            .append(draft(
                LedgerOp::Update,
                "rust-for-workgraph-v4",
                &["status", "decided_at"],
            ))
            .await
            .expect("second append should succeed");

        let mut lines = read_ledger_lines(temp_dir.path()).await;
        let mut tampered: LedgerEntry =
            serde_json::from_str(&lines[1]).expect("second ledger line should parse");
        tampered.primitive_id = "rewritten-history".to_owned();
        lines[1] = serde_json::to_string(&tampered).expect("tampered entry should serialize");
        write_ledger_lines(temp_dir.path(), &lines).await;

        let error = verify_chain(temp_dir.path())
            .await
            .expect_err("tampered payload should fail verification");
        assert_eq!(error.code(), "ledger_error");
        assert!(error.to_string().contains("hash mismatch"));
    }

    #[tokio::test]
    async fn verify_chain_detects_forged_prev_hash_links() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let clock = MockClock::new(
            "2026-03-14T12:00:00Z"
                .parse()
                .expect("test timestamp should parse as RFC 3339"),
        );
        let writer = LedgerWriter::new(temp_dir.path(), clock.clone());

        writer
            .append(draft(LedgerOp::Create, "rust-for-workgraph-v4", &["title"]))
            .await
            .expect("first append should succeed");
        clock.set(
            "2026-03-14T12:05:00Z"
                .parse()
                .expect("test timestamp should parse as RFC 3339"),
        );
        writer
            .append(draft(
                LedgerOp::Update,
                "rust-for-workgraph-v4",
                &["status", "decided_at"],
            ))
            .await
            .expect("second append should succeed");

        let mut lines = read_ledger_lines(temp_dir.path()).await;
        let mut forged: LedgerEntry =
            serde_json::from_str(&lines[1]).expect("second ledger line should parse");
        forged.prev_hash = Some("forged-prev-hash".to_owned());
        forged.hash = compute_entry_hash(&forged).expect("forged entry hash should recompute");
        lines[1] = serde_json::to_string(&forged).expect("forged entry should serialize");
        write_ledger_lines(temp_dir.path(), &lines).await;

        let error = verify_chain(temp_dir.path())
            .await
            .expect_err("forged chain link should fail verification");
        assert_eq!(error.code(), "ledger_error");
        assert!(error.to_string().contains("chain broken"));
    }
}
