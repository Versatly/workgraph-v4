//! Chain verification for immutable JSONL ledger files.

use std::path::PathBuf;

use wg_error::{Result, WorkgraphError};
use wg_paths::WorkspacePath;
use wg_types::LedgerEntry;

use crate::hash::compute_entry_hash;
use crate::storage::read_entries_from_path;

/// Recomputes and validates the full hash chain for a workspace ledger.
///
/// # Errors
///
/// Returns a ledger error when any `prev_hash` link or recomputed hash fails
/// validation, or an IO/encoding error when the ledger cannot be read.
pub async fn verify_chain(root: impl Into<PathBuf>) -> Result<()> {
    let root = WorkspacePath::new(root);
    let entries = read_entries_from_path(root.ledger_path().as_path()).await?;
    verify_entries(&entries)
}

/// Verifies a sequence of ledger entries in memory.
pub(crate) fn verify_entries(entries: &[LedgerEntry]) -> Result<()> {
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

#[cfg(test)]
mod tests {
    use tempfile::tempdir;
    use tokio::fs;
    use wg_clock::MockClock;
    use wg_types::{ActorId, LedgerEntry, LedgerOp};

    use crate::hash::compute_entry_hash;
    use crate::{LedgerEntryDraft, LedgerWriter, verify_chain};

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
