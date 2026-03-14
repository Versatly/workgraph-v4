//! Implementation of the `workgraph create` command.

use anyhow::{Context, bail};
use wg_clock::RealClock;
use wg_ledger::{LedgerEntryDraft, LedgerWriter};
use wg_store::{PrimitiveFrontmatter, StoredPrimitive, write_primitive};
use wg_types::{ActorId, LedgerOp};

use crate::app::AppContext;
use crate::args::KeyValueInput;
use crate::output::CreateOutput;
use crate::util::fields::{changed_fields, split_body_and_frontmatter};
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

    let path = write_primitive(app.workspace(), &registry, &primitive)
        .await
        .with_context(|| format!("failed to create {primitive_type}/{id}"))?;

    let writer = LedgerWriter::new(app.root().to_path_buf(), RealClock::new());
    let ledger_entry = writer
        .append(LedgerEntryDraft {
            actor,
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
