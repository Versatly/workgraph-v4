#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Hosted HTTP adapter for remotely executing WorkGraph CLI contracts.

use std::{net::SocketAddr, path::PathBuf, sync::Arc};

use anyhow::Context;
use async_trait::async_trait;
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use serde::Serialize;
use sha2::{Digest, Sha256};
use wg_types::{RemoteAccessScope, RemoteCommandRequest, RemoteCommandResponse};

/// Remote command executor used by the hosted API server.
#[async_trait]
pub trait RemoteCommandExecutor: Send + Sync {
    /// Executes one remotely supplied WorkGraph command request.
    async fn execute(
        &self,
        workspace_root: PathBuf,
        credential: AuthenticatedCredential,
        request: RemoteCommandRequest,
    ) -> anyhow::Result<RemoteCommandResponse>;
}

/// One hosted HTTP credential accepted by the server.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiCredential {
    /// Stable credential identifier.
    pub id: String,
    /// Actor identity bound to the credential.
    pub actor_id: String,
    /// Governance scope granted to this credential.
    pub access_scope: RemoteAccessScope,
    /// SHA-256 hash of the bearer token accepted for this credential.
    pub token_hash: String,
}

/// Credential details authenticated for a single HTTP request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthenticatedCredential {
    /// Stable credential identifier.
    pub id: String,
    /// Actor identity bound to the credential.
    pub actor_id: String,
    /// Governance scope granted to this credential.
    pub access_scope: RemoteAccessScope,
}

/// HTTP server configuration for one hosted WorkGraph workspace.
#[derive(Debug, Clone)]
pub struct ApiServerConfig {
    /// Socket address to bind the HTTP server to.
    pub listen_addr: SocketAddr,
    /// Filesystem root of the hosted workspace.
    pub workspace_root: PathBuf,
    /// Bearer credentials accepted by the server.
    pub credentials: Vec<ApiCredential>,
}

#[derive(Clone)]
struct ApiState {
    config: ApiServerConfig,
    executor: Arc<dyn RemoteCommandExecutor>,
}

/// Lightweight health probe returned by the hosted server.
#[derive(Debug, Clone, Serialize)]
pub struct HealthResponse {
    /// Fixed service identifier.
    pub service: &'static str,
    /// Whether the server is ready to accept requests.
    pub ok: bool,
    /// Hosted workspace root served by this process.
    pub workspace_root: String,
    /// Governance scope enforced for the authenticated credential.
    pub access_scope: RemoteAccessScope,
    /// Actor identity bound to the authenticated credential.
    pub actor_id: String,
    /// Credential id used for this authenticated request.
    pub credential_id: String,
}

/// Serves the hosted WorkGraph HTTP adapter until the process is terminated.
///
/// # Errors
///
/// Returns an error when the listener cannot bind or the HTTP server crashes.
pub async fn serve(
    config: ApiServerConfig,
    executor: Arc<dyn RemoteCommandExecutor>,
) -> anyhow::Result<()> {
    let listen_addr = config.listen_addr;
    let state = ApiState { config, executor };
    let router = Router::new()
        .route("/v1/health", get(health))
        .route("/v1/execute", post(execute))
        .with_state(state);
    let listener = tokio::net::TcpListener::bind(listen_addr)
        .await
        .with_context(|| format!("failed to bind hosted API listener on {listen_addr}"))?;
    axum::serve(listener, router)
        .await
        .context("hosted API server terminated unexpectedly")
}

async fn health(
    State(state): State<ApiState>,
    headers: HeaderMap,
) -> Result<Json<HealthResponse>, ApiError> {
    let credential = authorize(&headers, &state.config.credentials)?;
    Ok(Json(HealthResponse {
        service: "workgraph-api",
        ok: true,
        workspace_root: state.config.workspace_root.display().to_string(),
        access_scope: credential.access_scope,
        actor_id: credential.actor_id,
        credential_id: credential.id,
    }))
}

async fn execute(
    State(state): State<ApiState>,
    headers: HeaderMap,
    Json(request): Json<RemoteCommandRequest>,
) -> Result<Json<RemoteCommandResponse>, ApiError> {
    let credential = authorize(&headers, &state.config.credentials)?;
    let response = state
        .executor
        .execute(state.config.workspace_root.clone(), credential, request)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(response))
}

fn authorize(
    headers: &HeaderMap,
    credentials: &[ApiCredential],
) -> Result<AuthenticatedCredential, ApiError> {
    let provided = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "));
    let Some(provided) = provided else {
        return Err(ApiError::unauthorized());
    };

    let provided_hash = token_hash(provided);
    for credential in credentials {
        if credential.token_hash == provided_hash {
            return Ok(AuthenticatedCredential {
                id: credential.id.clone(),
                actor_id: credential.actor_id.clone(),
                access_scope: credential.access_scope,
            });
        }
    }
    Err(ApiError::unauthorized())
}

fn token_hash(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    format!("{digest:x}")
}

struct ApiError {
    status: StatusCode,
    response: RemoteCommandResponse,
}

impl ApiError {
    fn unauthorized() -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            response: RemoteCommandResponse {
                success: false,
                rendered: "remote authentication failed".to_owned(),
            },
        }
    }

    fn internal(error: anyhow::Error) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            response: RemoteCommandResponse {
                success: false,
                rendered: format!("remote execution failed: {error}"),
            },
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.response)).into_response()
    }
}
