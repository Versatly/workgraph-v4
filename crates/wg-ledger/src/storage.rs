//! Shared file IO helpers for reading and appending JSONL ledger entries.

use std::path::Path;

use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use wg_error::{Result, WorkgraphError};
use wg_fs::ensure_dir;
use wg_types::LedgerEntry;

/// Reads every ledger entry from a JSONL ledger path.
///
/// Missing files are treated as an empty ledger.
pub(crate) async fn read_entries_from_path(path: &Path) -> Result<Vec<LedgerEntry>> {
    match fs::read_to_string(path).await {
        Ok(contents) => parse_entries(&contents),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(error.into()),
    }
}

/// Appends a single JSON-encoded ledger entry line to the target path.
///
/// # Errors
///
/// Returns an error when the parent directory cannot be created or the entry
/// cannot be serialized and written.
pub(crate) async fn append_entry_line(path: &Path, entry: &LedgerEntry) -> Result<()> {
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
