#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Thin MCP adapter over the WorkGraph CLI contract.

use std::path::Path;

use anyhow::Context;
use serde::{Deserialize, Serialize};

/// Transport mode supported by the MCP server adapter.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum McpMode {
    /// Standard input/output transport.
    Stdio,
    /// HTTP transport.
    Http,
}

/// Machine-readable description of an exposed MCP tool.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct McpTool {
    /// Stable tool name.
    pub name: String,
    /// Human-readable purpose of the tool.
    pub description: String,
    /// Canonical CLI-shaped usage example.
    pub example: String,
}

/// Thin MCP server configuration and lifecycle handle.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct McpServer {
    mode: McpMode,
}

impl McpServer {
    /// Creates a new MCP server adapter.
    #[must_use]
    pub const fn new(mode: McpMode) -> Self {
        Self { mode }
    }

    /// Returns the configured transport mode.
    #[must_use]
    pub const fn mode(&self) -> McpMode {
        self.mode
    }

    /// Returns the MCP tool catalog exposed by this adapter.
    #[must_use]
    pub fn tools(&self) -> Vec<McpTool> {
        vec![
            tool(
                "brief",
                "Produce a structured workspace brief.",
                "brief --lens workspace",
            ),
            tool(
                "status",
                "Inspect counts, graph issues, and evidence gaps.",
                "status",
            ),
            tool(
                "capabilities",
                "Discover workflows and primitive contracts.",
                "capabilities",
            ),
            tool(
                "schema",
                "Inspect command and envelope contracts.",
                "schema create",
            ),
            tool(
                "show",
                "Load one primitive by reference.",
                "show org/versatly",
            ),
            tool(
                "query",
                "List primitives of one type.",
                "query decision --filter status=decided",
            ),
            tool(
                "create",
                "Create a primitive through the reference CLI surface.",
                "create org --title Versatly",
            ),
            tool(
                "thread",
                "Run thread lifecycle workflows.",
                "thread create --id launch-thread --title 'Launch readiness'",
            ),
            tool(
                "mission",
                "Run mission workflows.",
                "mission progress launch-mission",
            ),
            tool(
                "run",
                "Run execution lifecycle workflows.",
                "run complete run-1 --summary 'Completed successfully'",
            ),
            tool(
                "trigger",
                "Save or evaluate trigger definitions.",
                "trigger evaluate --entry-index 0",
            ),
            tool(
                "checkpoint",
                "Save a resumable checkpoint.",
                "checkpoint --working-on 'Kernel work' --focus 'Finalize tests'",
            ),
        ]
    }

    /// Invokes a WorkGraph command through the CLI JSON contract.
    ///
    /// The provided `args` should omit the binary name. For example:
    /// `["brief", "--lens", "workspace"]`.
    ///
    /// # Errors
    ///
    /// Returns an error when command execution fails or the JSON envelope cannot be parsed.
    pub async fn call<I, S>(
        &self,
        workspace_root: impl AsRef<Path>,
        args: I,
    ) -> anyhow::Result<serde_json::Value>
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        let mut command = vec!["workgraph".to_owned(), "--json".to_owned()];
        command.extend(args.into_iter().map(Into::into));
        let rendered = wg_cli::execute_envelope(command, workspace_root)
            .await
            .context("mcp adapter failed to execute workgraph command")?;
        serde_json::from_str(&rendered).context("mcp adapter failed to parse CLI JSON envelope")
    }
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new(McpMode::Stdio)
    }
}

fn tool(name: &str, description: &str, example: &str) -> McpTool {
    McpTool {
        name: name.to_owned(),
        description: description.to_owned(),
        example: format!("workgraph --json {example}"),
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{McpMode, McpServer};

    #[tokio::test]
    async fn mcp_server_invokes_cli_json_contract() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        let server = McpServer::new(McpMode::Stdio);

        let init = server
            .call(temp_dir.path(), ["init"])
            .await
            .expect("init should succeed");
        assert_eq!(init["success"], true);

        let status = server
            .call(temp_dir.path(), ["status"])
            .await
            .expect("status should succeed");
        assert_eq!(status["command"], "status");
    }

    #[test]
    fn mcp_server_exposes_core_tool_catalog() {
        let server = McpServer::default();
        let tools = server.tools();
        assert!(tools.iter().any(|tool| tool.name == "brief"));
        assert!(tools.iter().any(|tool| tool.name == "trigger"));
    }
}
