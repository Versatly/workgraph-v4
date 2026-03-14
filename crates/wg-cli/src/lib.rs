//! Clap-based CLI commands for initializing and operating a WorkGraph workspace.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::str::FromStr;

use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use clap::{Parser, Subcommand};
use serde_yaml::Value;
use uuid::Uuid;
use wg_encoding::{to_yaml_string, write_frontmatter};
use wg_ledger::{append, verify_chain};
use wg_paths::WorkspacePath;
use wg_store::{PrimitiveFrontmatter, StoredPrimitive, query, read_primitive, write_primitive};
use wg_types::{ActorId, LedgerEntryInput, LedgerOp, PrimitiveType, WorkgraphConfig};

/// Top-level CLI arguments for the `workgraph` binary.
#[derive(Debug, Parser)]
#[command(name = "workgraph", about = "WorkGraph v4 CLI")]
pub struct Cli {
    /// Path to the workspace root.
    #[arg(long, global = true, default_value = ".")]
    pub workspace: PathBuf,
    /// CLI subcommand to execute.
    #[command(subcommand)]
    pub command: Commands,
}

/// CLI subcommands.
#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Initialize a workspace structure and metadata files.
    Init,
    /// Show high-level workspace status.
    Status,
    /// Query primitives by type and optional field filters.
    Query {
        /// Primitive type to query.
        primitive_type: String,
        /// Repeated `field=value` filter.
        #[arg(long = "filter")]
        filter: Vec<String>,
    },
    /// Create a primitive with title and optional extra fields.
    Create {
        /// Primitive type to create.
        primitive_type: String,
        /// Primitive title.
        #[arg(long)]
        title: String,
        /// Repeated `key=value` field assignment.
        #[arg(long = "field")]
        field: Vec<String>,
    },
    /// Show a primitive by `<type>/<id>`.
    Show {
        /// Resource identifier in `<type>/<id>` format.
        resource: String,
    },
}

/// Runs the CLI command against the selected workspace.
pub fn run(cli: Cli) -> Result<()> {
    let workspace = WorkspacePath::new(cli.workspace);

    match cli.command {
        Commands::Init => cmd_init(&workspace),
        Commands::Status => cmd_status(&workspace),
        Commands::Query {
            primitive_type,
            filter,
        } => cmd_query(&workspace, &primitive_type, &filter),
        Commands::Create {
            primitive_type,
            title,
            field,
        } => cmd_create(&workspace, &primitive_type, &title, &field),
        Commands::Show { resource } => cmd_show(&workspace, &resource),
    }
}

/// Initializes a workspace directory with primitive stores and metadata.
pub fn init_workspace(workspace: &WorkspacePath) -> Result<()> {
    fs::create_dir_all(workspace.hidden_dir())?;
    for primitive_type in PrimitiveType::builtins() {
        fs::create_dir_all(workspace.store_dir_for(&primitive_type).as_path())?;
    }

    let config_path = workspace.config_path();
    if !config_path.exists() {
        let config = WorkgraphConfig::default();
        let yaml = to_yaml_string(&config)?;
        fs::write(config_path, yaml)?;
    }

    let ledger_path = workspace.ledger_path();
    if !ledger_path.as_path().exists() {
        fs::write(ledger_path.as_path(), b"" as &[u8])?;
    }

    Ok(())
}

fn cmd_init(workspace: &WorkspacePath) -> Result<()> {
    init_workspace(workspace)?;
    println!("Initialized workspace at {}", workspace.as_path().display());
    Ok(())
}

fn cmd_status(workspace: &WorkspacePath) -> Result<()> {
    let mut total = 0_usize;
    println!("Workspace: {}", workspace.as_path().display());

    for primitive_type in PrimitiveType::builtins() {
        let count = wg_store::list_primitives(workspace, primitive_type.clone())?.len();
        if count > 0 {
            println!("{}: {}", primitive_type.as_str(), count);
        }
        total += count;
    }

    println!("total: {total}");

    verify_chain(workspace).context("ledger verification failed")?;
    println!("ledger: ok");

    Ok(())
}

