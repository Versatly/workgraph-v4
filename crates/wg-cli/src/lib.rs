#![forbid(unsafe_code)]
#![deny(missing_docs)]

//! Command-line entrypoints for initializing and interacting with WorkGraph workspaces.

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow, bail};
use clap::{Parser, Subcommand};
use serde::Serialize;
use serde_yaml::Value;
use tokio::fs;
use tracing::debug;
use wg_clock::RealClock;
use wg_ledger::{LedgerCursor, LedgerEntryDraft, LedgerReader, LedgerWriter};
use wg_registry::RuntimeRegistry;
use wg_store::{
    FieldFilter, PrimitiveFrontmatter, StoredPrimitive, list_primitives, query_primitives,
    read_primitive, write_primitive,
};
use wg_types::{ActorId, LedgerEntry, LedgerOp, Registry};

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
    let cli = Cli::try_parse_from(args)?;
    let workspace = workspace_root.as_ref().to_path_buf();
    let output = match cli.command {
        Command::Init => CommandOutput::Init(init_workspace(&workspace).await?),
        Command::Status => CommandOutput::Status(workspace_status(&workspace).await?),
        Command::Create {
            primitive_type,
            title,
            fields,
        } => CommandOutput::Create(
            create_primitive(&workspace, &primitive_type, &title, &fields).await?,
        ),
        Command::Query {
            primitive_type,
            filters,
        } => CommandOutput::Query(query_workspace(&workspace, &primitive_type, &filters).await?),
        Command::Show { reference } => {
            CommandOutput::Show(show_primitive(&workspace, &reference).await?)
        }
    };

    format_output(&output, cli.json)
}

#[derive(Debug, Parser)]
#[command(name = "workgraph", version, about = "WorkGraph v4 CLI")]
struct Cli {
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Init,
    Status,
    Create {
        primitive_type: String,
        #[arg(long)]
        title: String,
        #[arg(long = "field", value_parser = parse_key_value)]
        fields: Vec<KeyValueInput>,
    },
    Query {
        primitive_type: String,
        #[arg(long = "filter", value_parser = parse_key_value)]
        filters: Vec<KeyValueInput>,
    },
    Show {
        reference: String,
    },
}

#[derive(Debug, Clone)]
struct KeyValueInput {
    key: String,
    value: String,
}

#[derive(Debug, Serialize)]
#[serde(tag = "command", content = "result", rename_all = "snake_case")]
enum CommandOutput {
    Init(InitOutput),
    Status(StatusOutput),
    Create(CreateOutput),
    Query(QueryOutput),
    Show(ShowOutput),
}

#[derive(Debug, Serialize)]
struct InitOutput {
    workspace_root: String,
    registry_path: String,
    ledger_path: String,
    created_directories: Vec<String>,
}

#[derive(Debug, Serialize)]
struct StatusOutput {
    workspace_root: String,
    type_counts: BTreeMap<String, usize>,
    last_entry: Option<LedgerEntry>,
}

#[derive(Debug, Serialize)]
struct CreateOutput {
    reference: String,
    path: String,
    primitive: StoredPrimitive,
    ledger_entry: LedgerEntry,
}

#[derive(Debug, Serialize)]
struct QueryOutput {
    primitive_type: String,
    count: usize,
    items: Vec<StoredPrimitive>,
}

#[derive(Debug, Serialize)]
struct ShowOutput {
    reference: String,
    primitive: StoredPrimitive,
}

