//! Filesystem path abstractions for workspace, store, and ledger locations.

use std::path::{Path, PathBuf};

use wg_types::PrimitiveType;

/// Root path wrapper for a WorkGraph workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspacePath {
    root: PathBuf,
}

impl WorkspacePath {
    /// Creates a new workspace path wrapper.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { root: path.into() }
    }

    /// Returns the underlying root path.
    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.root
    }

    /// Returns the hidden metadata directory path.
    #[must_use]
    pub fn hidden_dir(&self) -> PathBuf {
        self.root.join(".workgraph")
    }

    /// Returns the workspace config file path.
    #[must_use]
    pub fn config_path(&self) -> PathBuf {
        self.hidden_dir().join("config.yaml")
    }

    /// Returns the primitive store directory wrapper for a type.
    #[must_use]
    pub fn store_dir_for(&self, primitive_type: &PrimitiveType) -> StorePath {
        StorePath::new(self.root.join(primitive_type.directory_name()))
    }

    /// Returns the ledger file path wrapper.
    #[must_use]
    pub fn ledger_path(&self) -> LedgerPath {
        LedgerPath::new(self.hidden_dir().join("ledger.jsonl"))
    }
}

/// Store directory wrapper for primitive markdown files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorePath {
    path: PathBuf,
}

impl StorePath {
    /// Creates a new store path wrapper.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Returns the underlying path.
    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.path
    }

    /// Builds the markdown file path for a primitive ID.
    #[must_use]
    pub fn primitive_file(&self, id: &str) -> PathBuf {
        self.path.join(format!("{id}.md"))
    }
}

/// Ledger file path wrapper.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerPath {
    path: PathBuf,
}

impl LedgerPath {
    /// Creates a new ledger path wrapper.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Returns the underlying ledger file path.
    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.path
    }
}
