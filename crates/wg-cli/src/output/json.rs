//! JSON output rendering for CLI command results and failures.

use anyhow::Context;

use super::envelope::{ErrorEnvelope, SuccessEnvelope};

/// Serializes a structured successful command output to pretty-printed JSON.
///
/// # Errors
///
/// Returns an error when the output cannot be serialized.
pub fn render_success(envelope: &SuccessEnvelope) -> anyhow::Result<String> {
    serde_json::to_string_pretty(&envelope).context("failed to serialize JSON output")
}

/// Serializes a structured failed command result to pretty-printed JSON.
///
/// # Errors
///
/// Returns an error when the envelope cannot be serialized.
pub fn render_failure(envelope: &ErrorEnvelope) -> anyhow::Result<String> {
    serde_json::to_string_pretty(&envelope).context("failed to serialize JSON error output")
}
