//! Implementation of the `workgraph show` command.

use anyhow::Context;
use wg_store::read_primitive;

use crate::app::AppContext;
use crate::output::ShowOutput;
use crate::util::workspace::parse_reference;

/// Loads and returns a single primitive by `<type>/<id>` reference.
///
/// # Errors
///
/// Returns an error when the reference is invalid or the primitive cannot be read.
pub async fn handle(app: &AppContext, reference: &str) -> anyhow::Result<ShowOutput> {
    let (primitive_type, id) = parse_reference(reference)?;
    let primitive = read_primitive(app.workspace(), primitive_type, id)
        .await
        .with_context(|| format!("failed to read primitive '{reference}'"))?;

    Ok(ShowOutput {
        reference: reference.to_owned(),
        primitive,
    })
}