fn cmd_query(workspace: &WorkspacePath, primitive_type: &str, filters: &[String]) -> Result<()> {
    let primitive_type = parse_primitive_type(primitive_type)?;
    let filters = parse_key_value_pairs(filters)?;

    let results = query(workspace, primitive_type, &filters)?;
    for primitive in results {
        println!(
            "{}/{} | {}",
            primitive.frontmatter.primitive_type,
            primitive.frontmatter.id,
            primitive.frontmatter.title
        );
    }

    Ok(())
}

fn cmd_create(
    workspace: &WorkspacePath,
    primitive_type: &str,
    title: &str,
    fields: &[String],
) -> Result<()> {
    let primitive_type = parse_primitive_type(primitive_type)?;
    let mut id = slugify(title);
    if id.is_empty() {
        id = format!("item-{}", Uuid::new_v4().simple());
    }

    let path = workspace.store_dir_for(&primitive_type).primitive_file(&id);
    if path.exists() {
        id = format!("{}-{}", id, &Uuid::new_v4().simple().to_string()[..8]);
    }

    let frontmatter = PrimitiveFrontmatter {
        primitive_type: primitive_type.clone(),
        id: id.clone(),
        title: title.to_owned(),
        fields: parse_value_pairs(fields)?,
    };

    let primitive = StoredPrimitive::new(frontmatter.clone(), String::new());
    write_primitive(workspace, &primitive)?;

    let mut fields_changed = vec!["title".to_owned()];
    fields_changed.extend(frontmatter.fields.keys().cloned());
    append(
        workspace,
        LedgerEntryInput {
            ts: Utc::now(),
            actor: ActorId("cli".to_owned()),
            op: LedgerOp::Create,
            primitive_type,
            primitive_id: id.clone(),
            fields_changed,
        },
    )?;

    println!("created {}/{}", frontmatter.primitive_type, id);
    Ok(())
}

fn cmd_show(workspace: &WorkspacePath, resource: &str) -> Result<()> {
    let (type_part, id_part) = resource
        .split_once('/')
        .ok_or_else(|| anyhow!("resource must be in <type>/<id> format"))?;

    let primitive_type = parse_primitive_type(type_part)?;
    let primitive = read_primitive(workspace, primitive_type, id_part)?;
    let markdown = write_frontmatter(&primitive.frontmatter, &primitive.body)?;
    println!("{markdown}");
    Ok(())
}

fn parse_primitive_type(value: &str) -> Result<PrimitiveType> {
    PrimitiveType::from_str(value).map_err(|_| anyhow!("unknown primitive type: {value}"))
}

fn parse_key_value_pairs(values: &[String]) -> Result<BTreeMap<String, String>> {
    let mut map = BTreeMap::new();
    for value in values {
        let (key, val) = value
            .split_once('=')
            .ok_or_else(|| anyhow!("expected key=value, got {value}"))?;
        if key.trim().is_empty() {
            return Err(anyhow!("filter key cannot be empty"));
        }
        map.insert(key.trim().to_owned(), val.trim().to_owned());
    }
    Ok(map)
}

fn parse_value_pairs(values: &[String]) -> Result<BTreeMap<String, Value>> {
    let mut map = BTreeMap::new();
    for value in values {
        let (key, val) = value
            .split_once('=')
            .ok_or_else(|| anyhow!("expected key=value, got {value}"))?;
        if key.trim().is_empty() {
            return Err(anyhow!("field key cannot be empty"));
        }
        map.insert(key.trim().to_owned(), Value::String(val.trim().to_owned()));
    }
    Ok(map)
}

fn slugify(input: &str) -> String {
    let mut slug = String::new();
    let mut last_was_dash = false;

    for character in input.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            last_was_dash = false;
        } else if !last_was_dash {
            slug.push('-');
            last_was_dash = true;
        }
    }

    slug.trim_matches('-').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugify_normalizes_titles() {
        assert_eq!(slugify("Acme Corp"), "acme-corp");
        assert_eq!(slugify("  Hello___World  "), "hello-world");
    }
}
