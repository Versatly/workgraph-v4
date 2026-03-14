//! Implementation of the `workgraph query` command.

use anyhow::bail;
use wg_store::{FieldFilter, query_primitives};

use crate::app::AppContext;
use crate::args::KeyValueInput;
use crate::output::QueryOutput;

/// Queries primitives of a given type using optional exact-match frontmatter filters.
///
/// # Errors
///
/// Returns an error when the primitive type is unknown or the store cannot be queried.
pub async fn handle(
    app: &AppContext,
    primitive_type: &str,
    filters: &[KeyValueInput],
) -> anyhow::Result<QueryOutput> {
    let registry = app.load_registry().await?;

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
    let items = query_primitives(app.workspace(), primitive_type, &filters).await?;

    Ok(QueryOutput {
        primitive_type: primitive_type.to_owned(),
        count: items.len(),
        items,
    })
}
