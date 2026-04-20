//! Workspace registry loading helpers shared by store consumers.

use tokio::fs;
use wg_error::Result;
use wg_paths::WorkspacePath;
use wg_types::Registry;

/// Loads the workspace registry file when present, otherwise falls back to built-ins.
///
/// # Errors
///
/// Returns an error when the registry file exists but cannot be read or parsed.
pub async fn load_workspace_registry(workspace: &WorkspacePath) -> Result<Registry> {
    let path = workspace.as_path().join(".workgraph").join("registry.yaml");
    if !fs::try_exists(&path).await? {
        return Ok(Registry::builtins());
    }

    let encoded = fs::read_to_string(&path).await?;
    let registry = serde_yaml::from_str(&encoded)
        .map_err(|error| wg_error::WorkgraphError::EncodingError(error.to_string()))?;
    Ok(registry)
}
