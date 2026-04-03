#![forbid(unsafe_code)]

//! Thin remote API adapter over the same workspace operations exposed by the CLI.

use std::path::{Path, PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// Transport exposed by the API surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ApiTransport {
    /// HTTP transport for REST endpoints.
    Http,
    /// gRPC transport.
    Grpc,
    /// Server-sent events transport.
    Sse,
    /// Webhook transport.
    Webhook,
}

/// Typed request envelope handled by the thin API adapter.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ApiRequest {
    /// Build a workspace brief.
    Brief {
        /// Optional orientation lens.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        lens: Option<String>,
    },
    /// Build a workspace status response.
    Status,
    /// Return CLI capabilities metadata.
    Capabilities,
    /// Return CLI schema metadata.
    Schema {
        /// Optional command scope.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        command: Option<String>,
    },
    /// Show a single primitive.
    Show {
        /// Primitive reference in `<type>/<id>` form.
        reference: String,
    },
    /// Query primitives of one type.
    Query {
        /// Primitive type to query.
        primitive_type: String,
        /// Exact-match filters in `key=value` form.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        filters: Vec<String>,
    },
    /// Execute any CLI-backed command through the API transport.
    Command {
        /// Arguments following the `workgraph` binary name.
        args: Vec<String>,
    },
}

/// Structured API response returned by the thin adapter.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ApiResponse {
    /// HTTP-like status code for the translated result.
    pub status: u16,
    /// Machine-readable JSON body aligned with CLI envelopes.
    pub body: JsonValue,
}

/// Thin API server configuration and execution handle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ApiServer {
    transport: ApiTransport,
    workspace_root: PathBuf,
}

impl ApiServer {
    /// Creates a new API server rooted at a workspace.
    #[must_use]
    pub fn new(transport: ApiTransport, workspace_root: impl Into<PathBuf>) -> Self {
        Self {
            transport,
            workspace_root: workspace_root.into(),
        }
    }

    /// Returns the configured transport.
    #[must_use]
    pub const fn transport(&self) -> ApiTransport {
        self.transport
    }

    /// Returns the configured workspace root.
    #[must_use]
    pub fn workspace_root(&self) -> &Path {
        &self.workspace_root
    }

    /// Handles a typed API request by delegating to the CLI-backed workspace operations.
    ///
    /// # Errors
    ///
    /// Returns an error when the request cannot be translated or the underlying command fails to render JSON.
    pub async fn handle(&self, request: ApiRequest) -> anyhow::Result<ApiResponse> {
        let args = request_to_cli_args(request);
        let envelope = wg_cli::execute_envelope(args, &self.workspace_root)
            .await
            .context("failed to execute API-backed workspace operation")?;
        let envelope: JsonValue =
            serde_json::from_str(&envelope).context("failed to parse API JSON envelope")?;
        let status = if envelope["success"] == JsonValue::Bool(true) {
            200
        } else if envelope["error"]["code"] == JsonValue::String("invalid_arguments".to_owned()) {
            400
        } else {
            422
        };
        Ok(ApiResponse {
            status,
            body: envelope,
        })
    }
}

impl Default for ApiServer {
    fn default() -> Self {
        Self::new(ApiTransport::Http, ".")
    }
}

fn request_to_cli_args(request: ApiRequest) -> Vec<String> {
    let mut args = vec!["workgraph".to_owned(), "--json".to_owned()];
    match request {
        ApiRequest::Brief { lens } => {
            args.push("brief".to_owned());
            if let Some(lens) = lens {
                args.push("--lens".to_owned());
                args.push(lens);
            }
        }
        ApiRequest::Status => args.push("status".to_owned()),
        ApiRequest::Capabilities => args.push("capabilities".to_owned()),
        ApiRequest::Schema { command } => {
            args.push("schema".to_owned());
            if let Some(command) = command {
                args.push(command);
            }
        }
        ApiRequest::Show { reference } => {
            args.push("show".to_owned());
            args.push(reference);
        }
        ApiRequest::Query {
            primitive_type,
            filters,
        } => {
            args.push("query".to_owned());
            args.push(primitive_type);
            for filter in filters {
                args.push("--filter".to_owned());
                args.push(filter);
            }
        }
        ApiRequest::Command { args: command_args } => args.extend(command_args),
    }
    args
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{ApiRequest, ApiServer, ApiTransport};

    #[tokio::test]
    async fn api_server_routes_workspace_requests() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let server = ApiServer::new(ApiTransport::Http, temp_dir.path());

        let init = server
            .handle(ApiRequest::Command {
                args: vec!["init".to_owned()],
            })
            .await
            .expect("init should succeed");
        assert_eq!(init.status, 200);

        let status = server
            .handle(ApiRequest::Status)
            .await
            .expect("status should succeed");
        assert_eq!(status.status, 200);
        assert_eq!(status.body["command"], "status");

        let brief = server
            .handle(ApiRequest::Brief {
                lens: Some("workspace".to_owned()),
            })
            .await
            .expect("brief should succeed");
        assert_eq!(brief.status, 200);
        assert_eq!(brief.body["command"], "brief");
    }
}
