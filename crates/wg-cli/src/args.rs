//! CLI argument parsing and command definitions.

use clap::{Parser, Subcommand};
use wg_orientation::ContextLens;

use crate::util::fields::parse_key_value_input;

/// Top-level parsed CLI arguments.
#[derive(Debug, Parser)]
#[command(
    name = "workgraph",
    version,
    about = "WorkGraph v4 CLI",
    after_help = "Examples:\n  workgraph brief\n  workgraph --json brief\n  workgraph --format json status"
)]
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
    #[command(
        after_help = "Examples:\n  workgraph init\n  workgraph --json init\n  workgraph --format json init"
    )]
    Init,
    /// Produces an orientation summary for a human or agent entering the workspace.
    #[command(
        after_help = "Examples:\n  workgraph brief\n  workgraph brief --lens delivery\n  workgraph --json brief --lens workspace"
    )]
    Brief {
        /// Selects the orientation lens used to build the brief.
        #[arg(long, default_value_t = ContextLensArg(ContextLens::Workspace), value_parser = parse_context_lens)]
        lens: ContextLensArg,
    },
    /// Shows primitive counts and the latest recorded ledger entry.
    #[command(
        after_help = "Examples:\n  workgraph status\n  workgraph --json status\n  workgraph --format json status"
    )]
    Status,
    /// Claims an open thread for the configured actor.
    #[command(
        after_help = "Examples:\n  workgraph claim thread-1\n  workgraph --json claim launch-scoping"
    )]
    Claim {
        /// Stable thread identifier.
        thread_id: String,
    },
    /// Completes a thread after validating required evidence.
    #[command(
        after_help = "Examples:\n  workgraph complete thread-1\n  workgraph --json complete launch-verification"
    )]
    Complete {
        /// Stable thread identifier.
        thread_id: String,
    },
    /// Saves a durable working-context checkpoint.
    #[command(
        after_help = "Examples:\n  workgraph checkpoint --working-on \"Kernel hardening\" --focus \"Finish tests\"\n  workgraph --json checkpoint --working-on \"Phase 2\" --focus \"Evidence gaps\""
    )]
    Checkpoint {
        /// Current work item.
        #[arg(long)]
        working_on: String,
        /// Current focus.
        #[arg(long)]
        focus: String,
    },
    /// Views recent immutable ledger entries.
    #[command(
        after_help = "Examples:\n  workgraph ledger\n  workgraph ledger --last 20\n  workgraph --json ledger --last 5"
    )]
    Ledger {
        /// Number of most recent entries to include.
        #[arg(long)]
        last: Option<usize>,
    },
    /// Lists the structured capabilities and workflows exposed by this CLI.
    #[command(
        after_help = "Examples:\n  workgraph capabilities\n  workgraph --json capabilities\n  workgraph --format json capabilities"
    )]
    Capabilities,
    /// Describes primitive field definitions and output envelope metadata.
    #[command(
        after_help = "Examples:\n  workgraph schema\n  workgraph schema org\n  workgraph --json schema thread"
    )]
    Schema {
        /// Optionally narrows the schema view to a single primitive type.
        primitive_type: Option<String>,
    },
    /// Creates a new primitive in the markdown store.
    #[command(
        after_help = "Examples:\n  workgraph create org --title \"Versatly\"\n  workgraph create decision --title \"Use Rust\" --field status=decided\n  echo '{\"title\":\"Versatly\",\"fields\":{\"summary\":\"AI-native company\"}}' | workgraph create org --stdin"
    )]
    Create {
        /// The primitive type to create.
        primitive_type: String,
        /// The human-readable title of the new primitive.
        ///
        /// Optional when `--stdin` is provided and the stdin payload includes `title`.
        #[arg(long)]
        title: Option<String>,
        /// Additional frontmatter fields expressed as `key=value`.
        #[arg(long = "field", value_parser = parse_key_value_input)]
        fields: Vec<KeyValueInput>,
        /// Previews the created primitive and reference without writing anything.
        #[arg(long)]
        dry_run: bool,
        /// Reads a JSON payload from stdin (for example in pipelines).
        #[arg(long)]
        stdin: bool,
    },
    /// Queries primitives of a given type with optional exact-match filters.
    #[command(
        after_help = "Examples:\n  workgraph query org\n  workgraph query decision --filter status=decided\n  workgraph --json query thread --filter assigned_actor=cli"
    )]
    Query {
        /// The primitive type to query.
        primitive_type: String,
        /// Exact-match frontmatter filters expressed as `key=value`.
        #[arg(long = "filter", value_parser = parse_key_value_input)]
        filters: Vec<KeyValueInput>,
    },
    /// Displays a single primitive by `<type>/<id>`.
    #[command(
        after_help = "Examples:\n  workgraph show org/versatly\n  workgraph --json show decision/rust-for-workgraph-v4\n  workgraph show thread/kernel-thread-1"
    )]
    Show {
        /// The primitive reference in `<type>/<id>` form.
        reference: String,
    },
    /// Manages durable run receipts and run lifecycle transitions.
    #[command(
        after_help = "Examples:\n  workgraph run create --title \"Cursor pass\" --thread-id thread-1\n  workgraph run start cursor-pass\n  workgraph --json run complete cursor-pass --summary \"Finished implementation\""
    )]
    Run {
        /// Run-specific subcommand to execute.
        #[command(subcommand)]
        command: RunCommand,
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
            Self::Claim { .. } => "claim",
            Self::Complete { .. } => "complete",
            Self::Checkpoint { .. } => "checkpoint",
            Self::Ledger { .. } => "ledger",
            Self::Capabilities => "capabilities",
            Self::Schema { .. } => "schema",
            Self::Create { .. } => "create",
            Self::Query { .. } => "query",
            Self::Show { .. } => "show",
            Self::Run { command } => command.name(),
        }
    }
}

