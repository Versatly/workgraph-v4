//! Implementation of the `workgraph init` command.

use anyhow::Context;
use tokio::fs;
use tracing::debug;
use wg_registry::RuntimeRegistry;
use wg_types::ActorId;

use crate::app::AppContext;
use crate::output::InitOutput;
use crate::util::workspace::default_config;

/// Initializes the workspace metadata and primitive directories.
///
/// # Errors
///
/// Returns an error when metadata files or primitive directories cannot be created.
pub async fn handle(app: &AppContext) -> anyhow::Result<InitOutput> {
    let registry = RuntimeRegistry::with_builtins().into_registry();

    app.ensure_metadata_dir().await?;
    app.ensure_primitive_dirs(&registry).await?;

    if !fs::try_exists(app.registry_path())
        .await
        .context("failed to inspect registry file")?
    {
        app.write_registry(&registry).await?;
    }

    if !fs::try_exists(app.ledger_path())
        .await
        .context("failed to inspect ledger file")?
    {
        wg_fs::atomic_write(app.ledger_path(), b"")
            .await
            .with_context(|| {
                format!(
                    "failed to create ledger file '{}'",
                    app.ledger_path().display()
                )
            })?;
    }

    if !fs::try_exists(app.config_path())
        .await
        .context("failed to inspect config file")?
    {
        let config = default_config(app, Some(ActorId::new("cli")));
        app.write_config(&config).await?;
    }

    let config = app.load_config().await?;
    debug!(
        "initialized workgraph workspace at {}",
        app.root().display()
    );

    Ok(InitOutput {
        config,
        registry_path: app.registry_path().display().to_string(),
        ledger_path: app.ledger_path().display().to_string(),
        config_path: app.config_path().display().to_string(),
        created_directories: registry
            .list_types()
            .iter()
            .map(|primitive_type| primitive_type.directory.clone())
            .collect(),
    })
}
