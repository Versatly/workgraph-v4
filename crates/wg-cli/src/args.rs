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
    after_help = "Examples:\n  workgraph --json init\n  workgraph --json brief --lens workspace\n  workgraph --json create org --title Versatly --field summary='AI-native company'\n  printf 'Mission objective' | workgraph --json create mission --title 'Launch mission' --stdin-body\n  workgraph --json create decision --title 'Rust for WorkGraph' --dry-run"
)]
pub struct Cli {
    /// Emits machine-readable JSON instead of human-oriented text output.
    #[arg(long, global = true)]
    pub json: bool,
    /// Selects the output format explicitly.
    #[arg(long, global = true, default_value_t = OutputFormat::Human)]
    pub format: OutputFormat,
    /// Validates a write command and renders the intended result without mutating storage or ledger state.
    #[arg(long, global = true)]
    pub dry_run: bool,
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
    #[command(
        after_help = "Examples:\n  workgraph --json brief --lens workspace\n  workgraph --json brief --lens delivery"
    )]
    Brief {
        /// Selects the orientation lens used to build the brief.
        #[arg(long, default_value_t = ContextLensArg(ContextLens::Workspace), value_parser = parse_context_lens)]
        lens: ContextLensArg,
    },
    /// Shows primitive counts and the latest recorded ledger entry.
    #[command(after_help = "Examples:\n  workgraph --json status")]
    Status,
    /// Lists the structured capabilities and workflows exposed by this CLI.
    #[command(after_help = "Examples:\n  workgraph --json capabilities")]
    Capabilities,
    /// Describes command arguments, outputs, and result envelope structure.
    #[command(after_help = "Examples:\n  workgraph --json schema\n  workgraph --json schema create")]
    Schema {
        /// Optionally narrows the schema view to a single command.
        command: Option<String>,
    },
    /// Creates a new primitive in the markdown store.
    #[command(
        after_help = "Examples:\n  workgraph --json create org --title Versatly --field summary='AI-native company'\n  workgraph --json create decision --title 'Rust for WorkGraph' --id rust-for-workgraph --field status=decided\n  printf 'Long-form mission objective' | workgraph --json create mission --title 'Launch mission' --stdin-body\n  workgraph --json create strategic_note --title 'North star' --body 'Long-form context' --dry-run"
    )]
    Create {
        /// The primitive type to create.
        primitive_type: String,
        /// The human-readable title of the new primitive.
        #[arg(long)]
        title: String,
        /// Optional explicit primitive identifier. When omitted, WorkGraph derives a stable id from the title.
        #[arg(long)]
        id: Option<String>,
        /// Optional markdown body content supplied directly on the command line.
        #[arg(long)]
        body: Option<String>,
        /// Reads the markdown body from standard input. Useful for shell pipelines.
        #[arg(long)]
        stdin_body: bool,
        /// Additional frontmatter fields expressed as `key=value`.
        #[arg(long = "field", value_parser = parse_key_value_input)]
        fields: Vec<KeyValueInput>,
    },
    /// Mutates or inspects evidence-bearing coordination threads.
    #[command(
        after_help = "Examples:\n  workgraph --json thread create --id launch-thread --title 'Launch readiness'\n  workgraph --json thread claim launch-thread --actor pedro\n  workgraph --json thread add-message launch-thread --actor agent:cursor --text 'Investigating now.'\n  workgraph --json thread complete launch-thread"
    )]
    Thread {
        /// The thread workflow to execute.
        #[command(subcommand)]
        command: ThreadCommand,
    },
    /// Mutates or inspects missions that coordinate related threads and runs.
    #[command(
        after_help = "Examples:\n  workgraph --json mission create --id launch --title 'Launch mission' --objective 'Ship safely'\n  workgraph --json mission add-thread launch launch-thread\n  workgraph --json mission progress launch"
    )]
    Mission {
        /// The mission workflow to execute.
        #[command(subcommand)]
        command: MissionCommand,
    },
    /// Mutates or inspects run execution state.
    #[command(
        after_help = "Examples:\n  workgraph --json run create --id run-1 --title 'Cursor analysis' --actor agent:cursor --thread launch-thread\n  workgraph --json run start run-1\n  workgraph --json run complete run-1 --summary 'Completed successfully'"
    )]
    Run {
        /// The run workflow to execute.
        #[command(subcommand)]
        command: RunCommand,
    },
    /// Saves or evaluates trigger definitions.
    #[command(
        after_help = "Examples:\n  workgraph --json trigger save --id trigger-1 --title 'React to completed threads' --status active --event-source ledger --op done --primitive-type thread --field-name evidence --action-kind rebrief_actor --action-target agent/cursor --action-instruction 'Refresh the brief'\n  workgraph --json trigger evaluate --entry-index 3"
    )]
    Trigger {
        /// The trigger workflow to execute.
        #[command(subcommand)]
        command: TriggerCommand,
    },
    /// Saves a resumable checkpoint for the current work focus.
    #[command(
        after_help = "Examples:\n  workgraph --json checkpoint --working-on 'Kernel implementation' --focus 'Finish trigger CLI'"
    )]
    Checkpoint {
        /// Current work item.
        #[arg(long)]
        working_on: String,
        /// Current focus for the next agent or human.
        #[arg(long)]
        focus: String,
    },
    /// Queries primitives of a given type with optional exact-match filters.
    #[command(
        after_help = "Examples:\n  workgraph --json query decision\n  workgraph --json query decision --filter status=decided"
    )]
    Query {
        /// The primitive type to query.
        primitive_type: String,
        /// Exact-match frontmatter filters expressed as `key=value`.
        #[arg(long = "filter", value_parser = parse_key_value_input)]
        filters: Vec<KeyValueInput>,
    },
    /// Displays a single primitive by `<type>/<id>`.
    #[command(after_help = "Examples:\n  workgraph --json show org/versatly")]
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
            Self::Thread { .. } => "thread",
            Self::Mission { .. } => "mission",
            Self::Run { .. } => "run",
            Self::Trigger { .. } => "trigger",
            Self::Checkpoint { .. } => "checkpoint",
            Self::Query { .. } => "query",
            Self::Show { .. } => "show",
        }
    }
}

