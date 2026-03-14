//! Async filesystem helpers for atomic writes and directory utilities.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use tempfile::TempDir;
use wg_error::Result;

/// Ensures that a directory exists, creating parents as needed.
pub async fn ensure_dir(path: impl AsRef<Path>) -> Result<()> {
    tokio::fs::create_dir_all(path.as_ref()).await?;
    Ok(())
}

/// Writes bytes to a target path using a same-directory atomic rename.
pub async fn atomic_write(path: impl AsRef<Path>, contents: &[u8]) -> Result<()> {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        ensure_dir(parent).await?;
    }

    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0_u128, |duration| duration.as_nanos());
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("workgraph");
    let tmp_name = format!(".{file_name}.{timestamp}.tmp");
    let tmp_path = path.parent().map_or_else(
        || Path::new(".").join(&tmp_name),
        |parent| parent.join(&tmp_name),
    );

    tokio::fs::write(&tmp_path, contents).await?;
    tokio::fs::rename(&tmp_path, path).await?;
    Ok(())
}

/// Creates a temporary directory with the provided prefix.
pub fn temp_dir(prefix: &str) -> Result<TempDir> {
    Ok(tempfile::Builder::new().prefix(prefix).tempdir()?)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    #[tokio::test]
    async fn ensure_dir_creates_nested_directories() {
        let tmp = temp_dir("wg-fs-test").expect("temp dir should be created");
        let nested = tmp.path().join("a/b/c");
        ensure_dir(&nested)
            .await
            .expect("nested directory should be created");
        assert!(nested.exists());
        assert!(nested.is_dir());
    }

    #[tokio::test]
    async fn atomic_write_replaces_file_contents() {
        let tmp = temp_dir("wg-fs-test").expect("temp dir should be created");
        let file = tmp.path().join("file.txt");

        atomic_write(&file, b"first")
            .await
            .expect("first write should succeed");
        atomic_write(&file, b"second")
            .await
            .expect("second write should succeed");

        let content = tokio::fs::read_to_string(&file)
            .await
            .expect("file should be readable");
        assert_eq!(content, "second");

        let parent = file.parent().unwrap_or_else(|| Path::new("."));
        let mut entries = tokio::fs::read_dir(parent)
            .await
            .expect("parent should be readable");
        while let Some(entry) = entries
            .next_entry()
            .await
            .expect("reading directory should succeed")
        {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            assert!(!name.ends_with(".tmp"));
        }
    }
}
