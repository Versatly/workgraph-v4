//! CLI argument parsing and command definitions.

use clap::{Parser, Subcommand};
use wg_orientation::ContextLens;
use wg_types::RemoteAccessScope;

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
    /// Guides first-run workspace setup for an operator and initial agents.
    #[command(
        after_help = "Examples:\n  workgraph onboard --person-id person:pedro --person-title \"Pedro\" --org-title \"Versatly\" --project-title \"WorkGraph\"\n  workgraph onboard --person-id person:pedro --person-title \"Pedro\" --agent agent:pedro-openclaw=openclaw --agent agent:pedro-hermes=hermes --json"
    )]
    Onboard {
        /// Durable person actor id for the operator.
        #[arg(long = "person-id")]
        person_id: String,
        /// Human-readable operator name.
        #[arg(long = "person-title")]
        person_title: String,
        /// Optional operator email.
        #[arg(long)]
        email: Option<String>,
        /// Optional initial org title to create.
        #[arg(long = "org-title")]
        org_title: Option<String>,
        /// Optional initial project title to create.
        #[arg(long = "project-title")]
        project_title: Option<String>,
        /// Optional initial mission title to create.
        #[arg(long = "mission-title")]
        mission_title: Option<String>,
        /// Optional initial thread title to create.
        #[arg(long = "thread-title")]
        thread_title: Option<String>,
        /// Initial agent expressed as `<actor-id>=<runtime>`.
        #[arg(long = "agent", value_parser = parse_key_value_input)]
        agents: Vec<KeyValueInput>,
    },
    /// Connects the current directory to a hosted WorkGraph server profile.
    #[command(
        after_help = "Examples:\n  workgraph connect --server http://127.0.0.1:8787 --token secret --actor-id person:pedro\n  workgraph --json connect --server http://127.0.0.1:8787 --token secret --actor-id agent:cursor"
    )]
    Connect {
        /// Hosted WorkGraph server base URL.
        #[arg(long)]
        server: String,
        /// Bearer token used for authenticating remote requests.
        #[arg(long)]
        token: String,
        /// Actor identifier to attribute remote work to.
        #[arg(long = "actor-id")]
        actor_id: String,
    },
    /// Shows the effective actor and connection mode for this CLI profile.
    #[command(after_help = "Examples:\n  workgraph whoami\n  workgraph --json whoami")]
    Whoami,
    /// Registers and inspects person/agent actors.
    #[command(
        after_help = "Examples:\n  workgraph actor register --type agent --id agent:cursor --title \"Cursor\" --runtime cursor --capability coding\n  workgraph actor list --type agent\n  workgraph actor show agent/agent:cursor"
    )]
    Actor {
        /// Actor-specific subcommand to execute.
        #[command(subcommand)]
        command: ActorCommand,
    },
    /// Creates and manages actor-bound hosted invite credentials.
    #[command(
        after_help = "Examples:\n  workgraph invite create --actor-id agent:pedro-openclaw --label openclaw --server http://127.0.0.1:8787\n  workgraph invite list\n  workgraph invite revoke openclaw"
    )]
    Invite {
        /// Invite-specific subcommand to execute.
        #[command(subcommand)]
        command: InviteCommand,
    },
    /// Serves the WorkGraph MCP stdio adapter.
    #[command(
        after_help = "Examples:\n  workgraph mcp serve --actor-id agent:cursor\n  workgraph mcp serve --actor-id agent:cursor --access-scope operate\n  workgraph mcp serve --actor-id person:pedro --access-scope admin"
    )]
    Mcp {
        /// MCP-specific subcommand to execute.
        #[command(subcommand)]
        command: McpCommand,
    },
    /// Serves the current workspace over the hosted HTTP API.
    #[command(
        after_help = "Examples:\n  workgraph serve --listen 127.0.0.1:8787\n  workgraph invite create --actor-id agent:cursor --label cursor --server http://127.0.0.1:8787"
    )]
    Serve {
        /// Socket address to bind the hosted server to.
        #[arg(long)]
        listen: String,
    },
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
    /// Manages trigger validation, replay, and event ingestion workflows.
    #[command(
        after_help = "Examples:\n  workgraph trigger validate trigger/react-to-thread-complete\n  workgraph trigger replay --last 20\n  workgraph trigger ingest --source internal --event-name signal.sent --field subject_reference=thread/thread-1 --field actor_id=agent:cursor"
    )]
    Trigger {
        /// Trigger-specific subcommand to execute.
        #[command(subcommand)]
        command: TriggerCommand,
    },
}