/// Supported thread-specific workflows exposed by the CLI.
#[derive(Debug, Subcommand)]
pub enum ThreadCommand {
    /// Creates a new coordination thread.
    Create {
        /// Stable thread identifier.
        #[arg(long)]
        id: String,
        /// Human-readable thread title.
        #[arg(long)]
        title: String,
        /// Optional parent mission identifier.
        #[arg(long)]
        parent_mission_id: Option<String>,
    },
    /// Opens a thread for active work.
    Open {
        /// Stable thread identifier.
        thread_id: String,
    },
    /// Claims a thread for a human or agent actor.
    Claim {
        /// Stable thread identifier.
        thread_id: String,
        /// Actor claiming the thread.
        #[arg(long)]
        actor: String,
    },
    /// Adds an exit criterion that must be satisfied before completion.
    AddExitCriterion {
        /// Stable thread identifier.
        thread_id: String,
        /// Stable criterion identifier.
        #[arg(long)]
        id: String,
        /// Human-readable criterion title.
        #[arg(long)]
        title: String,
        /// Optional longer description.
        #[arg(long)]
        description: Option<String>,
        /// Optional supporting reference.
        #[arg(long)]
        reference: Option<String>,
        /// Marks the criterion optional instead of required.
        #[arg(long, default_value_t = false)]
        optional: bool,
    },
    /// Records evidence against a thread.
    AddEvidence {
        /// Stable thread identifier.
        thread_id: String,
        /// Stable evidence identifier.
        #[arg(long)]
        id: String,
        /// Human-readable evidence title.
        #[arg(long)]
        title: String,
        /// Optional description.
        #[arg(long)]
        description: Option<String>,
        /// Optional reference to a source record or primitive.
        #[arg(long)]
        reference: Option<String>,
        /// Criterion identifiers this evidence satisfies.
        #[arg(long = "satisfies")]
        satisfies: Vec<String>,
        /// Optional source label, such as manual or run.
        #[arg(long)]
        source: Option<String>,
    },
    /// Adds a planned update action to a thread.
    AddUpdateAction {
        /// Stable thread identifier.
        thread_id: String,
        /// Stable action identifier.
        #[arg(long)]
        id: String,
        /// Human-readable action title.
        #[arg(long)]
        title: String,
        /// Action kind.
        #[arg(long)]
        kind: String,
        /// Optional target reference.
        #[arg(long)]
        target_reference: Option<String>,
        /// Optional description.
        #[arg(long)]
        description: Option<String>,
    },
    /// Adds a planned completion action to a thread.
    AddCompletionAction {
        /// Stable thread identifier.
        thread_id: String,
        /// Stable action identifier.
        #[arg(long)]
        id: String,
        /// Human-readable action title.
        #[arg(long)]
        title: String,
        /// Action kind.
        #[arg(long)]
        kind: String,
        /// Optional target reference.
        #[arg(long)]
        target_reference: Option<String>,
        /// Optional description.
        #[arg(long)]
        description: Option<String>,
    },
    /// Appends a message to a thread conversation.
    AddMessage {
        /// Stable thread identifier.
        thread_id: String,
        /// Actor authoring the message.
        #[arg(long)]
        actor: String,
        /// Message text.
        #[arg(long)]
        text: String,
    },
    /// Completes a thread once required evidence is present.
    Complete {
        /// Stable thread identifier.
        thread_id: String,
    },
}