/// Supported `workgraph run` lifecycle subcommands.
#[derive(Debug, Subcommand)]
pub enum RunCommand {
    /// Creates a new queued run bound to a thread.
    #[command(
        after_help = "Examples:\n  workgraph run create --title \"Cursor pass\" --thread-id thread-1\n  workgraph run create --title \"Review pass\" --thread-id thread-1 --actor-id agent:reviewer --kind review --source cursor\n  workgraph --json run create --title \"Preview run\" --thread-id thread-1 --dry-run"
    )]
    Create {
        /// Human-readable title for the run.
        #[arg(long)]
        title: String,
        /// Owning thread identifier.
        #[arg(long = "thread-id")]
        thread_id: String,
        /// Tracked actor responsible for the run. Defaults to the configured actor.
        #[arg(long = "actor-id")]
        actor_id: Option<String>,
        /// Optional broad run classification such as `agent_pass` or `review`.
        #[arg(long)]
        kind: Option<String>,
        /// Optional source or surface that created or observed the run receipt.
        #[arg(long)]
        source: Option<String>,
        /// Tracked executor that performed the run when different from actor_id.
        #[arg(long = "executor-id")]
        executor_id: Option<String>,
        /// Related mission identifier, when known.
        #[arg(long = "mission-id")]
        mission_id: Option<String>,
        /// Parent run identifier for delegated execution, when any.
        #[arg(long = "parent-run-id")]
        parent_run_id: Option<String>,
        /// Optional initial summary stored alongside the run.
        #[arg(long)]
        summary: Option<String>,
        /// Previews the run without persisting it.
        #[arg(long)]
        dry_run: bool,
    },
    /// Marks a queued run as running.
    #[command(
        after_help = "Examples:\n  workgraph run start cursor-pass\n  workgraph --json run start review-pass"
    )]
    Start {
        /// Stable run identifier.
        run_id: String,
    },
    /// Marks a run as succeeded.
    #[command(
        after_help = "Examples:\n  workgraph run complete cursor-pass\n  workgraph --json run complete cursor-pass --summary \"Delivered final patch\""
    )]
    Complete {
        /// Stable run identifier.
        run_id: String,
        /// Optional summary to store with the terminal run state.
        #[arg(long)]
        summary: Option<String>,
    },
    /// Marks a run as failed.
    #[command(
        after_help = "Examples:\n  workgraph run fail cursor-pass\n  workgraph --json run fail cursor-pass --summary \"Blocked by missing dependency\""
    )]
    Fail {
        /// Stable run identifier.
        run_id: String,
        /// Optional summary to store with the terminal run state.
        #[arg(long)]
        summary: Option<String>,
    },
    /// Marks a run as cancelled.
    #[command(
        after_help = "Examples:\n  workgraph run cancel cursor-pass\n  workgraph --json run cancel cursor-pass --summary \"Superseded by newer run\""
    )]
    Cancel {
        /// Stable run identifier.
        run_id: String,
        /// Optional summary to store with the terminal run state.
        #[arg(long)]
        summary: Option<String>,
    },
}

impl RunCommand {
    /// Returns the stable command name associated with this parsed run subcommand.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Create { .. } => "run_create",
            Self::Start { .. } => "run_start",
            Self::Complete { .. } => "run_complete",
            Self::Fail { .. } => "run_fail",
            Self::Cancel { .. } => "run_cancel",
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
