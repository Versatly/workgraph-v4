//! Shared application context and workspace-loading helpers for CLI commands.

use std::path::{Path, PathBuf};

use anyhow::Context;
use tokio::fs;
use wg_ledger::{LedgerCursor, LedgerReader};
use wg_paths::WorkspacePath;
use wg_registry::RuntimeRegistry;
use wg_types::{ActorId, HostedCredentialStore, LedgerEntry, Registry, WorkgraphConfig};

/// Shared command context for operating on a single WorkGraph workspace.
#[derive(Debug, Clone)]
pub struct AppContext {
    root: PathBuf,
    workspace: WorkspacePath,
    actor_override: Option<ActorId>,
}

impl AppContext {
    /// Creates a new application context rooted at the provided workspace path.
    #[must_use]
    pub fn new(root: PathBuf) -> Self {
        Self {
            workspace: WorkspacePath::new(root.clone()),
            root,
            actor_override: None,
        }
    }

    /// Creates a new application context with a per-request actor override.
    #[must_use]
    pub fn with_actor(root: PathBuf, actor_override: Option<ActorId>) -> Self {
        Self {
            workspace: WorkspacePath::new(root.clone()),
            root,
            actor_override,
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

    /// Returns the per-request actor override when present.
    #[must_use]
    pub fn actor_override(&self) -> Option<&ActorId> {
        self.actor_override.as_ref()
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

    /// Returns the path to the hosted credential store.
    #[must_use]
    pub fn credentials_path(&self) -> PathBuf {
        self.metadata_dir_path().join("credentials.yaml")
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

    /// Loads the persisted workspace configuration file when it exists.
    ///
    /// # Errors
    ///
    /// Returns an error when the configuration file exists but cannot be read or parsed.
    pub async fn try_load_config(&self) -> anyhow::Result<Option<WorkgraphConfig>> {
        let path = self.config_path();
        if !fs::try_exists(&path)
            .await
            .with_context(|| format!("failed to inspect config file '{}'", path.display()))?
        {
            return Ok(None);
        }

        self.load_config().await.map(Some)
    }

    /// Resolves the effective actor for writes, preferring a per-request override.
    ///
    /// # Errors
    ///
    /// Returns an error when the persisted config cannot be loaded.
    pub async fn effective_actor_id(&self) -> anyhow::Result<ActorId> {
        if let Some(actor) = self.actor_override() {
            return Ok(actor.clone());
        }

        let config = self.load_config().await?;
        Ok(config
            .default_actor_id
            .unwrap_or_else(|| ActorId::new("cli")))
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

    /// Loads the hosted credential store, returning an empty store when none exists.
    ///
    /// # Errors
    ///
    /// Returns an error when the credential store cannot be read or parsed.
    pub async fn load_credentials(&self) -> anyhow::Result<HostedCredentialStore> {
        let path = self.credentials_path();
        if !fs::try_exists(&path)
            .await
            .with_context(|| format!("failed to inspect credential store '{}'", path.display()))?
        {
            return Ok(HostedCredentialStore::default());
        }

        let encoded = fs::read_to_string(&path)
            .await
            .with_context(|| format!("failed to read credential store '{}'", path.display()))?;

        serde_yaml::from_str(&encoded)
            .with_context(|| format!("failed to parse credential store '{}'", path.display()))
    }

    /// Writes the hosted credential store atomically.
    ///
    /// # Errors
    ///
    /// Returns an error when serialization or persistence fails.
    pub async fn write_credentials(&self, store: &HostedCredentialStore) -> anyhow::Result<()> {
        self.ensure_metadata_dir().await?;
        let encoded =
            serde_yaml::to_string(store).context("failed to serialize credential store")?;
        wg_fs::atomic_write(self.credentials_path(), encoded.as_bytes())
            .await
            .with_context(|| {
                format!(
                    "failed to write credential store '{}'",
                    self.credentials_path().display()
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
