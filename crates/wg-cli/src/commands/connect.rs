//! Implementation of hosted connection profile commands.

use crate::app::AppContext;
use crate::output::{ConnectOutput, WhoamiOutput};
use wg_types::{ActorId, RemoteWorkspaceConfig};

/// Connects the current local CLI profile to a hosted WorkGraph server.
///
/// # Errors
///
/// Returns an error when the workspace config cannot be loaded or written.
pub async fn handle(
    app: &AppContext,
    server: &str,
    token: &str,
    actor_id: &str,
) -> anyhow::Result<ConnectOutput> {
    let mut config = app.load_config().await?;
    config.default_actor_id = Some(ActorId::new(actor_id));
    config.remote = Some(RemoteWorkspaceConfig {
        server_url: server.trim_end_matches('/').to_owned(),
        auth_token: token.to_owned(),
        actor_id: ActorId::new(actor_id),
    });
    app.write_config(&config).await?;

    Ok(ConnectOutput {
        mode: "hosted".to_owned(),
        server_url: server.trim_end_matches('/').to_owned(),
        actor_id: actor_id.to_owned(),
        config,
    })
}

/// Returns the effective local/hosted CLI identity for this workspace.
///
/// # Errors
///
/// Returns an error when the workspace config cannot be loaded.
pub async fn whoami(app: &AppContext) -> anyhow::Result<WhoamiOutput> {
    let config = app.load_config().await?;
    let actor_id = config
        .remote
        .as_ref()
        .map(|remote| remote.actor_id.to_string())
        .or_else(|| config.default_actor_id.as_ref().map(ToString::to_string))
        .unwrap_or_else(|| "cli".to_owned());

    Ok(WhoamiOutput {
        mode: if config.remote.is_some() {
            "hosted".to_owned()
        } else {
            "local".to_owned()
        },
        actor_id,
        workspace_id: config.workspace_id.to_string(),
        workspace_name: config.workspace_name,
        hosted_server: config.remote.as_ref().map(|remote| remote.server_url.clone()),
        hosted_profile: config.remote.as_ref().map(|_| "default".to_owned()),
    })
}
