#![forbid(unsafe_code)]

//! API surface placeholders for HTTP, gRPC, SSE, and webhook endpoints.
//!
//! Phase 3 keeps event-plane semantics in the kernel and CLI. This crate remains
//! a transport-thin placeholder until remote runtime surfaces are intentionally
//! implemented in a later phase.

/// Transport exposed by the placeholder API server.
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

/// Placeholder API server configuration and lifecycle handle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ApiServer {
    transport: ApiTransport,
}

impl ApiServer {
    /// Creates a new placeholder API server.
    #[must_use]
    pub const fn new(transport: ApiTransport) -> Self {
        Self { transport }
    }

    /// Returns the configured transport.
    #[must_use]
    pub const fn transport(&self) -> ApiTransport {
        self.transport
    }
}

impl Default for ApiServer {
    fn default() -> Self {
        Self::new(ApiTransport::Http)
    }
}
