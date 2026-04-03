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
        Err(anyhow!("command execution failed"))
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
            let app = AppContext::new(workspace_root.as_ref().to_path_buf());
            match commands::execute(&app, cli.command).await {
                Ok(output) => Ok(ExecutionContract {
                    rendered: output::render_success(&output, cli.json || cli.format.is_json())?,
                    success: true,
                }),
                Err(error) => Ok(ExecutionContract {
                    rendered: output::render_failure(
                        Some(command_name),
                        &error,
                        cli.json || cli.format.is_json(),
                    )?,
                    success: false,
                }),
            }
        }
        Err(error) => Ok(ExecutionContract {
            rendered: output::render_failure(None, &error.into(), output_format.is_json())?,
            success: false,
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
        assert_eq!(init_json["schema_version"], "v1");
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
        assert_eq!(brief_json["result"]["orientation"]["lens"], "workspace");
        assert!(brief_json["result"]["workspace"]["id"].is_string());

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
        assert_eq!(create_json["result"]["outcome"], "created");
        assert!(create_json["next_actions"].is_array());

        let create_noop_output = execute(
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
        .expect("idempotent create should succeed");
        let create_noop_json: JsonValue = serde_json::from_str(&create_noop_output)
            .expect("idempotent create output should be valid JSON");
        assert_eq!(create_noop_json["result"]["outcome"], "noop");
        assert_eq!(create_noop_json["result"]["reference"], "org/versatly");
        assert_eq!(create_noop_json["result"]["ledger_entry"], JsonValue::Null);

        let create_dry_run_output = execute(
            [
                "workgraph",
                "--json",
                "create",
                "org",
                "--title",
                "Versatly Dry Run",
                "--dry-run",
            ],
            temp_dir.path(),
        )
        .await
        .expect("dry-run create should succeed");
        let create_dry_run_json: JsonValue = serde_json::from_str(&create_dry_run_output)
            .expect("dry-run create output should be valid JSON");
        assert_eq!(create_dry_run_json["result"]["outcome"], "dry_run");
        assert_eq!(
            create_dry_run_json["result"]["reference"],
            "org/versatly-dry-run"
        );
        assert_eq!(
            create_dry_run_json["result"]["path"],
            temp_dir
                .path()
                .join("orgs")
                .join("versatly-dry-run.md")
                .display()
                .to_string()
        );
        assert_eq!(
            create_dry_run_json["result"]["ledger_entry"],
            JsonValue::Null
        );

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
        assert_eq!(
            capabilities_json["result"]["first_command"],
            "workgraph brief --json"
        );
        assert!(capabilities_json["result"]["commands"].is_array());
        assert!(capabilities_json["result"]["commands"][0]["flags"].is_array());
        assert!(capabilities_json["result"]["commands"][0]["examples"].is_array());

        let schema_output = execute(["workgraph", "--json", "schema", "org"], temp_dir.path())
            .await
            .expect("schema should succeed");
        let schema_json: JsonValue =
            serde_json::from_str(&schema_output).expect("schema output should be valid JSON");
        assert_eq!(schema_json["command"], "schema");
        assert!(schema_json["result"]["envelope_fields"].is_array());
        assert!(schema_json["result"]["primitive_types"].is_array());
        assert_eq!(schema_json["result"]["primitive_types"][0]["name"], "org");
    }

    #[tokio::test]
    async fn create_is_idempotent_for_identical_payload() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        execute(["workgraph", "--json", "init"], temp_dir.path())
            .await
            .expect("init should succeed");

        let first = execute(
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
        .expect("first create should succeed");
        let first_json: JsonValue =
            serde_json::from_str(&first).expect("first create output should be valid JSON");
        assert_eq!(first_json["result"]["outcome"], "created");

        let second = execute(
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
        .expect("second create should return idempotent success");
        let second_json: JsonValue =
            serde_json::from_str(&second).expect("second create output should be valid JSON");
        assert_eq!(second_json["success"], true);
        assert_eq!(second_json["result"]["outcome"], "noop");
        assert!(second_json["result"]["ledger_entry"].is_null());
    }

    #[tokio::test]
    async fn create_dry_run_previews_without_writing() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        execute(["workgraph", "--json", "init"], temp_dir.path())
            .await
            .expect("init should succeed");

        let dry_run_output = execute(
            [
                "workgraph",
                "--json",
                "create",
                "org",
                "--title",
                "Preview Org",
                "--dry-run",
            ],
            temp_dir.path(),
        )
        .await
        .expect("dry run create should succeed");
        let dry_run_json: JsonValue =
            serde_json::from_str(&dry_run_output).expect("dry run output should be valid JSON");
        assert_eq!(dry_run_json["result"]["outcome"], "dry_run");
        assert!(dry_run_json["result"]["ledger_entry"].is_null());

        let query_output = execute(["workgraph", "--json", "query", "org"], temp_dir.path())
            .await
            .expect("query should succeed");
        let query_json: JsonValue =
            serde_json::from_str(&query_output).expect("query output should be valid JSON");
        assert_eq!(query_json["result"]["count"], 0);
    }
}
