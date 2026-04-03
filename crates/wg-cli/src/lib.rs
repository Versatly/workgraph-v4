#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Command-line entrypoints for initializing and interacting with WorkGraph workspaces.

mod app;
mod args;
mod commands;
mod output;
mod services;
mod util;

use std::ffi::OsString;
use std::path::Path;

use anyhow::{Context, anyhow};
use app::AppContext;
use args::{OutputFormat, parse_cli};

/// Structured CLI exit error used to preserve documented exit-code discipline.
#[derive(Debug)]
pub struct CliExitError {
    exit_code: u8,
}

impl CliExitError {
    /// Creates a new CLI exit error.
    #[must_use]
    pub const fn new(exit_code: u8) -> Self {
        Self { exit_code }
    }

    /// Returns the process exit code associated with this failure.
    #[must_use]
    pub const fn exit_code(&self) -> u8 {
        self.exit_code
    }
}

impl std::fmt::Display for CliExitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "workgraph exited with status {}", self.exit_code)
    }
}

impl std::error::Error for CliExitError {}

/// Parses CLI arguments from the current process, executes the requested command, and prints the result.
///
/// # Errors
///
/// Returns an error when argument parsing fails or when the requested command cannot be completed.
pub async fn run_from_env() -> anyhow::Result<()> {
    let current_dir =
        std::env::current_dir().context("failed to determine the current directory")?;
    let args = std::env::args_os().collect::<Vec<_>>();
    let execution = execute_contract(args, current_dir).await?;
    println!("{}", execution.rendered);

    if execution.success {
        Ok(())
    } else {
        let exit_code = if execution.usage_error { 2 } else { 1 };
        Err(CliExitError::new(exit_code).into())
    }
}

/// Executes the CLI using an arbitrary argument iterator and workspace root.
///
/// The iterator should include the binary name as its first argument, mirroring `std::env::args_os`.
///
/// # Errors
///
/// Returns an error when argument parsing fails or the selected subcommand encounters an operational error.
pub async fn execute<I, T>(args: I, workspace_root: impl AsRef<Path>) -> anyhow::Result<String>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let execution = execute_contract(args, workspace_root).await?;
    if execution.success {
        Ok(execution.rendered)
    } else {
        Err(anyhow!("command execution failed"))
    }
}

struct ExecutionContract {
    rendered: String,
    success: bool,
    usage_error: bool,
}

async fn execute_contract<I, T>(
    args: I,
    workspace_root: impl AsRef<Path>,
) -> anyhow::Result<ExecutionContract>
where
    I: IntoIterator<Item = T>,
    T: Into<OsString> + Clone,
{
    let args = args.into_iter().map(Into::into).collect::<Vec<OsString>>();
    let output_format = parse_output_format(&args);

    match parse_cli(args.clone()) {
        Ok(cli) => {
            let command_name = cli.command.name();
            let app = AppContext::new(workspace_root.as_ref().to_path_buf(), cli.dry_run);
            match commands::execute(&app, cli.command).await {
                Ok(output) => Ok(ExecutionContract {
                    rendered: output::render_success(&output, cli.json || cli.format.is_json())?,
                    success: true,
                    usage_error: false,
                }),
                Err(error) => Ok(ExecutionContract {
                    rendered: output::render_failure(
                        Some(command_name),
                        &error,
                        cli.json || cli.format.is_json(),
                    )?,
                    success: false,
                    usage_error: false,
                }),
            }
        }
        Err(error) => Ok(ExecutionContract {
            rendered: output::render_failure(None, &error.into(), output_format.is_json())?,
            success: false,
            usage_error: true,
        }),
    }
}

fn parse_output_format(args: &[OsString]) -> OutputFormat {
    let mut iter = args.iter();
    while let Some(argument) = iter.next() {
        if argument.to_str() == Some("--json") {
            return OutputFormat::Json;
        }

        if let Some(argument) = argument.to_str() {
            if let Some(value) = argument.strip_prefix("--format=") {
                return if value == "json" {
                    OutputFormat::Json
                } else {
                    OutputFormat::Human
                };
            }
        }

        if argument.to_str() == Some("--format") {
            if let Some(value) = iter.next().and_then(|value| value.to_str()) {
                return if value == "json" {
                    OutputFormat::Json
                } else {
                    OutputFormat::Human
                };
            }
        }
    }

    OutputFormat::Human
}

#[cfg(test)]
mod tests {
    use std::process::Command;

    use crate::execute;
    use crate::util::fields::parse_key_value_input;
    use crate::util::slug::slugify;
    use serde_json::Value as JsonValue;
    use tempfile::tempdir;

