//! Shared application context and workspace-loading helpers for CLI commands.

use std::path::{Path, PathBuf};

use anyhow::Context;
use tokio::fs;
use wg_ledger::{LedgerCursor, LedgerReader};
use wg_paths::WorkspacePath;
use wg_registry::RuntimeRegistry;
use wg_types::{LedgerEntry, Registry, WorkgraphConfig};

/// Shared command context for operating on a single WorkGraph workspace.
#[derive(Debug, Clone)]
pub struct AppContext {
    root: PathBuf,
    workspace: WorkspacePath,
}

impl AppContext {
    /// Creates a new application context rooted at the provided workspace path.
    #[must_use]
    pub fn new(root: PathBuf) -> Self {
        Self {
            workspace: WorkspacePath::new(root.clone()),
            root,
        }
    }

    /// Returns the workspace root path.
    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Returns the workspace path wrapper used by lower-level crates.
    #[must_use]
    pub fn workspace(&self) -> &WorkspacePath {
        &self.workspace
    }

    /// Returns the path to the hidden `.workgraph` metadata directory.
    #[must_use]
    pub fn metadata_dir_path(&self) -> PathBuf {
        self.root.join(".workgraph")
    }

    /// Returns the path to the workspace registry file.
    #[must_use]
    pub fn registry_path(&self) -> PathBuf {
        self.metadata_dir_path().join("registry.yaml")
    }

    /// Returns the path to the workspace configuration file.
    #[must_use]
    pub fn config_path(&self) -> PathBuf {
        self.metadata_dir_path().join("config.yaml")
    }

    /// Returns the path to the immutable workspace ledger file.
    #[must_use]
    pub fn ledger_path(&self) -> PathBuf {
        self.workspace.ledger_path().into_inner()
    }

    /// Ensures that the metadata directory exists.
    ///
    /// # Errors
    ///
    /// Returns an error when the metadata directory cannot be created.
    pub async fn ensure_metadata_dir(&self) -> anyhow::Result<()> {
        wg_fs::ensure_dir(self.metadata_dir_path())
            .await
            .context("failed to create .workgraph directory")
    }

    /// Loads the serialized primitive registry from disk.
    ///
    /// # Errors
    ///
    /// Returns an error when the registry file cannot be read or parsed.
    pub async fn load_registry(&self) -> anyhow::Result<Registry> {
        let path = self.registry_path();
        let encoded = fs::read_to_string(&path)
            .await
            .with_context(|| format!("failed to read registry file '{}'", path.display()))?;

        serde_yaml::from_str(&encoded)
            .with_context(|| format!("failed to parse registry file '{}'", path.display()))
    }

    /// Loads a runtime registry wrapper from the serialized registry file.
    ///
    /// # Errors
    ///
    /// Returns an error when the serialized registry cannot be loaded or contains invalid entries.
    pub async fn load_runtime_registry(&self) -> anyhow::Result<RuntimeRegistry> {
        RuntimeRegistry::from_registry(self.load_registry().await?)
            .context("failed to build runtime registry from serialized registry")
    }

    /// Loads the persisted workspace configuration file.
    ///
    /// # Errors
    ///
    /// Returns an error when the configuration file cannot be read or parsed.
    pub async fn load_config(&self) -> anyhow::Result<WorkgraphConfig> {
        let path = self.config_path();
        let encoded = fs::read_to_string(&path)
            .await
            .with_context(|| format!("failed to read config file '{}'", path.display()))?;

        serde_yaml::from_str(&encoded)
            .with_context(|| format!("failed to parse config file '{}'", path.display()))
    }

    /// Reads every ledger entry currently recorded for the workspace.
    ///
    /// # Errors
    ///
    /// Returns an error when the ledger cannot be read or decoded.
    pub async fn read_ledger_entries(&self) -> anyhow::Result<Vec<LedgerEntry>> {
        let reader = LedgerReader::new(self.root.clone());
        let (entries, _) = reader
            .read_from(LedgerCursor::default())
            .await
            .context("failed to read ledger entries")?;
        Ok(entries)
    }

    /// Atomically writes the serialized registry file to disk.
    ///
    /// # Errors
    ///
    /// Returns an error when serialization or persistence fails.
    pub async fn write_registry(&self, registry: &Registry) -> anyhow::Result<()> {
        let encoded = serde_yaml::to_string(registry).context("failed to serialize registry")?;
        wg_fs::atomic_write(self.registry_path(), encoded.as_bytes())
            .await
            .with_context(|| {
                format!(
                    "failed to write registry file '{}'",
                    self.registry_path().display()
                )
            })
    }

    /// Atomically writes the serialized workspace configuration file to disk.
    ///
    /// # Errors
    ///
    /// Returns an error when serialization or persistence fails.
    pub async fn write_config(&self, config: &WorkgraphConfig) -> anyhow::Result<()> {
        let encoded =
            serde_yaml::to_string(config).context("failed to serialize workspace config")?;
        wg_fs::atomic_write(self.config_path(), encoded.as_bytes())
            .await
            .with_context(|| {
                format!(
                    "failed to write config file '{}'",
                    self.config_path().display()
                )
            })
    }

    /// Ensures that every built-in primitive directory exists under the workspace root.
    ///
    /// # Errors
    ///
    /// Returns an error when any directory cannot be created.
    pub async fn ensure_primitive_dirs(&self, registry: &Registry) -> anyhow::Result<()> {
        for primitive_type in registry.list_types() {
            fs::create_dir_all(self.root.join(&primitive_type.directory))
                .await
                .with_context(|| {
                    format!(
                        "failed to create primitive directory '{}'",
                        primitive_type.directory
                    )
                })?;
        }

        Ok(())
    }
}
