#![forbid(unsafe_code)]

//! MCP surface placeholders for stdio and HTTP serving.

/// Transport mode supported by the placeholder MCP server.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum McpMode {
    /// Standard input/output transport.
    Stdio,
    /// HTTP transport.
    Http,
}

/// Placeholder MCP server configuration and lifecycle handle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct McpServer {
    mode: McpMode,
}

impl McpServer {
    /// Creates a new placeholder MCP server.
    #[must_use]
    pub const fn new(mode: McpMode) -> Self {
        Self { mode }
    }

    /// Returns the configured transport mode.
    #[must_use]
    pub const fn mode(&self) -> McpMode {
        self.mode
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new(McpMode::Stdio)
    }
}
