//! Async filesystem helpers used by WorkGraph storage crates.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;

static TEMP_FILE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Creates a directory and all missing parents if they do not already exist.
pub async fn ensure_dir(path: impl AsRef<Path>) -> std::io::Result<()> {
    fs::create_dir_all(path.as_ref()).await
}

/// Atomically writes bytes to a file by using a sibling temporary file followed by a rename.
pub async fn atomic_write(
    path: impl AsRef<Path>,
    contents: impl AsRef<[u8]>,
) -> std::io::Result<()> {
    let path = path.as_ref();

    if let Some(parent) = path.parent() {
        ensure_dir(parent).await?;
    }

    let temp_path = create_temp_file(path).await?;
    let write_result = write_and_rename(&temp_path, path, contents.as_ref()).await;

    if write_result.is_err() {
        let _ = fs::remove_file(&temp_path).await;
    }

    write_result
}

/// Lists markdown files in a directory, sorted by path.
pub async fn list_md_files(path: impl AsRef<Path>) -> std::io::Result<Vec<PathBuf>> {
    let mut entries = fs::read_dir(path.as_ref()).await?;
    let mut markdown_files = Vec::new();

    while let Some(entry) = entries.next_entry().await? {
        if entry.file_type().await?.is_file() {
            let entry_path = entry.path();
            if entry_path
                .extension()
                .is_some_and(|extension| extension == "md")
            {
                markdown_files.push(entry_path);
            }
        }
    }

    markdown_files.sort();
    Ok(markdown_files)
}

async fn create_temp_file(target_path: &Path) -> std::io::Result<PathBuf> {
    let parent = target_path.parent().unwrap_or_else(|| Path::new("."));
    let file_name = target_path.file_name().ok_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "atomic_write requires a file path",
        )
    })?;

    for _attempt in 0..16_u8 {
        let temp_path = parent.join(format!(
            ".{}.tmp.{}.{}.{}",
            file_name.to_string_lossy(),
            std::process::id(),
            timestamp_nanos(),
            TEMP_FILE_COUNTER.fetch_add(1, Ordering::Relaxed),
        ));

        match OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temp_path)
            .await
        {
            Ok(file) => {
                drop(file);
                return Ok(temp_path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }

    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "failed to allocate a unique temporary file",
    ))
}

async fn write_and_rename(
    temp_path: &Path,
    final_path: &Path,
    contents: &[u8],
) -> std::io::Result<()> {
    let mut file = OpenOptions::new()
        .write(true)
        .truncate(true)
        .open(temp_path)
        .await?;
    file.write_all(contents).await?;
    file.sync_all().await?;
    drop(file);
    fs::rename(temp_path, final_path).await
}

fn timestamp_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_nanos())
}

#[cfg(test)]
mod tests {
    use super::{atomic_write, ensure_dir, list_md_files};
    use tempfile::tempdir;
    use tokio::fs;

    #[tokio::test]
    async fn ensure_dir_creates_nested_directories_idempotently() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let nested_dir = temp_dir.path().join("a").join("b").join("c");

        ensure_dir(&nested_dir)
            .await
            .expect("first ensure_dir call should succeed");
        ensure_dir(&nested_dir)
            .await
            .expect("second ensure_dir call should also succeed");

        assert!(nested_dir.is_dir());
    }

    #[tokio::test]
    async fn atomic_write_creates_parent_directories_and_writes_contents() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let file_path = temp_dir.path().join("decisions").join("choice.md");

        atomic_write(&file_path, b"## Context\nRust\n")
            .await
            .expect("atomic_write should create parent directories");

        let stored = fs::read_to_string(&file_path)
            .await
            .expect("written file should be readable");
        assert_eq!(stored, "## Context\nRust\n");
    }

    #[tokio::test]
    async fn atomic_write_replaces_existing_contents_without_leaving_temp_files() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let file_path = temp_dir.path().join("clients").join("hale-pet-door.md");

        atomic_write(&file_path, b"first version")
            .await
            .expect("initial write should succeed");
        atomic_write(&file_path, b"second version")
            .await
            .expect("overwrite should succeed");

        let stored = fs::read_to_string(&file_path)
            .await
            .expect("overwritten file should be readable");
        assert_eq!(stored, "second version");

        let siblings = list_md_files(file_path.parent().expect("file should have a parent"))
            .await
            .expect("directory listing should succeed");
        assert_eq!(siblings, vec![file_path.clone()]);

        let mut directory_entries =
            fs::read_dir(file_path.parent().expect("file should have a parent"))
                .await
                .expect("directory should be readable");
        let mut entry_names = Vec::new();
        while let Some(entry) = directory_entries
            .next_entry()
            .await
            .expect("directory iteration should succeed")
        {
            entry_names.push(entry.file_name().to_string_lossy().into_owned());
        }
        entry_names.sort();
        assert_eq!(entry_names, vec!["hale-pet-door.md"]);
    }

    #[tokio::test]
    async fn list_md_files_returns_sorted_markdown_files_only() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let directory = temp_dir.path().join("patterns");

        ensure_dir(&directory)
            .await
            .expect("pattern directory should be created");
        fs::write(directory.join("zeta.md"), b"zeta")
            .await
            .expect("zeta markdown file should be created");
        fs::write(directory.join("alpha.md"), b"alpha")
            .await
            .expect("alpha markdown file should be created");
        fs::write(directory.join("notes.txt"), b"text")
            .await
            .expect("non-markdown file should be created");
        ensure_dir(directory.join("nested"))
            .await
            .expect("nested directory should be created");
        fs::write(directory.join("nested").join("ignored.md"), b"nested")
            .await
            .expect("nested markdown file should be created");

        let markdown_files = list_md_files(&directory)
            .await
            .expect("listing markdown files should succeed");

        assert_eq!(
            markdown_files,
            vec![directory.join("alpha.md"), directory.join("zeta.md")]
        );
    }
}
