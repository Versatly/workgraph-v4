use anyhow::Result;
use clap::Parser;
use wg_cli::{Cli, run};

fn main() -> Result<()> {
    let cli = Cli::parse();
    run(cli)
}
