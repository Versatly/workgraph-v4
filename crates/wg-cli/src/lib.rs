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
use wg_types::{ActorId, RemoteCommandRequest, RemoteCommandResponse};

/// Parses CLI arguments from the current process, executes the requested command, and prints the result.
///
/// # Errors
///
/// Returns an error when argument parsing fails or when the requested command cannot be completed.
pub async fn run_from_env() -> anyhow::Result<()> {
    let current_dir =
        std::env::current_dir().context("failed to determine the current directory")?;
    let args = std::env::args_os().collect::<Vec<_>>();
    if let Ok(cli) = parse_cli(args.clone()) {
        let json_output = cli.json || cli.format.is_json();
        let app = AppContext::new(current_dir.clone());
        match &cli.command {
            args::Command::Serve {
                listen,
                token,
                actor_id,
                access_scope,
            } => {
                let output = output::CommandOutput::Serve(commands::serve::describe_http(
                    &app,
                    listen,
                    Some(actor_id.as_str()),
                    access_scope.map(|scope| scope.0),
                )?);
                println!("{}", output::render_success(&output, json_output)?);
                return commands::serve::run_http(
                    &app,
                    listen,
                    token,
                    Some(actor_id.as_str()),
                    access_scope.map(|scope| scope.0),
                )
                .await;
            }
            args::Command::Mcp {
                command:
                    args::McpCommand::Serve {
                        actor_id,
                        access_scope,
                    },
            } => {
                return commands::serve::run_mcp(
                    &app,
                    Some(actor_id.as_str()),
                    access_scope.map(|scope| scope.0),
                )
                .await;
            }
            _ => {}
        }
    }
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
            if cli.command.can_execute_remotely() {
                if let Some(remote_response) =
                    try_execute_remote(&app, &args, cli.json || cli.format.is_json()).await?
                {
                    return Ok(ExecutionContract {
                        rendered: remote_response.rendered,
                        success: remote_response.success,
                    });
                }
            }
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
        Err(error) => {
            // Clap help/version requests are not failures — render as-is, exit success.
            if error.kind() == clap::error::ErrorKind::DisplayHelp
                || error.kind() == clap::error::ErrorKind::DisplayVersion
            {
                return Ok(ExecutionContract {
                    rendered: error.to_string().trim_end().to_owned(),
                    success: true,
                });
            }
            Ok(ExecutionContract {
                rendered: output::render_failure(None, &error.into(), output_format.is_json())?,
                success: false,
            })
        }
    }
}

async fn try_execute_remote(
    app: &AppContext,
    args: &[OsString],
    json_output: bool,
) -> anyhow::Result<Option<RemoteCommandResponse>> {
    let Some(config) = app.try_load_config().await? else {
        return Ok(None);
    };
    let Some(remote) = config.remote else {
        return Ok(None);
    };

    let request = RemoteCommandRequest {
        args: args
            .iter()
            .map(|value| value.to_string_lossy().into_owned())
            .collect(),
        actor_id: Some(remote.actor_id.to_string()),
    };

    let endpoint = format!("{}/v1/execute", remote.server_url.trim_end_matches('/'));
    let response = reqwest::Client::new()
        .post(endpoint)
        .bearer_auth(remote.auth_token)
        .json(&request)
        .send()
        .await
        .context("failed to reach hosted WorkGraph server")?;
    let status = response.status();
    let remote_response = response
        .json::<RemoteCommandResponse>()
        .await
        .context("failed to decode hosted WorkGraph response")?;

    if status.is_success() || json_output {
        Ok(Some(remote_response))
    } else {
        Err(anyhow!(remote_response.rendered))
    }
}

pub(crate) async fn execute_remote_contract(
    args: &[String],
    workspace_root: impl AsRef<Path>,
    actor_override: Option<ActorId>,
) -> anyhow::Result<RemoteCommandResponse> {
    let args = args
        .iter()
        .cloned()
        .map(OsString::from)
        .collect::<Vec<OsString>>();
    let json_output = parse_output_format(&args).is_json();

    match parse_cli(args.clone()) {
        Ok(cli) => {
            let app = AppContext::with_actor(workspace_root.as_ref().to_path_buf(), actor_override);
            let command_name = cli.command.name();
            match commands::execute(&app, cli.command).await {
                Ok(output) => Ok(RemoteCommandResponse {
                    success: true,
                    rendered: output::render_success(&output, cli.json || cli.format.is_json())?,
                }),
                Err(error) => render_failure_contract(
                    Some(command_name),
                    &error,
                    cli.json || cli.format.is_json(),
                ),
            }
        }
        Err(error) => {
            if error.kind() == clap::error::ErrorKind::DisplayHelp
                || error.kind() == clap::error::ErrorKind::DisplayVersion
            {
                return Ok(RemoteCommandResponse {
                    success: true,
                    rendered: error.to_string().trim_end().to_owned(),
                });
            }
            let error: anyhow::Error = error.into();
            render_failure_contract(None, &error, json_output)
        }
    }
}

