//! CLI argument parsing and command definitions.

use clap::{Parser, Subcommand};
use wg_orientation::ContextLens;

use crate::util::fields::parse_key_value_input;

/// Top-level parsed CLI arguments.
#[derive(Debug, Parser)]
#[command(name = "workgraph", version, about = "WorkGraph v4 CLI")]
pub struct Cli {
    /// Emits machine-readable JSON instead of human-oriented text output.
    #[arg(long, global = true)]
    pub json: bool,
    /// Selects the output format explicitly.
    #[arg(long, global = true, default_value_t = OutputFormat::Human)]
    pub format: OutputFormat,
    /// The subcommand to execute.
    #[command(subcommand)]
    pub command: Command,
}

/// Supported output formats for CLI command rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum OutputFormat {
    /// Human-readable terminal output.
    Human,
    /// Machine-readable JSON output.
    Json,
}

impl std::fmt::Display for OutputFormat {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Human => "human",
            Self::Json => "json",
        })
    }
}

impl OutputFormat {
    /// Returns true when the selected format is machine-readable JSON.
    #[must_use]
    pub const fn is_json(self) -> bool {
        matches!(self, Self::Json)
    }
}

/// Supported WorkGraph CLI commands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Initializes a new WorkGraph workspace in the current directory.
    Init,
    /// Produces an orientation summary for a human or agent entering the workspace.
    Brief {
        /// Selects the orientation lens used to build the brief.
        #[arg(long, default_value_t = ContextLensArg(ContextLens::Workspace), value_parser = parse_context_lens)]
        lens: ContextLensArg,
    },
    /// Shows primitive counts and the latest recorded ledger entry.
    Status,
    /// Lists the structured capabilities and workflows exposed by this CLI.
    Capabilities,
    /// Describes command arguments, outputs, and result envelope structure.
    Schema {
        /// Optionally narrows the schema view to a single command.
        command: Option<String>,
    },
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

impl Command {
    /// Returns the stable command name associated with this parsed command.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::Brief { .. } => "brief",
            Self::Status => "status",
            Self::Capabilities => "capabilities",
            Self::Schema { .. } => "schema",
            Self::Create { .. } => "create",
            Self::Query { .. } => "query",
            Self::Show { .. } => "show",
        }
    }
}

/// A parsed `key=value` argument pair used by create and query commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyValueInput {
    /// The parsed key portion.
    pub key: String,
    /// The parsed value portion.
    pub value: String,
}

/// A clap-friendly wrapper around [`wg_orientation::ContextLens`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextLensArg(pub ContextLens);

impl std::fmt::Display for ContextLensArg {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(self.0.as_str())
    }
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

fn parse_context_lens(input: &str) -> Result<ContextLensArg, String> {
    input.parse::<ContextLens>().map(ContextLensArg)
}
