//! JSON output rendering for CLI command results and failures.

use anyhow::Context;
use serde::Serialize;
use wg_error::WorkgraphError;

use super::{AGENT_SCHEMA_VERSION, AgentError, CommandOutput};

#[derive(Debug, Serialize)]
struct JsonEnvelope<'a, T>
where
    T: Serialize,
{
    schema_version: &'a str,
    success: bool,
    command: &'a str,
    result: Option<T>,
    error: Option<AgentError>,
    fix: Option<String>,
    next_actions: Vec<super::NextAction>,
}

/// Serializes a structured successful command output to pretty-printed JSON.
///
/// # Errors
///
/// Returns an error when the output cannot be serialized.
pub fn render_success(output: &CommandOutput) -> anyhow::Result<String> {
    let envelope = JsonEnvelope {
        schema_version: AGENT_SCHEMA_VERSION,
        success: true,
        command: output.command_name(),
        result: Some(output.result_value()?),
        error: None,
        fix: None,
        next_actions: output.next_actions(),
    };

    serde_json::to_string_pretty(&envelope).context("failed to serialize JSON output")
}

/// Serializes a structured failed command result to pretty-printed JSON.
///
/// # Errors
///
/// Returns an error when the envelope cannot be serialized.
pub fn render_failure(command: Option<&str>, error: &anyhow::Error) -> anyhow::Result<String> {
    let (code, fix, next_actions) = classify_error(command, error);
    let envelope = JsonEnvelope::<serde_json::Value> {
        schema_version: AGENT_SCHEMA_VERSION,
        success: false,
        command: command.unwrap_or("unknown"),
        result: None,
        error: Some(AgentError {
            code,
            message: error.to_string(),
        }),
        fix,
        next_actions,
    };

    serde_json::to_string_pretty(&envelope).context("failed to serialize JSON error output")
}

fn classify_error(
    command: Option<&str>,
    error: &anyhow::Error,
) -> (String, Option<String>, Vec<super::NextAction>) {
    if let Some(clap_error) = error.downcast_ref::<clap::Error>() {
        return (
            "invalid_arguments".to_owned(),
            Some(clap_error.to_string()),
            vec![
                next_action(
                    "schema",
                    "workgraph --json schema",
                    "Inspect structured command contracts and arguments.",
                ),
                next_action(
                    "capabilities",
                    "workgraph --json capabilities",
                    "Inspect recommended workflows and examples.",
                ),
            ],
        );
    }

    if let Some(workgraph_error) = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<WorkgraphError>())
    {
        return match workgraph_error {
            WorkgraphError::ValidationError(_) => (
                workgraph_error.code().to_owned(),
                Some(
                    "Inspect the command schema or provide required fields before retrying."
                        .to_owned(),
                ),
                vec![
                    next_action(
                        "schema",
                        "workgraph --json schema",
                        "Inspect structured command schemas.",
                    ),
                    next_action(
                        "capabilities",
                        "workgraph --json capabilities",
                        "Inspect valid CLI workflows and examples.",
                    ),
                ],
            ),
            WorkgraphError::RegistryError(_) => (
                workgraph_error.code().to_owned(),
                Some("Inspect known primitive types and retry with a supported type.".to_owned()),
                vec![next_action(
                    "schema-create",
                    "workgraph --json schema create",
                    "Inspect the create command contract and supported usage.",
                )],
            ),
            _ => (
                workgraph_error.code().to_owned(),
                guess_fix(command, error),
                generic_recovery_actions(),
            ),
        };
    }

    (
        "command_failed".to_owned(),
        guess_fix(command, error),
        generic_recovery_actions(),
    )
}

fn guess_fix(command: Option<&str>, error: &anyhow::Error) -> Option<String> {
    let message = error.to_string();
    if message.contains("config file") || message.contains("registry file") {
        return Some("Initialize the workspace first with `workgraph init`.".to_owned());
    }
    if message.contains("primitive reference") {
        return Some("Use the `<type>/<id>` form for primitive references.".to_owned());
    }
    if matches!(
        command,
        Some("brief" | "status" | "create" | "query" | "show")
    ) {
        return Some("Inspect `workgraph --json capabilities` or `workgraph --json schema` for a structured recovery path.".to_owned());
    }
    None
}

fn generic_recovery_actions() -> Vec<super::NextAction> {
    vec![
        next_action(
            "capabilities",
            "workgraph --json capabilities",
            "Inspect available workflows and examples for autonomous recovery.",
        ),
        next_action(
            "schema",
            "workgraph --json schema",
            "Inspect structured command and output contracts.",
        ),
    ]
}

fn next_action(title: &str, command: &str, description: &str) -> super::NextAction {
    super::NextAction {
        title: title.to_owned(),
        command: command.to_owned(),
        description: description.to_owned(),
    }
}
