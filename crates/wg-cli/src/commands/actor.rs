//! Implementation of the `workgraph actor` command family.

use anyhow::{Context, bail};
use wg_store::read_primitive;

use crate::app::AppContext;
use crate::args::KeyValueInput;
use crate::output::{ActorListOutput, ActorRegisterOutput, ActorShowOutput};

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

    let mut fields = vec![KeyValueInput {
        key: "id".to_owned(),
        value: args.id.clone(),
    }];

    if let Some(email) = args.email.filter(|value| !value.trim().is_empty()) {
        fields.push(KeyValueInput {
            key: "email".to_owned(),
            value: email,
        });
    }
    if let Some(runtime) = args.runtime.filter(|value| !value.trim().is_empty()) {
        fields.push(KeyValueInput {
            key: "runtime".to_owned(),
            value: runtime,
        });
    }
    if let Some(parent_actor_id) = args
        .parent_actor_id
        .filter(|value| !value.trim().is_empty())
    {
        fields.push(KeyValueInput {
            key: "parent_actor_id".to_owned(),
            value: parent_actor_id,
        });
    }
    if let Some(root_actor_id) = args.root_actor_id.filter(|value| !value.trim().is_empty()) {
        fields.push(KeyValueInput {
            key: "root_actor_id".to_owned(),
            value: root_actor_id,
        });
    }
    if let Some(lineage_mode) = args.lineage_mode.filter(|value| !value.trim().is_empty()) {
        fields.push(KeyValueInput {
            key: "lineage_mode".to_owned(),
            value: lineage_mode,
        });
    }
    for capability in args
        .capabilities
        .into_iter()
        .filter(|value| !value.trim().is_empty())
    {
        fields.push(KeyValueInput {
            key: "capabilities".to_owned(),
            value: capability,
        });
    }

    let created = crate::commands::create::handle(
        app,
        actor_type,
        Some(&args.title),
        &fields,
        false,
        false,
    )
    .await?;

    Ok(ActorRegisterOutput {
        reference: created.reference,
        primitive: created.primitive,
        ledger_entry: created.ledger_entry,
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
