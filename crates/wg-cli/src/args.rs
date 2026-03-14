//! CLI argument parsing and command definitions.

use clap::{Parser, Subcommand};

use crate::util::fields::parse_key_value_input;

/// Top-level parsed CLI arguments.
#[derive(Debug, Parser)]
#[command(name = "workgraph", version, about = "WorkGraph v4 CLI")]
pub struct Cli {
    /// Emits machine-readable JSON instead of human-oriented text output.
    #[arg(long, global = true)]
    pub json: bool,
    /// The subcommand to execute.
    #[command(subcommand)]
    pub command: Command,
}

/// Supported WorkGraph CLI commands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Initializes a new WorkGraph workspace in the current directory.
    Init,
    /// Produces an orientation summary for a human or agent entering the workspace.
    Brief,
    /// Shows primitive counts and the latest recorded ledger entry.
    Status,
    /// Creates a new primitive in the markdown store.
    Create {
        /// The primitive type to create.
        primitive_type: String,
        /// The human-readable title of the new primitive.
        #[arg(long)]
        title: String,
        /// Additional frontmatter fields expressed as `key=value`.
        #[arg(long = "field", value_parser = parse_key_value_input)]
        fields: Vec<KeyValueInput>,
    },
    /// Queries primitives of a given type with optional exact-match filters.
    Query {
        /// The primitive type to query.
        primitive_type: String,
        /// Exact-match frontmatter filters expressed as `key=value`.
        #[arg(long = "filter", value_parser = parse_key_value_input)]
        filters: Vec<KeyValueInput>,
    },
    /// Displays a single primitive by `<type>/<id>`.
    Show {
        /// The primitive reference in `<type>/<id>` form.
        reference: String,
    },
}

/// A parsed `key=value` argument pair used by create and query commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyValueInput {
    /// The parsed key portion.
    pub key: String,
    /// The parsed value portion.
    pub value: String,
}

/// Parses CLI arguments into the typed [`Cli`] structure.
///
/// # Errors
///
/// Returns a clap parsing error when the provided argument sequence is invalid.
pub fn parse_cli<I, T>(args: I) -> Result<Cli, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    Cli::try_parse_from(args)
}