impl Command {
    /// Returns the stable command name associated with this parsed command.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::Onboard { .. } => "onboard",
            Self::Connect { .. } => "connect",
            Self::Whoami => "whoami",
            Self::Serve { .. } => "serve",
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
            Self::Trigger { command } => command.name(),
            Self::Actor { command } => command.name(),
            Self::Invite { command } => command.name(),
            Self::Mcp { command } => command.name(),
        }
    }

    /// Returns true when this command may be executed through a hosted remote profile.
    #[must_use]
    pub const fn can_execute_remotely(&self) -> bool {
        !matches!(
            self,
            Self::Init
                | Self::Onboard { .. }
                | Self::Connect { .. }
                | Self::Whoami
                | Self::Serve { .. }
                | Self::Mcp { .. }
        )
    }

    /// Returns the minimum hosted/MCP access scope required to execute this command remotely.
    #[must_use]
    pub const fn required_remote_access_scope(&self) -> RemoteAccessScope {
        match self {
            Self::Init
            | Self::Onboard { .. }
            | Self::Connect { .. }
            | Self::Serve { .. }
            | Self::Mcp { .. } => RemoteAccessScope::Admin,
            Self::Whoami
            | Self::Brief { .. }
            | Self::Status
            | Self::Ledger { .. }
            | Self::Capabilities
            | Self::Schema { .. }
            | Self::Query { .. }
            | Self::Show { .. } => RemoteAccessScope::Read,
            Self::Claim { .. }
            | Self::Complete { .. }
            | Self::Checkpoint { .. }
            | Self::Run { .. } => RemoteAccessScope::Operate,
            Self::Create { .. } => RemoteAccessScope::Admin,
            Self::Trigger { command } => match command {
                TriggerCommand::Validate { .. } => RemoteAccessScope::Read,
                TriggerCommand::Replay { .. } | TriggerCommand::Ingest { .. } => {
                    RemoteAccessScope::Admin
                }
            },
            Self::Actor { command } => match command {
                ActorCommand::List { .. } | ActorCommand::Show { .. } => RemoteAccessScope::Read,
                ActorCommand::Register { .. } => RemoteAccessScope::Admin,
            },
            Self::Invite { command } => match command {
                InviteCommand::List => RemoteAccessScope::Read,
                InviteCommand::Create { .. } | InviteCommand::Revoke { .. } => {
                    RemoteAccessScope::Admin
                }
            },
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

/// Supported `workgraph trigger` subcommands.
#[derive(Debug, Subcommand)]
pub enum TriggerCommand {
    /// Validates a stored trigger definition by `<type>/<id>` reference.
    #[command(
        after_help = "Examples:\n  workgraph trigger validate trigger/react-to-thread-complete\n  workgraph --json trigger validate trigger/react-to-thread-complete"
    )]
    Validate {
        /// Trigger reference in `<type>/<id>` form.
        reference: String,
    },
    /// Replays recent ledger entries through the trigger plane.
    #[command(
        after_help = "Examples:\n  workgraph trigger replay\n  workgraph trigger replay --last 20\n  workgraph --json trigger replay --last 5"
    )]
    Replay {
        /// Number of most recent ledger entries to replay.
        #[arg(long)]
        last: Option<usize>,
    },
    /// Ingests one normalized event into the trigger plane without a live runtime.
    #[command(
        after_help = "Examples:\n  workgraph trigger ingest --source internal --event-name signal.sent --field subject_reference=thread/thread-1\n  workgraph --json trigger ingest --source webhook --provider github --event-name pull_request.merged --field subject_reference=project/dealer-portal"
    )]
    Ingest {
        /// Event source kind.
        #[arg(long)]
        source: String,
        /// Stable event name.
        #[arg(long = "event-name")]
        event_name: Option<String>,
        /// Provider or emitter for webhook/internal events.
        #[arg(long)]
        provider: Option<String>,
        /// Explicit event id. When omitted, a deterministic id is derived from fields.
        #[arg(long = "event-id")]
        event_id: Option<String>,
        /// Event payload fields expressed as `key=value`.
        #[arg(long = "field", value_parser = parse_key_value_input)]
        fields: Vec<KeyValueInput>,
    },
}

impl TriggerCommand {
    /// Returns the stable command name associated with this parsed trigger subcommand.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Validate { .. } => "trigger_validate",
            Self::Replay { .. } => "trigger_replay",
            Self::Ingest { .. } => "trigger_ingest",
        }
    }
}

