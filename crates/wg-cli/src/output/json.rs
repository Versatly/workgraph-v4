//! JSON output rendering for CLI command results.

use anyhow::Context;

use super::CommandOutput;

/// Serializes a structured command output to pretty-printed JSON.
///
/// # Errors
///
/// Returns an error when the output cannot be serialized.
pub fn render(output: &CommandOutput) -> anyhow::Result<String> {
    serde_json::to_string_pretty(output).context("failed to serialize JSON output")
}
