//! Filesystem path abstractions for WorkGraph workspaces.

use std::path::{Path, PathBuf};

/// Root path for a WorkGraph workspace.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct WorkspacePath(PathBuf);

/// Filesystem path inside the markdown primitive store.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct StorePath(PathBuf);

/// Filesystem path to the append-only ledger file.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LedgerPath(PathBuf);

impl WorkspacePath {
    /// Creates a new workspace path wrapper from any owned path-like value.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    /// Returns the underlying workspace root path.
    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    /// Consumes the wrapper and returns the owned workspace root path.
    #[must_use]
    pub fn into_inner(self) -> PathBuf {
        self.0
    }

    /// Returns the directory that stores primitives for the given type name.
    #[must_use]
    pub fn type_dir(&self, type_name: &str) -> StorePath {
        StorePath(self.0.join(pluralized_type_dir(type_name)))
    }

    /// Returns the markdown file path for a primitive type and identifier.
    #[must_use]
    pub fn primitive_path(&self, type_name: &str, id: &str) -> StorePath {
        let file_name = format!("{id}.md");
        StorePath(self.type_dir(type_name).into_inner().join(file_name))
    }

    /// Returns the path to the workspace ledger file.
    #[must_use]
    pub fn ledger_path(&self) -> LedgerPath {
        LedgerPath(self.0.join(".workgraph").join("ledger.jsonl"))
    }
}

impl StorePath {
    /// Creates a new store path wrapper from any owned path-like value.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    /// Returns the underlying store path.
    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    /// Consumes the wrapper and returns the owned store path.
    #[must_use]
    pub fn into_inner(self) -> PathBuf {
        self.0
    }
}

impl LedgerPath {
    /// Creates a new ledger path wrapper from any owned path-like value.
    #[must_use]
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self(path.into())
    }

    /// Returns the underlying ledger path.
    #[must_use]
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    /// Consumes the wrapper and returns the owned ledger path.
    #[must_use]
    pub fn into_inner(self) -> PathBuf {
        self.0
    }
}

impl AsRef<Path> for WorkspacePath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl AsRef<Path> for StorePath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

impl AsRef<Path> for LedgerPath {
    fn as_ref(&self) -> &Path {
        self.as_path()
    }
}

fn pluralized_type_dir(type_name: &str) -> String {
    match type_name {
        "org" => "orgs".to_owned(),
        "team" => "teams".to_owned(),
        "person" => "people".to_owned(),
        "agent" => "agents".to_owned(),
        "client" => "clients".to_owned(),
        "project" => "projects".to_owned(),
        "decision" => "decisions".to_owned(),
        "pattern" => "patterns".to_owned(),
        "lesson" => "lessons".to_owned(),
        "policy" => "policies".to_owned(),
        "relationship" => "relationships".to_owned(),
        "strategic_note" => "strategic_notes".to_owned(),
        "thread" => "threads".to_owned(),
        "run" => "runs".to_owned(),
        "mission" => "missions".to_owned(),
        "trigger" => "triggers".to_owned(),
        other => format!("{other}s"),
    }
}

#[cfg(test)]
mod tests {
    use super::{LedgerPath, StorePath, WorkspacePath};
    use std::path::PathBuf;

    #[test]
    fn built_in_types_map_to_expected_directories() {
        let workspace = WorkspacePath::new("/tmp/workgraph");

        let expectations = [
            ("org", "/tmp/workgraph/orgs"),
            ("team", "/tmp/workgraph/teams"),
            ("person", "/tmp/workgraph/people"),
            ("agent", "/tmp/workgraph/agents"),
            ("client", "/tmp/workgraph/clients"),
            ("project", "/tmp/workgraph/projects"),
            ("decision", "/tmp/workgraph/decisions"),
            ("pattern", "/tmp/workgraph/patterns"),
            ("lesson", "/tmp/workgraph/lessons"),
            ("policy", "/tmp/workgraph/policies"),
            ("relationship", "/tmp/workgraph/relationships"),
            ("strategic_note", "/tmp/workgraph/strategic_notes"),
            ("thread", "/tmp/workgraph/threads"),
            ("run", "/tmp/workgraph/runs"),
            ("mission", "/tmp/workgraph/missions"),
            ("trigger", "/tmp/workgraph/triggers"),
        ];

        for (type_name, expected_dir) in expectations {
            assert_eq!(
                workspace.type_dir(type_name).as_path(),
                PathBuf::from(expected_dir)
            );
        }
    }

    #[test]
    fn unknown_types_fall_back_to_appending_s() {
        let workspace = WorkspacePath::new("/tmp/workgraph");

        assert_eq!(
            workspace.type_dir("checkpoint").as_path(),
            PathBuf::from("/tmp/workgraph/checkpoints")
        );
    }

    #[test]
    fn primitive_paths_point_to_markdown_files_inside_type_directories() {
        let workspace = WorkspacePath::new("/tmp/workgraph");

        assert_eq!(
            workspace
                .primitive_path("decision", "rust-for-workgraph-v4")
                .as_path(),
            PathBuf::from("/tmp/workgraph/decisions/rust-for-workgraph-v4.md")
        );
        assert_eq!(
            workspace
                .primitive_path("checkpoint", "daily-sync")
                .as_path(),
            PathBuf::from("/tmp/workgraph/checkpoints/daily-sync.md")
        );
    }

    #[test]
    fn ledger_path_uses_hidden_workgraph_directory() {
        let workspace = WorkspacePath::new("/tmp/workgraph");

        assert_eq!(
            workspace.ledger_path().as_path(),
            PathBuf::from("/tmp/workgraph/.workgraph/ledger.jsonl")
        );
    }

    #[test]
    fn wrappers_round_trip_owned_paths() {
        let workspace = WorkspacePath::new("/tmp/workgraph");
        let store = StorePath::new("/tmp/workgraph/clients");
        let ledger = LedgerPath::new("/tmp/workgraph/.workgraph/ledger.jsonl");

        assert_eq!(
            workspace.clone().into_inner(),
            PathBuf::from("/tmp/workgraph")
        );
        assert_eq!(
            store.clone().into_inner(),
            PathBuf::from("/tmp/workgraph/clients")
        );
        assert_eq!(
            ledger.clone().into_inner(),
            PathBuf::from("/tmp/workgraph/.workgraph/ledger.jsonl")
        );

        assert_eq!(workspace.as_ref(), workspace.as_path());
        assert_eq!(store.as_ref(), store.as_path());
        assert_eq!(ledger.as_ref(), ledger.as_path());
    }
}