    #[test]
    fn slugify_normalizes_titles() {
        assert_eq!(slugify("Rust for WorkGraph v4"), "rust-for-workgraph-v4");
        assert_eq!(slugify("  Multiple   Spaces  "), "multiple-spaces");
        assert_eq!(slugify("!!!"), "untitled");
    }

    #[test]
    fn parse_key_value_requires_equals() {
        assert!(parse_key_value_input("status=decided").is_ok());
        assert!(parse_key_value_input("missing").is_err());
        assert!(parse_key_value_input("=bad").is_err());
    }

    #[tokio::test]
    async fn execute_supports_agent_native_json_contract_and_discovery_commands() {
        let temp_dir = tempdir().expect("temporary directory should be created");

        let init_output = execute(["workgraph", "--json", "init"], temp_dir.path())
            .await
            .expect("init should succeed");
        let init_json: JsonValue =
            serde_json::from_str(&init_output).expect("init output should be valid JSON");
        assert_eq!(init_json["schema_version"], "workgraph.cli.v1alpha2");
        assert_eq!(init_json["success"], true);
        assert_eq!(init_json["command"], "init");
        assert!(init_json["next_actions"].is_array());

        let brief_output = execute(
            ["workgraph", "--json", "brief", "--lens", "workspace"],
            temp_dir.path(),
        )
        .await
        .expect("brief should succeed");
        let brief_json: JsonValue =
            serde_json::from_str(&brief_output).expect("brief output should be valid JSON");
        assert_eq!(brief_json["command"], "brief");
        assert_eq!(brief_json["result"]["lens"], "workspace");

        let create_output = execute(
            [
                "workgraph",
                "--json",
                "create",
                "org",
                "--title",
                "Versatly",
                "--field",
                "summary=AI-native company",
            ],
            temp_dir.path(),
        )
        .await
        .expect("create should succeed");
        let create_json: JsonValue =
            serde_json::from_str(&create_output).expect("create output should be valid JSON");
        assert_eq!(create_json["command"], "create");
        assert_eq!(create_json["result"]["reference"], "org/versatly");
        assert!(create_json["next_actions"].is_array());

        let status_output = execute(["workgraph", "--json", "status"], temp_dir.path())
            .await
            .expect("status should succeed");
        let status_json: JsonValue =
            serde_json::from_str(&status_output).expect("status output should be valid JSON");
        assert_eq!(status_json["command"], "status");
        assert_eq!(status_json["result"]["type_counts"]["org"], 1);

        let query_output = execute(["workgraph", "--json", "query", "org"], temp_dir.path())
            .await
            .expect("query should succeed");
        let query_json: JsonValue =
            serde_json::from_str(&query_output).expect("query output should be valid JSON");
        assert_eq!(query_json["command"], "query");
        assert_eq!(query_json["result"]["count"], 1);

        let show_output = execute(
            ["workgraph", "--json", "show", "org/versatly"],
            temp_dir.path(),
        )
        .await
        .expect("show should succeed");
        let show_json: JsonValue =
            serde_json::from_str(&show_output).expect("show output should be valid JSON");
        assert_eq!(show_json["command"], "show");
        assert_eq!(show_json["result"]["reference"], "org/versatly");

        let capabilities_output = execute(["workgraph", "--json", "capabilities"], temp_dir.path())
            .await
            .expect("capabilities should succeed");
        let capabilities_json: JsonValue = serde_json::from_str(&capabilities_output)
            .expect("capabilities output should be valid JSON");
        assert_eq!(capabilities_json["command"], "capabilities");
        assert!(capabilities_json["result"]["workflows"].is_array());
        assert!(capabilities_json["result"]["primitive_contracts"].is_array());

        let schema_output = execute(["workgraph", "--json", "schema", "create"], temp_dir.path())
            .await
            .expect("schema should succeed");
        let schema_json: JsonValue =
            serde_json::from_str(&schema_output).expect("schema output should be valid JSON");
        assert_eq!(schema_json["command"], "schema");
        assert_eq!(schema_json["result"]["commands"][0]["name"], "create");
        assert!(schema_json["result"]["primitive_contracts"].is_array());
    }

    #[test]
    fn clap_usage_errors_exit_with_code_two() {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        let workspace_root = std::path::Path::new(manifest_dir)
            .parent()
            .and_then(std::path::Path::parent)
            .expect("workspace root should resolve");
        let output = Command::new("cargo")
            .arg("run")
            .arg("--quiet")
            .arg("-p")
            .arg("workgraph")
            .arg("--bin")
            .arg("workgraph")
            .arg("--")
            .arg("create")
            .current_dir(workspace_root)
            .output()
            .expect("binary should execute");

        assert_eq!(output.status.code(), Some(2));
    }

}