async fn init_workspace(workspace_root: &Path) -> anyhow::Result<InitOutput> {
    let workspace = wg_paths::WorkspacePath::new(workspace_root.to_path_buf());
    let registry = RuntimeRegistry::with_builtins().into_registry();
    let registry_path = registry_path(&workspace);
    let ledger_path = workspace.ledger_path().into_inner();

    ensure_metadata_dir(&workspace).await?;

    for primitive_type in registry.list_types() {
        fs::create_dir_all(workspace_root.join(&primitive_type.directory))
            .await
            .with_context(|| {
                format!(
                    "failed to create primitive directory '{}'",
                    primitive_type.directory
                )
            })?;
    }

    if !fs::try_exists(&registry_path)
        .await
        .context("failed to inspect registry file")?
    {
        let encoded = serde_yaml::to_string(&registry).context("failed to serialize registry")?;
        wg_fs::atomic_write(&registry_path, encoded.as_bytes())
            .await
            .with_context(|| {
                format!(
                    "failed to write registry file '{}'",
                    registry_path.display()
                )
            })?;
    }

    if !fs::try_exists(&ledger_path)
        .await
        .context("failed to inspect ledger file")?
    {
        wg_fs::atomic_write(&ledger_path, b"")
            .await
            .with_context(|| format!("failed to create ledger file '{}'", ledger_path.display()))?;
    }

    debug!(
        "initialized workgraph workspace at {}",
        workspace_root.display()
    );

    Ok(InitOutput {
        workspace_root: workspace_root.display().to_string(),
        registry_path: registry_path.display().to_string(),
        ledger_path: ledger_path.display().to_string(),
        created_directories: registry
            .list_types()
            .iter()
            .map(|primitive_type| primitive_type.directory.clone())
            .collect(),
    })
}

async fn workspace_status(workspace_root: &Path) -> anyhow::Result<StatusOutput> {
    let workspace = wg_paths::WorkspacePath::new(workspace_root.to_path_buf());
    let registry = load_registry(&workspace).await?;
    let mut counts = BTreeMap::new();

    for primitive_type in registry.list_types() {
        let primitives = list_primitives(&workspace, &primitive_type.name)
            .await
            .with_context(|| format!("failed to list primitive type '{}'", primitive_type.name))?;
        counts.insert(primitive_type.name.clone(), primitives.len());
    }

    let reader = LedgerReader::new(workspace_root.to_path_buf());
    let (entries, _) = reader
        .read_from(LedgerCursor::default())
        .await
        .context("failed to read ledger entries")?;

    Ok(StatusOutput {
        workspace_root: workspace_root.display().to_string(),
        type_counts: counts,
        last_entry: entries.last().cloned(),
    })
}

async fn create_primitive(
    workspace_root: &Path,
    primitive_type: &str,
    title: &str,
    fields: &[KeyValueInput],
) -> anyhow::Result<CreateOutput> {
    let workspace = wg_paths::WorkspacePath::new(workspace_root.to_path_buf());
    let registry = load_registry(&workspace).await?;
    let runtime_registry = RuntimeRegistry::from_registry(registry.clone())
        .context("failed to load runtime registry")?;

    if runtime_registry.get_type(primitive_type).is_none() {
        bail!("unknown primitive type '{primitive_type}'");
    }

    let id = unique_slug(&workspace, primitive_type, title).await?;
    let (body, extra_fields) = build_primitive_fields(fields);
    let primitive = StoredPrimitive {
        frontmatter: PrimitiveFrontmatter {
            r#type: primitive_type.to_owned(),
            id: id.clone(),
            title: title.to_owned(),
            extra_fields,
        },
        body,
    };

    let path = write_primitive(&workspace, &registry, &primitive)
        .await
        .with_context(|| format!("failed to create {primitive_type}/{id}"))?;

    let writer = LedgerWriter::new(workspace_root.to_path_buf(), RealClock::new());
    let ledger_entry = writer
        .append(LedgerEntryDraft {
            actor: ActorId::new("cli"),
            op: LedgerOp::Create,
            primitive_type: primitive_type.to_owned(),
            primitive_id: id.clone(),
            fields_changed: changed_fields(&primitive),
        })
        .await
        .with_context(|| format!("failed to append ledger entry for {primitive_type}/{id}"))?;

    Ok(CreateOutput {
        reference: format!("{primitive_type}/{id}"),
        path: path.as_path().display().to_string(),
        primitive,
        ledger_entry,
    })
}

async fn query_workspace(
    workspace_root: &Path,
    primitive_type: &str,
    filters: &[KeyValueInput],
) -> anyhow::Result<QueryOutput> {
    let workspace = wg_paths::WorkspacePath::new(workspace_root.to_path_buf());
    let registry = load_registry(&workspace).await?;

    if registry.get_type(primitive_type).is_none() {
        bail!("unknown primitive type '{primitive_type}'");
    }

    let filters = filters
        .iter()
        .map(|filter| FieldFilter {
            field: filter.key.clone(),
            value: filter.value.clone(),
        })
        .collect::<Vec<_>>();
    let items = query_primitives(&workspace, primitive_type, &filters)
        .await
        .with_context(|| format!("failed to query primitive type '{primitive_type}'"))?;

    Ok(QueryOutput {
        primitive_type: primitive_type.to_owned(),
        count: items.len(),
        items,
    })
}

