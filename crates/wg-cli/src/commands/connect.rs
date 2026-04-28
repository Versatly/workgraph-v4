//! Implementation of hosted connection profile commands.

use anyhow::{Context, bail};
use reqwest::StatusCode;
use serde::Deserialize;

use crate::app::AppContext;
use crate::output::{ConnectOutput, WhoamiOutput};
use wg_types::{ActorId, RemoteAccessScope, RemoteWorkspaceConfig};

#[derive(Debug, Deserialize)]
struct HostedHealthResponse {
    actor_id: String,
    access_scope: RemoteAccessScope,
    credential_id: String,
}

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
    let server = server.trim_end_matches('/').to_owned();
    let health = fetch_health(&server, token).await?;
    if health.actor_id != actor_id {
        bail!(
            "hosted credential is bound to actor '{}' and cannot connect as '{}'",
            health.actor_id,
            actor_id
        );
    }

    let mut config = app.load_config().await?;
    config.default_actor_id = Some(ActorId::new(actor_id));
    config.remote = Some(RemoteWorkspaceConfig {
        server_url: server.clone(),
        auth_token: token.to_owned(),
        actor_id: ActorId::new(actor_id),
        access_scope: health.access_scope,
    });
    app.write_config(&config).await?;

    Ok(ConnectOutput {
        mode: "hosted".to_owned(),
        server_url: server,
        actor_id: actor_id.to_owned(),
        access_scope: health.access_scope.as_str().to_owned(),
        credential_id: health.credential_id,
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
        hosted_server: config
            .remote
            .as_ref()
            .map(|remote| remote.server_url.clone()),
        hosted_profile: config.remote.as_ref().map(|_| "default".to_owned()),
        access_scope: config
            .remote
            .as_ref()
            .map(|remote| remote.access_scope.as_str().to_owned()),
    })
}

async fn fetch_health(server: &str, token: &str) -> anyhow::Result<HostedHealthResponse> {
    let endpoint = format!("{server}/v1/health");
    let response = reqwest::Client::new()
        .get(endpoint)
        .bearer_auth(token)
        .send()
        .await
        .context("failed to reach hosted WorkGraph health endpoint")?;
    if response.status() == StatusCode::UNAUTHORIZED {
        bail!("hosted credential was rejected by the remote server");
    }
    response
        .error_for_status()
        .context("hosted WorkGraph health probe failed")?
        .json::<HostedHealthResponse>()
        .await
        .context("failed to decode hosted WorkGraph health response")
}
