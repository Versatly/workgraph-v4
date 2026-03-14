//! Append-only JSONL ledger with SHA-256 hash chain verification.

use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};

use chrono::{DateTime, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};
use wg_error::{Result, WorkgraphError};
use wg_paths::WorkspacePath;
use wg_types::{ActorId, LedgerEntry, LedgerEntryInput, LedgerOp, PrimitiveType};

/// Appends a new ledger entry and returns the persisted entry with hash fields.
pub fn append(workspace: &WorkspacePath, input: LedgerEntryInput) -> Result<LedgerEntry> {
    let ledger_path = workspace.ledger_path();
    if let Some(parent) = ledger_path.as_path().parent() {
        fs::create_dir_all(parent)?;
    }

    let existing = read_all(workspace)?;
    let prev_hash = existing.last().map(|entry| entry.hash.clone());

    let mut entry = LedgerEntry {
        ts: input.ts,
        actor: input.actor,
        op: input.op,
        primitive_type: input.primitive_type,
        primitive_id: input.primitive_id,
        fields_changed: input.fields_changed,
        hash: String::new(),
        prev_hash,
    };

    entry.hash = compute_hash(&entry)?;

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(ledger_path.as_path())?;
    writeln!(file, "{}", serde_json::to_string(&entry)?)?;

    Ok(entry)
}

/// Reads ledger entries starting from the provided cursor (line index).
pub fn read_from_cursor(workspace: &WorkspacePath, cursor: usize) -> Result<Vec<LedgerEntry>> {
    let ledger_file = workspace.ledger_path();
    if !ledger_file.as_path().exists() {
        return Ok(Vec::new());
    }

    let file = fs::File::open(ledger_file.as_path())?;
    let reader = BufReader::new(file);

    let mut entries = Vec::new();
    for (index, line) in reader.lines().enumerate() {
        if index < cursor {
            continue;
        }
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        entries.push(serde_json::from_str::<LedgerEntry>(&line)?);
    }

    Ok(entries)
}

/// Verifies the full ledger hash chain from the beginning.
pub fn verify_chain(workspace: &WorkspacePath) -> Result<()> {
    let entries = read_all(workspace)?;
    let mut expected_prev_hash: Option<String> = None;

    for entry in entries {
        if entry.prev_hash != expected_prev_hash {
            return Err(WorkgraphError::Integrity(format!(
                "prev_hash mismatch for {}",
                entry.primitive_id
            )));
        }

        let expected_hash = compute_hash(&entry)?;
        if entry.hash != expected_hash {
            return Err(WorkgraphError::Integrity(format!(
                "hash mismatch for {}",
                entry.primitive_id
            )));
        }

        expected_prev_hash = Some(entry.hash);
    }

    Ok(())
}

fn read_all(workspace: &WorkspacePath) -> Result<Vec<LedgerEntry>> {
    read_from_cursor(workspace, 0)
}

fn compute_hash(entry: &LedgerEntry) -> Result<String> {
    #[derive(Serialize)]
    struct HashInput<'a> {
        ts: &'a DateTime<Utc>,
        actor: &'a ActorId,
        op: &'a LedgerOp,
        primitive_type: &'a PrimitiveType,
        primitive_id: &'a str,
        fields_changed: &'a [String],
        prev_hash: &'a Option<String>,
    }

    let input = HashInput {
        ts: &entry.ts,
        actor: &entry.actor,
        op: &entry.op,
        primitive_type: &entry.primitive_type,
        primitive_id: &entry.primitive_id,
        fields_changed: &entry.fields_changed,
        prev_hash: &entry.prev_hash,
    };

    let payload = serde_json::to_vec(&input)?;
    let digest = Sha256::digest(payload);
    Ok(format!("{digest:x}"))
}

#[cfg(test)]
mod tests {
    use chrono::{TimeZone, Utc};

    use super::*;

    fn entry_input(id: &str) -> LedgerEntryInput {
        LedgerEntryInput {
            ts: Utc
                .with_ymd_and_hms(2026, 1, 1, 0, 0, 0)
                .single()
                .expect("valid timestamp"),
            actor: ActorId("tester".to_owned()),
            op: LedgerOp::Create,
            primitive_type: PrimitiveType::Org,
            primitive_id: id.to_owned(),
            fields_changed: vec!["title".to_owned()],
        }
    }

    #[test]
    fn append_and_verify_chain() {
        let tempdir = tempfile::tempdir().expect("tempdir should be created");
        let workspace = WorkspacePath::new(tempdir.path());

        append(&workspace, entry_input("acme")).expect("first append should succeed");
        append(&workspace, entry_input("globex")).expect("second append should succeed");

        verify_chain(&workspace).expect("chain should verify");
        let entries = read_from_cursor(&workspace, 0).expect("read should succeed");
        assert_eq!(entries.len(), 2);
        assert!(entries[1].prev_hash.is_some());
    }

    #[test]
    fn verify_chain_detects_tampering() {
        let tempdir = tempfile::tempdir().expect("tempdir should be created");
        let workspace = WorkspacePath::new(tempdir.path());

        append(&workspace, entry_input("acme")).expect("append should succeed");

        let ledger = workspace.ledger_path().as_path().to_path_buf();
        let mut content = fs::read_to_string(&ledger).expect("ledger should be readable");
        content = content.replace("acme", "evil");
        fs::write(&ledger, content).expect("ledger should be writable");

        let error = verify_chain(&workspace).expect_err("tampering should fail verification");
        assert_eq!(error.code().as_str(), "integrity");
    }
}