async fn show_primitive(workspace_root: &Path, reference: &str) -> anyhow::Result<ShowOutput> {
    let workspace = wg_paths::WorkspacePath::new(workspace_root.to_path_buf());
    let (primitive_type, id) = parse_reference(reference)?;
    let primitive = read_primitive(&workspace, primitive_type, id)
        .await
        .with_context(|| format!("failed to read primitive '{reference}'"))?;

    Ok(ShowOutput {
        reference: reference.to_owned(),
        primitive,
    })
}

fn format_output(output: &CommandOutput, json: bool) -> anyhow::Result<String> {
    if json {
        return serde_json::to_string_pretty(output).context("failed to serialize JSON output");
    }

    Ok(match output {
        CommandOutput::Init(output) => render_init(output),
        CommandOutput::Status(output) => render_status(output),
        CommandOutput::Create(output) => render_create(output),
        CommandOutput::Query(output) => render_query(output),
        CommandOutput::Show(output) => render_show(output),
    })
}

fn render_init(output: &InitOutput) -> String {
    let mut rendered = String::new();
    let _ = writeln!(
        rendered,
        "Initialized WorkGraph workspace at {}",
        output.workspace_root
    );
    let _ = writeln!(rendered, "Registry: {}", output.registry_path);
    let _ = writeln!(rendered, "Ledger: {}", output.ledger_path);
    let _ = writeln!(rendered, "Primitive directories:");
    for directory in &output.created_directories {
        let _ = writeln!(rendered, "- {directory}");
    }
    rendered.trim_end().to_owned()
}

fn render_status(output: &StatusOutput) -> String {
    let mut rendered = String::new();
    let _ = writeln!(rendered, "Workspace: {}", output.workspace_root);
    let _ = writeln!(rendered, "Type counts:");
    for (primitive_type, count) in &output.type_counts {
        let _ = writeln!(rendered, "- {primitive_type}: {count}");
    }

    match &output.last_entry {
        Some(entry) => {
            let _ = writeln!(
                rendered,
                "Last ledger entry: {} {} {}/{}",
                entry.ts.to_rfc3339(),
                entry.actor,
                entry.primitive_type,
                entry.primitive_id
            );
        }
        None => {
            let _ = writeln!(rendered, "Last ledger entry: none");
        }
    }

    rendered.trim_end().to_owned()
}

fn render_create(output: &CreateOutput) -> String {
    format!(
        "Created {} at {}\nLedger hash: {}",
        output.reference, output.path, output.ledger_entry.hash
    )
}

fn render_query(output: &QueryOutput) -> String {
    let mut rendered = String::new();
    let _ = writeln!(
        rendered,
        "{} result(s) for type '{}':",
        output.count, output.primitive_type
    );
    for item in &output.items {
        let _ = writeln!(
            rendered,
            "- {}/{} — {}",
            item.frontmatter.r#type, item.frontmatter.id, item.frontmatter.title
        );
    }
    rendered.trim_end().to_owned()
}

