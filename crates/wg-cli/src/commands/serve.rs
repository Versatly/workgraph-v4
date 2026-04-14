//! Implementation of hosted `serve` and MCP-serving commands.

use std::{net::SocketAddr, path::Path, path::PathBuf, sync::Arc};

use anyhow::{Context, bail};
use async_trait::async_trait;
use wg_api::{ApiServerConfig, RemoteCommandExecutor};
use wg_mcp::McpCommandExecutor;
use wg_types::{ActorId, RemoteAccessScope, RemoteCommandRequest, RemoteCommandResponse};

use crate::{args::Command, output::ServeOutput};

/// Builds a description of an HTTP serve endpoint.
///
/// # Errors
///
/// Returns an error when the requested actor/scope combination is invalid.
pub fn describe_http(
    app: &crate::app::AppContext,
    listen: &str,
    actor_id: Option<&str>,
    access_scope: Option<RemoteAccessScope>,
) -> anyhow::Result<ServeOutput> {
    let access_scope = resolve_access_scope(actor_id, access_scope);
    validate_access_config(actor_id, access_scope)?;
    Ok(ServeOutput {
        transport: "http".to_owned(),
        endpoint: Some(format!("http://{listen}")),
        workspace_root: app.root().display().to_string(),
        actor_id: actor_id.map(ToOwned::to_owned),
        access_scope: access_scope.as_str().to_owned(),
    })
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
    actor_id: Option<&str>,
    access_scope: Option<RemoteAccessScope>,
) -> anyhow::Result<()> {
    let listen_addr: SocketAddr = listen
        .parse()
        .with_context(|| format!("invalid listen address '{listen}'"))?;
    let access_scope = resolve_access_scope(actor_id, access_scope);
    validate_access_config(actor_id, access_scope)?;
    let config = ApiServerConfig {
        listen_addr,
        workspace_root: app.root().to_path_buf(),
        auth_token: token.to_owned(),
        actor_id: actor_id
            .expect("validate_access_config requires actor_id for hosted serve")
            .to_owned(),
        access_scope,
    };
    let executor = CliRemoteExecutor {
        workspace_root: None,
        bound_actor_id: actor_id.map(ActorId::new),
        access_scope,
    };
    wg_api::serve(config, Arc::new(executor)).await
}

/// Builds a description of the MCP stdio endpoint.
///
/// # Errors
///
/// Returns an error when the requested actor/scope combination is invalid.
pub fn describe_mcp(
    app: &crate::app::AppContext,
    actor_id: Option<&str>,
    access_scope: Option<RemoteAccessScope>,
) -> anyhow::Result<ServeOutput> {
    let access_scope = resolve_access_scope(actor_id, access_scope);
    validate_access_config(actor_id, access_scope)?;
    Ok(ServeOutput {
        transport: "mcp".to_owned(),
        endpoint: Some("stdio".to_owned()),
        workspace_root: app.root().display().to_string(),
        actor_id: actor_id.map(ToOwned::to_owned),
        access_scope: access_scope.as_str().to_owned(),
    })
}

/// Serves the MCP stdio adapter for the current workspace.
///
/// # Errors
///
/// Returns an error when the MCP protocol loop fails.
pub async fn run_mcp(
    app: &crate::app::AppContext,
    actor_id: Option<&str>,
    access_scope: Option<RemoteAccessScope>,
) -> anyhow::Result<()> {
    let access_scope = resolve_access_scope(actor_id, access_scope);
    validate_access_config(actor_id, access_scope)?;
    let executor = CliRemoteExecutor {
        workspace_root: Some(app.root().to_path_buf()),
        bound_actor_id: actor_id.map(ActorId::new),
        access_scope,
    };
    wg_mcp::serve_stdio(Arc::new(executor)).await
}

struct CliRemoteExecutor {
    workspace_root: Option<PathBuf>,
    bound_actor_id: Option<ActorId>,
    access_scope: RemoteAccessScope,
}

#[async_trait]
impl RemoteCommandExecutor for CliRemoteExecutor {
    async fn execute(
        &self,
        workspace_root: PathBuf,
        request: RemoteCommandRequest,
    ) -> anyhow::Result<RemoteCommandResponse> {
        execute_local_request(
            &request.args,
            workspace_root.as_path(),
            self.bound_actor_id.as_ref(),
            self.access_scope,
            request.actor_id.as_deref(),
        )
        .await
    }
}

#[async_trait]
impl McpCommandExecutor for CliRemoteExecutor {
    async fn execute(
        &self,
        request: RemoteCommandRequest,
    ) -> anyhow::Result<RemoteCommandResponse> {
        let workspace_root = self
            .workspace_root
            .as_ref()
            .context("missing workspace root for MCP executor")?;
        execute_local_request(
            &request.args,
            workspace_root.as_path(),
            self.bound_actor_id.as_ref(),
            self.access_scope,
            request.actor_id.as_deref(),
        )
        .await
    }
}

