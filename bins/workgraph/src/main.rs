#![forbid(unsafe_code)]

//! WorkGraph v4 executable entrypoint.

use std::process::ExitCode;

/// Runs the WorkGraph CLI.
#[tokio::main]
async fn main() -> ExitCode {
    let _ = tracing_subscriber::fmt::try_init();
    match wg_cli::run_from_env().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            if let Some(exit_code) = error
                .downcast_ref::<wg_cli::CliExitError>()
                .map(wg_cli::CliExitError::exit_code)
            {
                ExitCode::from(exit_code)
            } else {
                ExitCode::from(1)
            }
        }
    }
}