fn render_show(output: &ShowOutput) -> String {
    let mut rendered = String::new();
    let _ = writeln!(rendered, "{}", output.reference);
    let _ = writeln!(rendered, "type: {}", output.primitive.frontmatter.r#type);
    let _ = writeln!(rendered, "id: {}", output.primitive.frontmatter.id);
    let _ = writeln!(rendered, "title: {}", output.primitive.frontmatter.title);
    for (key, value) in &output.primitive.frontmatter.extra_fields {
        let _ = writeln!(rendered, "{key}: {}", yaml_scalar_or_inline(value));
    }
    let _ = writeln!(rendered);
    rendered.push_str(output.primitive.body.trim_end());
    rendered
}

fn yaml_scalar_or_inline(value: &Value) -> String {
    match value {
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        other => serde_yaml::to_string(other)
            .map(|text| text.trim().replace('\n', " "))
            .unwrap_or_else(|_| "<unrenderable>".to_owned()),
    }
}

async fn load_registry(workspace: &wg_paths::WorkspacePath) -> anyhow::Result<Registry> {
    let path = registry_path(workspace);
    let encoded = fs::read_to_string(&path)
        .await
        .with_context(|| format!("failed to read registry file '{}'", path.display()))?;

    serde_yaml::from_str(&encoded)
        .with_context(|| format!("failed to parse registry file '{}'", path.display()))
}

fn registry_path(workspace: &wg_paths::WorkspacePath) -> PathBuf {
    workspace.as_path().join(".workgraph").join("registry.yaml")
}

async fn ensure_metadata_dir(workspace: &wg_paths::WorkspacePath) -> anyhow::Result<()> {
    wg_fs::ensure_dir(workspace.as_path().join(".workgraph"))
        .await
        .context("failed to create .workgraph directory")
}

fn build_primitive_fields(fields: &[KeyValueInput]) -> (String, BTreeMap<String, Value>) {
    let mut body = String::new();
    let mut extra_fields = BTreeMap::new();

    for field in fields {
        if field.key == "body" {
            body = field.value.clone();
        } else {
            extra_fields.insert(field.key.clone(), parse_scalar_value(&field.value));
        }
    }

    (body, extra_fields)
}

fn parse_scalar_value(input: &str) -> Value {
    if let Ok(value) = input.parse::<i64>() {
        return Value::Number(value.into());
    }

    if let Ok(value) = input.parse::<f64>() {
        return serde_yaml::to_value(value).unwrap_or_else(|_| Value::String(input.to_owned()));
    }

    match input {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        _ => Value::String(input.to_owned()),
    }
}

fn changed_fields(primitive: &StoredPrimitive) -> Vec<String> {
    let mut fields = vec!["id".to_owned(), "title".to_owned(), "type".to_owned()];
    fields.extend(primitive.frontmatter.extra_fields.keys().cloned());
    if !primitive.body.is_empty() {
        fields.push("body".to_owned());
    }
    fields.sort();
    fields
}

async fn unique_slug(
    workspace: &wg_paths::WorkspacePath,
    primitive_type: &str,
    title: &str,
) -> anyhow::Result<String> {
    let base = slugify(title);
    let mut candidate = base.clone();
    let mut suffix = 2_usize;

    loop {
        let exists = fs::try_exists(
            workspace
                .primitive_path(primitive_type, &candidate)
                .as_path(),
        )
        .await
        .with_context(|| {
            format!(
                "failed to inspect primitive path for type '{primitive_type}' and id '{candidate}'"
            )
        })?;

        if !exists {
            return Ok(candidate);
        }

        candidate = format!("{base}-{suffix}");
        suffix += 1;
    }
}

fn slugify(input: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;

    for character in input.chars().flat_map(char::to_lowercase) {
        if character.is_ascii_alphanumeric() {
            slug.push(character);
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }

    let slug = slug.trim_matches('-').to_owned();
    if slug.is_empty() {
        "untitled".to_owned()
    } else {
        slug
    }
}

fn parse_reference(reference: &str) -> anyhow::Result<(&str, &str)> {
    reference
        .split_once('/')
        .ok_or_else(|| anyhow!("primitive reference must be in the form <type>/<id>"))
}

fn parse_key_value(input: &str) -> Result<KeyValueInput, String> {
    let (key, value) = input
        .split_once('=')
        .ok_or_else(|| "expected key=value".to_owned())?;

    if key.trim().is_empty() {
        return Err("field key must not be empty".to_owned());
    }

    Ok(KeyValueInput {
        key: key.trim().to_owned(),
        value: value.to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::{execute, parse_key_value, slugify};
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
        assert!(parse_key_value("status=decided").is_ok());
        assert!(parse_key_value("missing").is_err());
        assert!(parse_key_value("=bad").is_err());
    }

    #[tokio::test]
    async fn execute_supports_init_create_status_query_show_and_json() {
        let temp_dir = tempdir().expect("temporary directory should be created");

        let init_output = execute(["workgraph", "init"], temp_dir.path())
            .await
            .expect("init should succeed");
        assert!(init_output.contains("Initialized WorkGraph workspace"));

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
    }
}
