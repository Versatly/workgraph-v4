//! Implementation of the `workgraph actor` command family.

use std::collections::BTreeMap;

use anyhow::{Context, anyhow, bail};
use serde_yaml::Value;
use tokio::fs;
use wg_store::{PrimitiveFrontmatter, StoredPrimitive, read_primitive};

use crate::app::AppContext;
use crate::output::{ActorListOutput, ActorRegisterOutput, ActorShowOutput};
use crate::services::mutation::PrimitiveMutationService;

/// Parsed arguments for `workgraph actor register`.
#[derive(Debug, Clone)]
pub struct ActorRegisterArgs {
    /// Actor primitive type to create (`person` or `agent`).
    pub actor_type: String,
    /// Stable actor identifier.
    pub id: String,
    /// Human-readable actor title.
    pub title: String,
    /// Optional email for person actors.
    pub email: Option<String>,
    /// Optional primary role for person actors.
    pub role: Option<String>,
    /// Optional team references for person actors.
    pub team_ids: Vec<String>,
    /// Optional runtime for agent actors.
    pub runtime: Option<String>,
    /// Optional tracked parent actor id.
    pub parent_actor_id: Option<String>,
    /// Optional tracked root actor id.
    pub root_actor_id: Option<String>,
    /// Optional lineage mode.
    pub lineage_mode: Option<String>,
    /// Optional advertised capabilities.
    pub capabilities: Vec<String>,
    /// Optional description for agent actors.
    pub description: Option<String>,
    /// Optional owner reference for agent actors.
    pub owner: Option<String>,
    /// Optional tags for person or agent actors.
    pub tags: Vec<String>,
}

/// Registers a new person or agent actor.
///
/// # Errors
///
/// Returns an error when validation fails or the actor cannot be persisted.
pub async fn register(
    app: &AppContext,
    args: ActorRegisterArgs,
) -> anyhow::Result<ActorRegisterOutput> {
    let actor_type = args.actor_type.trim();
    if actor_type != "person" && actor_type != "agent" {
        bail!("actor registration requires --type person or --type agent");
    }

    let registry = app.load_registry().await?;
    let primitive = actor_primitive(actor_type, &args)?;
    let reference = format!("{actor_type}/{}", primitive.frontmatter.id);
    let primitive_path = app
        .workspace()
        .primitive_path(actor_type, &primitive.frontmatter.id);

    if fs::try_exists(primitive_path.as_path())
        .await
        .context("failed to inspect existing actor path")?
    {
        let existing = read_primitive(app.workspace(), actor_type, &primitive.frontmatter.id)
            .await
            .with_context(|| format!("failed to read existing actor '{reference}'"))?;
        if existing == primitive {
            return Ok(ActorRegisterOutput {
                reference,
                primitive,
                ledger_entry: None,
            });
        }

        return Err(anyhow!(
            "actor '{reference}' already exists with different data"
        ));
    }

    let (_, ledger_entry) = PrimitiveMutationService::new(app, &registry)
        .create(app.effective_actor_id().await?, &primitive)
        .await?;

    Ok(ActorRegisterOutput {
        reference,
        primitive,
        ledger_entry: Some(ledger_entry),
    })
}

/// Lists registered actors, optionally filtered by type.
///
/// # Errors
///
/// Returns an error when actor primitives cannot be queried.
pub async fn list(app: &AppContext, actor_type: Option<&str>) -> anyhow::Result<ActorListOutput> {
    let mut items = Vec::new();
    for primitive_type in ["person", "agent"] {
        if actor_type.is_some_and(|requested| requested != primitive_type) {
            continue;
        }

        let query = crate::commands::query::handle(app, primitive_type, &[]).await?;
        items.extend(query.items);
    }

    items.sort_by(|left, right| {
        left.frontmatter
            .r#type
            .cmp(&right.frontmatter.r#type)
            .then(left.frontmatter.id.cmp(&right.frontmatter.id))
    });

    Ok(ActorListOutput {
        count: items.len(),
        items,
    })
}

fn actor_primitive(actor_type: &str, args: &ActorRegisterArgs) -> anyhow::Result<StoredPrimitive> {
    let mut extra_fields = BTreeMap::new();

    if let Some(email) = args
        .email
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        extra_fields.insert("email".to_owned(), Value::String(email.to_owned()));
    }
    if let Some(role) = args
        .role
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        extra_fields.insert("role".to_owned(), Value::String(role.to_owned()));
    }
    let team_ids = args
        .team_ids
        .iter()
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if !team_ids.is_empty() {
        extra_fields.insert(
            "team_ids".to_owned(),
            serde_yaml::to_value(team_ids).context("failed to encode team_ids")?,
        );
    }
    if let Some(runtime) = args
        .runtime
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        extra_fields.insert("runtime".to_owned(), Value::String(runtime.to_owned()));
    }
    if let Some(description) = args
        .description
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        extra_fields.insert(
            "description".to_owned(),
            Value::String(description.to_owned()),
        );
    }
    if let Some(owner) = args
        .owner
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        extra_fields.insert("owner".to_owned(), Value::String(owner.to_owned()));
    }
    if let Some(parent_actor_id) = args
        .parent_actor_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        extra_fields.insert(
            "parent_actor_id".to_owned(),
            Value::String(parent_actor_id.to_owned()),
        );
    }
    if let Some(root_actor_id) = args
        .root_actor_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        extra_fields.insert(
            "root_actor_id".to_owned(),
            Value::String(root_actor_id.to_owned()),
        );
    }
    if let Some(lineage_mode) = args
        .lineage_mode
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        extra_fields.insert(
            "lineage_mode".to_owned(),
            Value::String(lineage_mode.to_owned()),
        );
    }

    let capabilities = args
        .capabilities
        .iter()
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if !capabilities.is_empty() {
        extra_fields.insert(
            "capabilities".to_owned(),
            serde_yaml::to_value(capabilities).context("failed to encode capabilities")?,
        );
    }
    let tags = args
        .tags
        .iter()
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if !tags.is_empty() {
        extra_fields.insert(
            "tags".to_owned(),
            serde_yaml::to_value(tags).context("failed to encode tags")?,
        );
    }

    Ok(StoredPrimitive {
        frontmatter: PrimitiveFrontmatter {
            r#type: actor_type.to_owned(),
            id: args.id.clone(),
            title: args.title.clone(),
            extra_fields,
        },
        body: String::new(),
    })
}

/// Shows one registered actor by reference.
///
/// # Errors
///
/// Returns an error when the actor reference is invalid or the primitive cannot be loaded.
pub async fn show(app: &AppContext, reference: &str) -> anyhow::Result<ActorShowOutput> {
    let (primitive_type, primitive_id) = crate::util::workspace::parse_reference(reference)?;
    if primitive_type != "person" && primitive_type != "agent" {
        bail!("actor show expects a person/<id> or agent/<id> reference");
    }

    let primitive = read_primitive(app.workspace(), primitive_type, primitive_id)
        .await
        .with_context(|| format!("failed to load actor '{reference}'"))?;

    Ok(ActorShowOutput {
        reference: reference.to_owned(),
        primitive,
    })
}
