//! Implementation of the `workgraph create` command.

use anyhow::bail;
use wg_store::{PrimitiveFrontmatter, StoredPrimitive};
use wg_types::ActorId;

use crate::app::AppContext;
use crate::args::KeyValueInput;
use crate::output::CreateOutput;
use crate::services::mutation::PrimitiveMutationService;
use crate::util::fields::split_body_and_frontmatter;
use crate::util::slug::unique_slug;

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
    let id = unique_slug(app.workspace(), primitive_type, title).await?;
    let (body, extra_fields) = split_body_and_frontmatter(fields);
    let primitive = StoredPrimitive {
        frontmatter: PrimitiveFrontmatter {
            r#type: primitive_type.to_owned(),
            id: id.clone(),
            title: title.to_owned(),
            extra_fields,
        },
        body,
    };

    let (path, ledger_entry) = PrimitiveMutationService::new(app, &registry)
        .create(actor, &primitive)
        .await?;

    Ok(CreateOutput {
        reference: format!("{primitive_type}/{id}"),
        path,
        primitive,
        ledger_entry,
    })
}