pub(crate) fn render_failure_contract(
    command: Option<&str>,
    error: &anyhow::Error,
    json_output: bool,
) -> anyhow::Result<RemoteCommandResponse> {
    Ok(RemoteCommandResponse {
        success: false,
        rendered: output::render_failure(command, error, json_output)?,
    })
}

pub(crate) fn parse_output_format(args: &[OsString]) -> OutputFormat {
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
    use std::{net::TcpListener, time::Duration};

    use crate::app::AppContext;
    use crate::execute;
    use crate::util::fields::parse_key_value_input;
    use crate::util::slug::slugify;
    use reqwest::StatusCode;
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
        assert!(status_json["result"]["orphan_nodes"].is_array());

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

        execute(
            [
                "workgraph",
                "--json",
                "create",
                "thread",
                "--title",
                "Kernel Thread",
                "--field",
                "status=ready",
            ],
            temp_dir.path(),
        )
        .await
        .expect("thread create should succeed");
        let claim_output = execute(
            ["workgraph", "--json", "claim", "kernel-thread"],
            temp_dir.path(),
        )
        .await
        .expect("claim should succeed");
        let claim_json: JsonValue =
            serde_json::from_str(&claim_output).expect("claim output should be valid JSON");
        assert_eq!(claim_json["command"], "claim");
        assert_eq!(claim_json["result"]["thread"]["status"], "active");
        assert_eq!(claim_json["result"]["thread"]["assigned_actor"], "cli");

        let complete_output = execute(
            ["workgraph", "--json", "complete", "kernel-thread"],
            temp_dir.path(),
        )
        .await
        .expect("complete should succeed");
        let complete_json: JsonValue =
            serde_json::from_str(&complete_output).expect("complete output should be valid JSON");
        assert_eq!(complete_json["command"], "complete");
        assert_eq!(complete_json["result"]["thread"]["status"], "done");

        let checkpoint_output = execute(
            [
                "workgraph",
                "--json",
                "checkpoint",
                "--working-on",
                "Kernel hardening",
                "--focus",
                "Phase 2 delivery",
            ],
            temp_dir.path(),
        )
        .await
        .expect("checkpoint should succeed");
        let checkpoint_json: JsonValue = serde_json::from_str(&checkpoint_output)
            .expect("checkpoint output should be valid JSON");
        assert_eq!(checkpoint_json["command"], "checkpoint");
        assert_eq!(
            checkpoint_json["result"]["primitive"]["frontmatter"]["type"],
            "checkpoint"
        );

        let run_create_output = execute(
            [
                "workgraph",
                "--json",
                "run",
                "create",
                "--title",
                "Kernel Run",
                "--thread-id",
                "kernel-thread",
            ],
            temp_dir.path(),
        )
        .await
        .expect("run create should succeed");
        let run_create_json: JsonValue = serde_json::from_str(&run_create_output)
            .expect("run create output should be valid JSON");
        assert_eq!(run_create_json["command"], "run_create");
        assert_eq!(run_create_json["result"]["reference"], "run/kernel-run");
        assert_eq!(run_create_json["result"]["run"]["actor_id"], "cli");
        assert_eq!(run_create_json["result"]["run"]["status"], "queued");

        let run_start_output = execute(
            ["workgraph", "--json", "run", "start", "kernel-run"],
            temp_dir.path(),
        )
        .await
        .expect("run start should succeed");
        let run_start_json: JsonValue =
            serde_json::from_str(&run_start_output).expect("run start output should be valid JSON");
        assert_eq!(run_start_json["command"], "run_start");
        assert_eq!(run_start_json["result"]["run"]["status"], "running");

        let run_complete_output = execute(
            [
                "workgraph",
                "--json",
                "run",
                "complete",
                "kernel-run",
                "--summary",
                "Finished implementation",
            ],
            temp_dir.path(),
        )
        .await
        .expect("run complete should succeed");
        let run_complete_json: JsonValue = serde_json::from_str(&run_complete_output)
            .expect("run complete output should be valid JSON");
        assert_eq!(run_complete_json["command"], "run_complete");
        assert_eq!(run_complete_json["result"]["run"]["status"], "succeeded");
        assert_eq!(
            run_complete_json["result"]["run"]["summary"],
            "Finished implementation"
        );

        let show_run_output = execute(
            ["workgraph", "--json", "show", "run/kernel-run"],
            temp_dir.path(),
        )
        .await
        .expect("show run should succeed");
        let show_run_json: JsonValue =
            serde_json::from_str(&show_run_output).expect("show run output should be valid JSON");
        assert_eq!(show_run_json["command"], "show");
        assert_eq!(show_run_json["result"]["reference"], "run/kernel-run");

        let query_run_output = execute(["workgraph", "--json", "query", "run"], temp_dir.path())
            .await
            .expect("query run should succeed");
        let query_run_json: JsonValue =
            serde_json::from_str(&query_run_output).expect("query run output should be valid JSON");
        assert_eq!(query_run_json["result"]["count"], 1);

        let ledger_output = execute(
            ["workgraph", "--json", "ledger", "--last", "10"],
            temp_dir.path(),
        )
        .await
        .expect("ledger should succeed");
        let ledger_json: JsonValue =
            serde_json::from_str(&ledger_output).expect("ledger output should be valid JSON");
        assert_eq!(ledger_json["command"], "ledger");
        assert!(ledger_json["result"]["entries"].is_array());
        assert!(
            capabilities_json["result"]["commands"]
                .as_array()
                .expect("commands should be an array")
                .iter()
                .any(|command| command["name"] == "run create")
        );
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

    #[tokio::test]
    async fn run_create_supports_actor_override_and_dry_run() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        execute(["workgraph", "--json", "init"], temp_dir.path())
            .await
            .expect("init should succeed");

        execute(
            [
                "workgraph",
                "--json",
                "create",
                "thread",
                "--title",
                "Run Thread",
                "--field",
                "status=ready",
            ],
            temp_dir.path(),
        )
        .await
        .expect("thread create should succeed");

        let dry_run_output = execute(
            [
                "workgraph",
                "--json",
                "run",
                "create",
                "--title",
                "Preview Run",
                "--thread-id",
                "run-thread",
                "--dry-run",
            ],
            temp_dir.path(),
        )
        .await
        .expect("dry run create should succeed");
        let dry_run_json: JsonValue = serde_json::from_str(&dry_run_output)
            .expect("dry run create output should be valid JSON");
        assert_eq!(dry_run_json["command"], "run_create");
        assert_eq!(dry_run_json["result"]["outcome"], "dry_run");
        assert!(dry_run_json["result"]["ledger_entry"].is_null());

        let query_empty_output = execute(["workgraph", "--json", "query", "run"], temp_dir.path())
            .await
            .expect("query run should succeed");
        let query_empty_json: JsonValue = serde_json::from_str(&query_empty_output)
            .expect("query run output should be valid JSON");
        assert_eq!(query_empty_json["result"]["count"], 0);

        let override_output = execute(
            [
                "workgraph",
                "--json",
                "run",
                "create",
                "--title",
                "Reviewer Run",
                "--thread-id",
                "run-thread",
                "--actor-id",
                "agent:reviewer",
                "--kind",
                "review",
                "--source",
                "cursor",
            ],
            temp_dir.path(),
        )
        .await
        .expect("actor override create should succeed");
        let override_json: JsonValue = serde_json::from_str(&override_output)
            .expect("actor override output should be valid JSON");
        assert_eq!(override_json["result"]["run"]["actor_id"], "agent:reviewer");
        assert_eq!(override_json["result"]["run"]["kind"], "review");
        assert_eq!(override_json["result"]["run"]["source"], "cursor");
    }

    #[tokio::test]
    async fn run_fail_and_cancel_commands_transition_runs() {
        let temp_dir = tempdir().expect("temporary directory should be created");
        execute(["workgraph", "--json", "init"], temp_dir.path())
            .await
            .expect("init should succeed");

        execute(
            [
                "workgraph",
                "--json",
                "create",
                "thread",
                "--title",
                "Failure Thread",
                "--field",
                "status=ready",
            ],
            temp_dir.path(),
        )
        .await
        .expect("thread create should succeed");

        execute(
            [
                "workgraph",
                "--json",
                "run",
                "create",
                "--title",
                "Failure Run",
                "--thread-id",
                "failure-thread",
            ],
            temp_dir.path(),
        )
        .await
        .expect("failure run create should succeed");
        execute(
            ["workgraph", "--json", "run", "start", "failure-run"],
            temp_dir.path(),
        )
        .await
        .expect("failure run start should succeed");
        let fail_output = execute(
            [
                "workgraph",
                "--json",
                "run",
                "fail",
                "failure-run",
                "--summary",
                "Dependency missing",
            ],
            temp_dir.path(),
        )
        .await
        .expect("run fail should succeed");
        let fail_json: JsonValue =
            serde_json::from_str(&fail_output).expect("fail output should be valid JSON");
        assert_eq!(fail_json["command"], "run_fail");
        assert_eq!(fail_json["result"]["run"]["status"], "failed");

        execute(
            [
                "workgraph",
                "--json",
                "run",
                "create",
                "--title",
                "Cancel Run",
                "--thread-id",
                "failure-thread",
            ],
            temp_dir.path(),
        )
        .await
        .expect("cancel run create should succeed");
        execute(
            ["workgraph", "--json", "run", "start", "cancel-run"],
            temp_dir.path(),
        )
        .await
        .expect("cancel run start should succeed");
        let cancel_output = execute(
            [
                "workgraph",
                "--json",
                "run",
                "cancel",
                "cancel-run",
                "--summary",
                "Superseded",
            ],
            temp_dir.path(),
        )
        .await
        .expect("run cancel should succeed");
        let cancel_json: JsonValue =
            serde_json::from_str(&cancel_output).expect("cancel output should be valid JSON");
        assert_eq!(cancel_json["command"], "run_cancel");
        assert_eq!(cancel_json["result"]["run"]["status"], "cancelled");
    }

    #[tokio::test]
    async fn hosted_server_supports_actor_bound_remote_roundtrip() {
        let server_workspace = tempdir().expect("server workspace should exist");
        let client_workspace = tempdir().expect("client workspace should exist");
        let listen_addr = format!("127.0.0.1:{}", reserve_local_port());
        let server_url = format!("http://{listen_addr}");

        execute(["workgraph", "--json", "init"], server_workspace.path())
            .await
            .expect("server init should succeed");
        let server_app = AppContext::new(server_workspace.path().to_path_buf());
        let listen_for_task = listen_addr.clone();
        tokio::spawn(async move {
            let _ = crate::commands::serve::run_http(
                &server_app,
                &listen_for_task,
                "cursor-token",
                Some("agent:cursor"),
                Some(
                    crate::args::Command::Claim {
                        thread_id: "unused".to_owned(),
                    }
                    .required_remote_access_scope(),
                ),
            )
            .await;
        });
        wait_for_health(&server_url)
            .await
            .expect("server should become healthy");

        execute(["workgraph", "--json", "init"], client_workspace.path())
            .await
            .expect("client init should succeed");

        let connect_output = execute(
            [
                "workgraph",
                "--json",
                "connect",
                "--server",
                &server_url,
                "--token",
                "cursor-token",
                "--actor-id",
                "agent:cursor",
            ],
            client_workspace.path(),
        )
        .await
        .expect("connect should succeed");
        let connect_json: JsonValue =
            serde_json::from_str(&connect_output).expect("connect output should be valid JSON");
        assert_eq!(connect_json["result"]["actor_id"], "agent:cursor");
        assert_eq!(connect_json["result"]["access_scope"], "operate");

        let whoami_output = execute(["workgraph", "--json", "whoami"], client_workspace.path())
            .await
            .expect("whoami should succeed");
        let whoami_json: JsonValue =
            serde_json::from_str(&whoami_output).expect("whoami output should be valid JSON");
        assert_eq!(whoami_json["result"]["actor_id"], "agent:cursor");
        assert_eq!(whoami_json["result"]["access_scope"], "operate");

        execute(
            [
                "workgraph",
                "--json",
                "create",
                "thread",
                "--title",
                "Remote Thread",
                "--field",
                "status=ready",
            ],
            server_workspace.path(),
        )
        .await
        .expect("server-side thread creation should succeed");

        let claim_output = execute(
            ["workgraph", "--json", "claim", "remote-thread"],
            client_workspace.path(),
        )
        .await
        .expect("remote claim should succeed");
        let claim_json: JsonValue =
            serde_json::from_str(&claim_output).expect("claim output should be valid JSON");
        assert_eq!(
            claim_json["result"]["thread"]["assigned_actor"],
            "agent:cursor"
        );

        let run_output = execute(
            [
                "workgraph",
                "--json",
                "run",
                "create",
                "--title",
                "Remote Run",
                "--thread-id",
                "remote-thread",
            ],
            client_workspace.path(),
        )
        .await
        .expect("remote run create should succeed");
        let run_json: JsonValue =
            serde_json::from_str(&run_output).expect("run output should be valid JSON");
        assert_eq!(run_json["result"]["run"]["actor_id"], "agent:cursor");
    }

    #[tokio::test]
    async fn hosted_server_rejects_actor_mismatch_and_insufficient_scope() {
        let server_workspace = tempdir().expect("server workspace should exist");
        let mismatch_client = tempdir().expect("mismatch client workspace should exist");
        let read_only_client = tempdir().expect("read-only client workspace should exist");
        let operate_addr = format!("127.0.0.1:{}", reserve_local_port());
        let operate_url = format!("http://{operate_addr}");
        let read_only_addr = format!("127.0.0.1:{}", reserve_local_port());
        let read_only_url = format!("http://{read_only_addr}");

        execute(["workgraph", "--json", "init"], server_workspace.path())
            .await
            .expect("server init should succeed");
        let operate_app = AppContext::new(server_workspace.path().to_path_buf());
        let read_only_app = AppContext::new(server_workspace.path().to_path_buf());

        let operate_addr_for_task = operate_addr.clone();
        tokio::spawn(async move {
            let _ = crate::commands::serve::run_http(
                &operate_app,
                &operate_addr_for_task,
                "cursor-token",
                Some("agent:cursor"),
                Some(
                    crate::args::Command::Claim {
                        thread_id: "unused".to_owned(),
                    }
                    .required_remote_access_scope(),
                ),
            )
            .await;
        });

        let read_only_addr_for_task = read_only_addr.clone();
        tokio::spawn(async move {
            let _ = crate::commands::serve::run_http(
                &read_only_app,
                &read_only_addr_for_task,
                "reader-token",
                Some("person:reader"),
                Some(
                    crate::args::Command::Query {
                        primitive_type: "thread".to_owned(),
                        filters: Vec::new(),
                    }
                    .required_remote_access_scope(),
                ),
            )
            .await;
        });

        wait_for_health(&operate_url)
            .await
            .expect("operate server should become healthy");
        wait_for_health(&read_only_url)
            .await
            .expect("read-only server should become healthy");

        execute(["workgraph", "--json", "init"], mismatch_client.path())
            .await
            .expect("mismatch client init should succeed");
        execute(["workgraph", "--json", "init"], read_only_client.path())
            .await
            .expect("read-only client init should succeed");

        let mismatch = execute(
            [
                "workgraph",
                "--json",
                "connect",
                "--server",
                &operate_url,
                "--token",
                "cursor-token",
                "--actor-id",
                "person:pedro",
            ],
            mismatch_client.path(),
        )
        .await;
        assert!(mismatch.is_err(), "mismatched actor should be rejected");

        let connect_reader = execute(
            [
                "workgraph",
                "--json",
                "connect",
                "--server",
                &read_only_url,
                "--token",
                "reader-token",
                "--actor-id",
                "person:reader",
            ],
            read_only_client.path(),
        )
        .await
        .expect("read-only connect should succeed");
        let connect_reader_json: JsonValue = serde_json::from_str(&connect_reader)
            .expect("reader connect output should be valid JSON");
        assert_eq!(connect_reader_json["result"]["access_scope"], "read");

        let denied = execute(
            ["workgraph", "--json", "claim", "remote-thread"],
            read_only_client.path(),
        )
        .await;
        assert!(denied.is_err(), "read-only credential should reject writes");
    }

    fn reserve_local_port() -> u16 {
        TcpListener::bind("127.0.0.1:0")
            .expect("ephemeral listener should bind")
            .local_addr()
            .expect("listener should expose local addr")
            .port()
    }

    async fn wait_for_health(server_url: &str) -> anyhow::Result<()> {
        let client = reqwest::Client::new();
        let endpoint = format!("{server_url}/v1/health");
        for _ in 0..40 {
            if let Ok(response) = client.get(&endpoint).send().await {
                if response.status() == StatusCode::OK {
                    return Ok(());
                }
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }

        anyhow::bail!("timed out waiting for hosted server health endpoint");
    }
}