async fn execute_local_request(
    args: &[String],
    workspace_root: &Path,
    bound_actor_id: Option<&ActorId>,
    access_scope: RemoteAccessScope,
    requested_actor_id: Option<&str>,
) -> anyhow::Result<RemoteCommandResponse> {
    let parsed = crate::args::parse_cli(args.iter().cloned());
    match parsed {
        Ok(cli) => {
            let json_output = cli.json || cli.format.is_json();
            match authorize_remote_command(
                &cli.command,
                bound_actor_id,
                access_scope,
                requested_actor_id,
            ) {
                Ok(actor_override) => {
                    crate::execute_remote_contract(args, workspace_root, actor_override).await
                }
                Err(error) => {
                    crate::render_failure_contract(Some(cli.command.name()), &error, json_output)
                }
            }
        }
        Err(error) => {
            let error: anyhow::Error = error.into();
            let json_output = crate::parse_output_format(
                &args
                    .iter()
                    .cloned()
                    .map(std::ffi::OsString::from)
                    .collect::<Vec<_>>(),
            )
            .is_json();
            crate::render_failure_contract(None, &error, json_output)
        }
    }
}

fn resolve_access_scope(
    actor_id: Option<&str>,
    access_scope: Option<RemoteAccessScope>,
) -> RemoteAccessScope {
    access_scope.unwrap_or_else(|| {
        if actor_id.is_some() {
            RemoteAccessScope::Operate
        } else {
            RemoteAccessScope::Read
        }
    })
}

fn validate_access_config(
    actor_id: Option<&str>,
    access_scope: RemoteAccessScope,
) -> anyhow::Result<()> {
    if !matches!(access_scope, RemoteAccessScope::Read) && actor_id.is_none() {
        bail!(
            "remote access scope '{}' requires --actor-id so hosted writes stay durably attributable",
            access_scope
        );
    }
    Ok(())
}

fn authorize_remote_command(
    command: &Command,
    bound_actor_id: Option<&ActorId>,
    access_scope: RemoteAccessScope,
    requested_actor_id: Option<&str>,
) -> anyhow::Result<Option<ActorId>> {
    let required_scope = command.required_remote_access_scope();
    if !access_scope.allows(required_scope) {
        bail!(
            "remote credential scope '{}' does not allow command '{}' (requires '{}')",
            access_scope,
            command.name(),
            required_scope
        );
    }

    if let Some(bound_actor_id) = bound_actor_id {
        if let Some(requested_actor_id) = requested_actor_id {
            if requested_actor_id != bound_actor_id.as_str() {
                bail!(
                    "remote credential is bound to actor '{}' and cannot impersonate '{}'",
                    bound_actor_id,
                    requested_actor_id
                );
            }
        }
        return Ok(Some(bound_actor_id.clone()));
    }

    if matches!(
        required_scope,
        RemoteAccessScope::Operate | RemoteAccessScope::Admin
    ) {
        bail!(
            "remote command '{}' requires a bound actor credential",
            command.name()
        );
    }

    Ok(requested_actor_id.map(ActorId::new))
}

#[cfg(test)]
mod tests {
    use super::{authorize_remote_command, resolve_access_scope, validate_access_config};
    use crate::args::Command;
    use wg_types::{ActorId, RemoteAccessScope};

    #[test]
    fn default_scope_is_read_without_bound_actor() {
        assert_eq!(resolve_access_scope(None, None), RemoteAccessScope::Read);
        assert_eq!(
            resolve_access_scope(Some("agent:cursor"), None),
            RemoteAccessScope::Operate
        );
    }

    #[test]
    fn write_scopes_require_bound_actor() {
        assert!(validate_access_config(None, RemoteAccessScope::Read).is_ok());
        assert!(validate_access_config(Some("agent:cursor"), RemoteAccessScope::Operate).is_ok());
        assert!(validate_access_config(None, RemoteAccessScope::Operate).is_err());
        assert!(validate_access_config(None, RemoteAccessScope::Admin).is_err());
    }

    #[test]
    fn actor_bound_credentials_cannot_impersonate_other_actors() {
        let command = Command::Claim {
            thread_id: "thread-1".to_owned(),
        };
        let actor = ActorId::new("agent:cursor");
        assert_eq!(
            authorize_remote_command(
                &command,
                Some(&actor),
                RemoteAccessScope::Operate,
                Some("agent:cursor")
            )
            .expect("matching actor should be allowed"),
            Some(actor.clone())
        );
        assert!(
            authorize_remote_command(
                &command,
                Some(&actor),
                RemoteAccessScope::Operate,
                Some("person:pedro")
            )
            .is_err()
        );
    }

    #[test]
    fn insufficient_scope_is_rejected() {
        let command = Command::Create {
            primitive_type: "thread".to_owned(),
            title: Some("Remote Thread".to_owned()),
            fields: Vec::new(),
            dry_run: false,
            stdin: false,
        };
        let actor = ActorId::new("agent:cursor");
        assert!(
            authorize_remote_command(
                &command,
                Some(&actor),
                RemoteAccessScope::Operate,
                Some("agent:cursor")
            )
            .is_err()
        );
    }
}