/// Supported mission-specific workflows exposed by the CLI.
#[derive(Debug, Subcommand)]
pub enum MissionCommand {
    /// Creates a new mission.
    Create {
        /// Stable mission identifier.
        #[arg(long)]
        id: String,
        /// Human-readable mission title.
        #[arg(long)]
        title: String,
        /// Mission objective markdown.
        #[arg(long)]
        objective: String,
    },
    /// Marks a mission active.
    Activate { mission_id: String },
    /// Marks a mission blocked.
    Block { mission_id: String },
    /// Marks a mission completed.
    Complete { mission_id: String },
    /// Attaches an existing thread to a mission.
    AddThread { mission_id: String, thread_id: String },
    /// Attaches an existing run to a mission.
    AddRun { mission_id: String, run_id: String },
    /// Computes mission progress from stored threads.
    Progress { mission_id: String },
}

/// Supported run-specific workflows exposed by the CLI.
#[derive(Debug, Subcommand)]
pub enum RunCommand {
    /// Creates a queued run.
    Create {
        /// Stable run identifier.
        #[arg(long)]
        id: String,
        /// Human-readable run title.
        #[arg(long)]
        title: String,
        /// Logical actor responsible for the run.
        #[arg(long)]
        actor: String,
        /// Owning thread identifier.
        #[arg(long)]
        thread: String,
        /// Optional concrete executor.
        #[arg(long)]
        executor: Option<String>,
        /// Optional related mission identifier.
        #[arg(long)]
        mission: Option<String>,
        /// Optional parent run identifier.
        #[arg(long)]
        parent_run: Option<String>,
        /// Optional run summary.
        #[arg(long)]
        summary: Option<String>,
    },
    /// Starts a queued or retryable run.
    Start { run_id: String },
    /// Marks a run complete.
    Complete {
        /// Stable run identifier.
        run_id: String,
        /// Optional completion summary.
        #[arg(long)]
        summary: Option<String>,
    },
    /// Marks a run failed.
    Fail {
        /// Stable run identifier.
        run_id: String,
        /// Optional failure summary.
        #[arg(long)]
        summary: Option<String>,
    },
    /// Cancels a run.
    Cancel {
        /// Stable run identifier.
        run_id: String,
        /// Optional cancellation summary.
        #[arg(long)]
        summary: Option<String>,
    },
}

/// Supported trigger-specific workflows exposed by the CLI.
#[derive(Debug, Subcommand)]
pub enum TriggerCommand {
    /// Saves a trigger definition.
    Save {
        /// Stable trigger identifier.
        #[arg(long)]
        id: String,
        /// Human-readable trigger title.
        #[arg(long)]
        title: String,
        /// Trigger status: draft, active, paused, or disabled.
        #[arg(long, default_value = "draft")]
        status: String,
        /// Event source: ledger, webhook, or internal.
        #[arg(long = "event-source")]
        event_source: String,
        /// Optional event name for webhook/internal triggers.
        #[arg(long)]
        event_name: Option<String>,
        /// Optional ledger operation filter.
        #[arg(long = "op")]
        ops: Vec<String>,
        /// Optional primitive type filters.
        #[arg(long = "primitive-type")]
        primitive_types: Vec<String>,
        /// Optional primitive id filter.
        #[arg(long)]
        primitive_id: Option<String>,
        /// Optional required field names in the event.
        #[arg(long = "field-name")]
        field_names: Vec<String>,
        /// Optional provider/emitter name.
        #[arg(long)]
        provider: Option<String>,
        /// Action plan kind.
        #[arg(long = "action-kind")]
        action_kind: String,
        /// Optional action target reference.
        #[arg(long = "action-target")]
        action_target: Option<String>,
        /// Durable action instruction.
        #[arg(long = "action-instruction")]
        action_instruction: String,
    },
    /// Evaluates all active triggers against an existing ledger entry.
    Evaluate {
        /// Zero-based ledger entry index to evaluate.
        #[arg(long = "entry-index")]
        entry_index: usize,
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
