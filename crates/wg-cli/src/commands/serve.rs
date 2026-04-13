//! Implementation of hosted `serve` and MCP-serving commands.

use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use anyhow::Context;
use async_trait::async_trait;
use wg_api::{ApiServerConfig, RemoteCommandExecutor};
use wg_mcp::McpCommandExecutor;
use wg_types::{ActorId, RemoteCommandRequest, RemoteCommandResponse};

use crate::output::ServeOutput;

/// Builds a description of an HTTP serve endpoint.
#[must_use]
pub fn describe_http(app: &crate::app::AppContext, listen: &str) -> ServeOutput {
    ServeOutput {
        transport: "http".to_owned(),
        endpoint: Some(format!("http://{listen}")),
        workspace_root: app.root().display().to_string(),
    }
}

/// Serves the current workspace over the hosted HTTP API.
///
/// # Errors
///
/// Returns an error when the socket address is invalid or the server fails.
pub async fn run_http(
    app: &crate::app::AppContext,
    listen: &str,
    token: &str,
) -> anyhow::Result<()> {
    let listen_addr: SocketAddr = listen
        .parse()
        .with_context(|| format!("invalid listen address '{listen}'"))?;
    let config = ApiServerConfig {
        listen_addr,
        workspace_root: app.root().to_path_buf(),
        auth_token: token.to_owned(),
    };
    wg_api::serve(config, Arc::new(CliRemoteExecutor)).await
}

/// Builds a description of the MCP stdio endpoint.
#[must_use]
pub fn describe_mcp(app: &crate::app::AppContext) -> ServeOutput {
    ServeOutput {
        transport: "mcp".to_owned(),
        endpoint: Some("stdio".to_owned()),
        workspace_root: app.root().display().to_string(),
    }
}

/// Serves the MCP stdio adapter for the current workspace.
///
/// # Errors
///
/// Returns an error when the MCP protocol loop fails.
pub async fn run_mcp(_app: &crate::app::AppContext) -> anyhow::Result<()> {
    wg_mcp::serve_stdio(Arc::new(CliRemoteExecutor)).await
}

struct CliRemoteExecutor;

#[async_trait]
impl RemoteCommandExecutor for CliRemoteExecutor {
    async fn execute(
        &self,
        workspace_root: PathBuf,
        request: RemoteCommandRequest,
    ) -> anyhow::Result<RemoteCommandResponse> {
        let rendered =
            execute_local_request(&request.args, workspace_root.as_path(), request.actor_id.as_deref())
                .await?;
        Ok(RemoteCommandResponse {
            success: true,
            rendered,
        })
    }
}

#[async_trait]
impl McpCommandExecutor for CliRemoteExecutor {
    async fn execute(&self, request: RemoteCommandRequest) -> anyhow::Result<RemoteCommandResponse> {
        let current_dir = std::env::current_dir().context("failed to determine current directory")?;
        let rendered =
            execute_local_request(&request.args, current_dir.as_path(), request.actor_id.as_deref())
                .await?;
        Ok(RemoteCommandResponse {
            success: true,
            rendered,
        })
    }
}

async fn execute_local_request(
    args: &[String],
    workspace_root: &std::path::Path,
    actor_id: Option<&str>,
) -> anyhow::Result<String> {
    let app = crate::app::AppContext::with_actor(
        workspace_root.to_path_buf(),
        actor_id.map(ActorId::new),
    );
    let cli = crate::args::parse_cli(args.iter().cloned())?;
    let output = crate::commands::execute(&app, cli.command).await?;
    crate::output::render_success(&output, cli.json || cli.format.is_json())
}
