#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Command-line entrypoints for initializing and interacting with WorkGraph workspaces.

mod app;
mod args;
mod commands;
mod output;
mod util;

use std::ffi::OsString;
use std::path::Path;

use anyhow::Context;
use app::AppContext;
use args::parse_cli;

/// Parses CLI arguments from the current process, executes the requested command, and prints the result.
///
/// # Errors
///
/// Returns an error when argument parsing fails or when the requested command cannot be completed.
pub async fn run_from_env() -> anyhow::Result<()> {
    let current_dir =
        std::env::current_dir().context("failed to determine the current directory")?;
    let output = execute(std::env::args_os(), current_dir).await?;
    println!("{output}");
    Ok(())
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
    let cli = parse_cli(args)?;
    let app = AppContext::new(workspace_root.as_ref().to_path_buf());
    let output = commands::execute(&app, cli.command).await?;
    output::render(&output, cli.json)
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
    async fn execute_supports_init_brief_create_status_query_show_and_json() {
        let temp_dir = tempdir().expect("temporary directory should be created");

        let init_output = execute(["workgraph", "init"], temp_dir.path())
            .await
            .expect("init should succeed");
        assert!(init_output.contains("Initialized WorkGraph workspace"));

        let brief_output = execute(["workgraph", "brief"], temp_dir.path())
            .await
            .expect("brief should succeed");
        assert!(brief_output.contains("Workspace brief"));

        let create_output = execute(
            [
                "workgraph",
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
        assert!(create_output.contains("Created org/versatly"));

        let status_output = execute(["workgraph", "status"], temp_dir.path())
            .await
            .expect("status should succeed");
        assert!(status_output.contains("org: 1"));
        assert!(status_output.contains("Last ledger entry:"));

        let query_output = execute(["workgraph", "query", "org"], temp_dir.path())
            .await
            .expect("query should succeed");
        assert!(query_output.contains("org/versatly"));

        let show_output = execute(["workgraph", "show", "org/versatly"], temp_dir.path())
            .await
            .expect("show should succeed");
        assert!(show_output.contains("summary: AI-native company"));

        let json_output = execute(["workgraph", "--json", "status"], temp_dir.path())
            .await
            .expect("json status should succeed");
        let parsed: JsonValue =
            serde_json::from_str(&json_output).expect("status output should be valid JSON");
        assert_eq!(parsed["command"], "status");
        assert_eq!(parsed["result"]["type_counts"]["org"], 1);

        let brief_json = execute(["workgraph", "--json", "brief"], temp_dir.path())
            .await
            .expect("json brief should succeed");
        let parsed_brief: JsonValue =
            serde_json::from_str(&brief_json).expect("brief output should be valid JSON");
        assert_eq!(parsed_brief["command"], "brief");
        assert!(
            parsed_brief["result"]["workspace_name"]
                .as_str()
                .expect("workspace name should be a string")
                .len()
                > 1
        );
    }
}
