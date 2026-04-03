//! Implementation of the `workgraph create` command.

use anyhow::{Context, bail};
use wg_store::{PrimitiveFrontmatter, StoredPrimitive, read_primitive};
use wg_types::ActorId;

use crate::app::AppContext;
use crate::args::KeyValueInput;
use crate::output::CreateOutput;
use crate::services::mutation::PrimitiveMutationService;
use crate::util::fields::{resolve_body_input, split_body_and_frontmatter};
use crate::util::slug::{slugify, validate_or_derive_id};

/// Creates a new primitive and appends a matching ledger entry.
///
/// # Errors
///
/// Returns an error when the workspace metadata is missing, the primitive type is unknown,
/// validation fails, or persistence cannot be completed.
pub async fn handle(
    app: &AppContext,
    primitive_type: &str,
    title: &str,
    id: Option<&str>,
    body: Option<&str>,
    stdin_body: bool,
    fields: &[KeyValueInput],
) -> anyhow::Result<CreateOutput> {
    let registry = app.load_registry().await?;
    let runtime_registry = app.load_runtime_registry().await?;

    if runtime_registry.get_type(primitive_type).is_none() {
        bail!("unknown primitive type '{primitive_type}'");
    }

    let config = app.load_config().await?;
    let actor = config
        .default_actor_id
        .unwrap_or_else(|| ActorId::new("cli"));
    let resolved_id = validate_or_derive_id(id, title)?;
    let resolved_body = resolve_body_input(body, stdin_body, fields)?;
    let (_, extra_fields) = split_body_and_frontmatter(fields);
    let reference = format!("{primitive_type}/{resolved_id}");
    let path = app
        .workspace()
        .primitive_path(primitive_type, &resolved_id)
        .as_path()
        .display()
        .to_string();
    let primitive = StoredPrimitive {
        frontmatter: PrimitiveFrontmatter {
            r#type: primitive_type.to_owned(),
            id: resolved_id.clone(),
            title: title.to_owned(),
            extra_fields,
        },
        body: resolved_body,
    };

    if let Ok(existing) = read_primitive(app.workspace(), primitive_type, &resolved_id).await {
        if existing == primitive {
            return Ok(CreateOutput {
                reference,
                dry_run: app.dry_run(),
                idempotent: true,
                path,
                primitive: existing,
                ledger_entry: None,
            });
        }

        bail!(
            "primitive '{primitive_type}/{resolved_id}' already exists with different content; choose a different --id or update the existing primitive"
        );
    }

    if app.dry_run() {
        return Ok(CreateOutput {
            reference,
            dry_run: true,
            idempotent: false,
            path,
            primitive,
            ledger_entry: None,
        });
    }

    let (path, ledger_entry) = PrimitiveMutationService::new(app, &registry)
        .create(actor, &primitive)
        .await
        .with_context(|| format!("failed to create '{}'", slugify(title)))?;

    Ok(CreateOutput {
        reference,
        dry_run: false,
        idempotent: false,
        path,
        primitive,
        ledger_entry: Some(ledger_entry),
    })
}