/// Supported `workgraph actor` subcommands.
#[derive(Debug, Subcommand)]
pub enum ActorCommand {
    /// Registers a new person or agent actor.
    #[command(
        after_help = "Examples:\n  workgraph actor register --type person --id person:pedro --title \"Pedro\" --email pedro@example.com\n  workgraph actor register --type agent --id agent:cursor --title \"Cursor\" --runtime cursor --capability coding"
    )]
    Register {
        /// Actor primitive type to create (`person` or `agent`).
        #[arg(long = "type")]
        actor_type: String,
        /// Stable actor identifier.
        #[arg(long)]
        id: String,
        /// Human-readable actor title.
        #[arg(long)]
        title: String,
        /// Preferred email for person actors.
        #[arg(long)]
        email: Option<String>,
        /// Default runtime for agent actors.
        #[arg(long)]
        runtime: Option<String>,
        /// Optional tracked parent actor above this agent.
        #[arg(long = "parent-actor-id")]
        parent_actor_id: Option<String>,
        /// Optional root tracked actor for delegated lineages.
        #[arg(long = "root-actor-id")]
        root_actor_id: Option<String>,
        /// Optional lineage mode (`tracked` or `opaque`).
        #[arg(long = "lineage-mode")]
        lineage_mode: Option<String>,
        /// Advertised capabilities for an agent actor.
        #[arg(long = "capability")]
        capabilities: Vec<String>,
    },
    /// Lists registered actors.
    #[command(
        after_help = "Examples:\n  workgraph actor list\n  workgraph actor list --type agent\n  workgraph --json actor list"
    )]
    List {
        /// Optional actor type filter (`person` or `agent`).
        #[arg(long = "type")]
        actor_type: Option<String>,
    },
    /// Shows a registered actor by reference.
    #[command(
        after_help = "Examples:\n  workgraph actor show person/person:pedro\n  workgraph --json actor show agent/agent:cursor"
    )]
    Show {
        /// Actor reference in `<type>/<id>` form.
        reference: String,
    },
}

impl ActorCommand {
    /// Returns the stable command name associated with this parsed actor subcommand.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Register { .. } => "actor_register",
            Self::List { .. } => "actor_list",
            Self::Show { .. } => "actor_show",
        }
    }
}

/// Supported `workgraph invite` subcommands.
#[derive(Debug, Subcommand)]
pub enum InviteCommand {
    /// Creates or reuses an actor-bound hosted credential and prints the connect command.
    #[command(
        after_help = "Examples:\n  workgraph invite create --actor-id agent:pedro-openclaw --label openclaw --server http://127.0.0.1:8787\n  workgraph invite create --actor-id agent:pedro-hermes --label hermes --access-scope admin"
    )]
    Create {
        /// Stable invite/credential label.
        #[arg(long)]
        label: String,
        /// Actor identity bound to this invite.
        #[arg(long = "actor-id")]
        actor_id: String,
        /// Server URL agents should use with `workgraph connect`.
        #[arg(long, default_value = "http://127.0.0.1:8787")]
        server: String,
        /// Governance scope granted by the invite.
        #[arg(long = "access-scope", value_parser = parse_remote_access_scope)]
        access_scope: Option<RemoteAccessScopeArg>,
    },
    /// Lists hosted invite credentials without revealing raw tokens.
    #[command(after_help = "Examples:\n  workgraph invite list\n  workgraph --json invite list")]
    List,
    /// Revokes one hosted invite credential by label or id.
    #[command(
        after_help = "Examples:\n  workgraph invite revoke openclaw\n  workgraph invite revoke invite-openclaw"
    )]
    Revoke {
        /// Credential label or id to revoke.
        label_or_id: String,
    },
}

impl InviteCommand {
    /// Returns the stable command name associated with this parsed invite subcommand.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Create { .. } => "invite_create",
            Self::List => "invite_list",
            Self::Revoke { .. } => "invite_revoke",
        }
    }
}

/// Supported `workgraph mcp` subcommands.
#[derive(Debug, Subcommand)]
pub enum McpCommand {
    /// Serves the MCP stdio adapter.
    #[command(
        after_help = "Examples:\n  workgraph mcp serve\n  workgraph mcp serve --actor-id agent:cursor --access-scope operate\n  workgraph mcp serve --actor-id person:pedro --access-scope admin"
    )]
    Serve {
        /// Actor identity bound to the MCP session.
        #[arg(long = "actor-id")]
        actor_id: String,
        /// Governance scope granted to the MCP session.
        #[arg(long = "access-scope", value_parser = parse_remote_access_scope)]
        access_scope: Option<RemoteAccessScopeArg>,
    },
}

impl McpCommand {
    /// Returns the stable command name associated with this parsed MCP subcommand.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Serve { .. } => "mcp_serve",
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

/// A clap-friendly wrapper around [`wg_types::RemoteAccessScope`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RemoteAccessScopeArg(pub RemoteAccessScope);

impl std::fmt::Display for RemoteAccessScopeArg {
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

fn parse_remote_access_scope(input: &str) -> Result<RemoteAccessScopeArg, String> {
    input.parse::<RemoteAccessScope>().map(RemoteAccessScopeArg)
}
