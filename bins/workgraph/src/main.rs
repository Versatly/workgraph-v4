#![forbid(unsafe_code)]

//! WorkGraph v4 executable entrypoint.

/// Runs the WorkGraph CLI.
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let _ = tracing_subscriber::fmt::try_init();
    wg_cli::run_from_env().await
}
