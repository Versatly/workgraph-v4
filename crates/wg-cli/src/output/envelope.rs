//! Shared result envelope builders for JSON and human output paths.

use serde::Serialize;
use serde_json::Value as JsonValue;
use wg_error::WorkgraphError;

use super::{AGENT_SCHEMA_VERSION, CommandOutput};

/// Canonical success envelope for machine-readable command responses.
#[derive(Debug, Clone, Serialize)]
pub struct SuccessEnvelope {
    /// Stable schema version for the envelope contract.
    pub schema_version: &'static str,
    /// Indicates whether command execution succeeded.
    pub success: bool,
    /// Stable command name that produced this response.
    pub command: String,
    /// Command-specific structured payload.
    pub result: JsonValue,
    /// Suggested follow-up command invocations.
    pub next_actions: Vec<String>,
}

/// Canonical error envelope for machine-readable command failures.
#[derive(Debug, Clone, Serialize)]
pub struct ErrorEnvelope {
    /// Stable schema version for the envelope contract.
    pub schema_version: &'static str,
    /// Indicates whether command execution succeeded.
    pub success: bool,
    /// Stable command name when known.
    pub command: String,
    /// Human-readable failure description.
    pub error: String,
    /// Actionable command invocation to recover.
    pub fix: String,
}

/// Builds a success envelope for a command output.
///
/// # Errors
///
/// Returns an error when serializing the command result payload fails.
pub fn success(output: &CommandOutput) -> anyhow::Result<SuccessEnvelope> {
    Ok(SuccessEnvelope {
        schema_version: AGENT_SCHEMA_VERSION,
        success: true,
        command: output.command_name().to_owned(),
        result: output.result_value()?,
        next_actions: output.next_actions(),
    })
}

/// Builds a failure envelope for a command error.
#[must_use]
pub fn failure(command: Option<&str>, error: &anyhow::Error) -> ErrorEnvelope {
    ErrorEnvelope {
        schema_version: AGENT_SCHEMA_VERSION,
        success: false,
        command: command.unwrap_or("unknown").to_owned(),
        error: error.to_string(),
        fix: classify_fix(command, error),
    }
}

fn classify_fix(command: Option<&str>, error: &anyhow::Error) -> String {
    if let Some(clap_error) = error.downcast_ref::<clap::Error>() {
        return fix_for_clap_error(command, clap_error);
    }

    if let Some(workgraph_error) = error
        .chain()
        .find_map(|cause| cause.downcast_ref::<WorkgraphError>())
    {
        return fix_for_workgraph_error(command, workgraph_error);
    }

    let message = error.to_string();
    if message.contains("primitive reference must be in the form <type>/<id>") {
        return "workgraph show <type>/<id>".to_owned();
    }
    if message.contains("unknown primitive type") {
        return match command {
            Some("create") => {
                "workgraph schema && workgraph create <type> --title \"<title>\"".to_owned()
            }
            Some("query") => "workgraph schema && workgraph query <type>".to_owned(),
            Some("schema") => "workgraph schema <type>".to_owned(),
            _ => "workgraph schema".to_owned(),
        };
    }
    if message.contains("missing title") {
        return "workgraph create <type> --title \"<title>\"".to_owned();
    }
    if message.contains("stdin payload") || message.contains("stdin JSON") {
        return "echo '{\"title\":\"<title>\",\"fields\":{\"key\":\"value\"}}' | workgraph create <type> --stdin".to_owned();
    }
    if message.contains("config file") || message.contains("registry file") {
        return "workgraph init".to_owned();
    }

    default_fix(command).to_owned()
}

fn fix_for_clap_error(command: Option<&str>, _error: &clap::Error) -> String {
    match command {
        Some("claim") => "workgraph claim <thread-id>",
        Some("complete") => "workgraph complete <thread-id>",
        Some("checkpoint") => {
            "workgraph checkpoint --working-on \"<work item>\" --focus \"<focus>\""
        }
        Some("ledger") => "workgraph ledger --last <n>",
        Some("create") => "workgraph create <type> --title \"<title>\" --field key=value",
        Some("query") => "workgraph query <type> --filter key=value",
        Some("show") => "workgraph show <type>/<id>",
        Some("brief") => "workgraph brief --lens workspace",
        Some("schema") => "workgraph schema <type>",
        Some("capabilities") => "workgraph capabilities",
        Some("status") => "workgraph status",
        Some("init") => "workgraph init",
        _ => "workgraph --help",
    }
    .to_owned()
}

fn fix_for_workgraph_error(command: Option<&str>, error: &WorkgraphError) -> String {
    match error {
        WorkgraphError::RegistryError(_) => "workgraph schema".to_owned(),
        WorkgraphError::ValidationError(_) => default_fix(command).to_owned(),
        WorkgraphError::StoreError(_) | WorkgraphError::IoError(_) => match command {
            Some("show") => "workgraph query <type> && workgraph show <type>/<id>".to_owned(),
            Some("create") => {
                "workgraph init && workgraph create <type> --title \"<title>\"".to_owned()
            }
            _ => "workgraph init".to_owned(),
        },
        WorkgraphError::LedgerError(_) => "workgraph status".to_owned(),
        WorkgraphError::EncodingError(_) => default_fix(command).to_owned(),
    }
}

fn default_fix(command: Option<&str>) -> &'static str {
    match command {
        Some("init") => "workgraph init",
        Some("brief") => "workgraph brief --lens workspace",
        Some("status") => "workgraph status",
        Some("claim") => "workgraph claim <thread-id>",
        Some("complete") => "workgraph complete <thread-id>",
        Some("checkpoint") => {
            "workgraph checkpoint --working-on \"<work item>\" --focus \"<focus>\""
        }
        Some("ledger") => "workgraph ledger --last <n>",
        Some("capabilities") => "workgraph capabilities",
        Some("schema") => "workgraph schema <type>",
        Some("create") => "workgraph create <type> --title \"<title>\"",
        Some("query") => "workgraph query <type>",
        Some("show") => "workgraph show <type>/<id>",
        _ => "workgraph --help",
    }
}

#[cfg(test)]
mod tests {
    use super::{failure, success};
    use crate::output::{CommandOutput, QueryOutput};

    #[test]
    fn success_envelope_serializes_required_contract() {
        let output = CommandOutput::Query(QueryOutput {
            primitive_type: "org".to_owned(),
            count: 0,
            items: Vec::new(),
        });
        let envelope = success(&output).expect("success envelope should build");
        let json = serde_json::to_value(&envelope).expect("success envelope should serialize");

        assert_eq!(json["schema_version"], "v1");
        assert_eq!(json["success"], true);
        assert_eq!(json["command"], "query");
        assert!(json["result"].is_object());
        assert!(json["next_actions"].is_array());
    }

    #[test]
    fn error_envelope_includes_actionable_fix() {
        let error = anyhow::anyhow!("primitive reference must be in the form <type>/<id>");
        let envelope = failure(Some("show"), &error);
        let json = serde_json::to_value(&envelope).expect("error envelope should serialize");

        assert_eq!(json["schema_version"], "v1");
        assert_eq!(json["success"], false);
        assert_eq!(json["command"], "show");
        assert!(json["error"].is_string());
        assert_eq!(json["fix"], "workgraph show <type>/<id>");
    }
}
