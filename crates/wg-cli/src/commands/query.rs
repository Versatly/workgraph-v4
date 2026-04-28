//! Implementation of the `workgraph query` command.

use anyhow::bail;
use wg_store::{FieldFilter, FilterOperator, query_primitives};

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

    let Some(primitive_definition) = registry.get_type(primitive_type) else {
        bail!("unknown primitive type '{primitive_type}'");
    };

    let filters = filters
        .iter()
        .map(|filter| FieldFilter {
            field: filter.key.clone(),
            operator: if filter.value.is_empty() {
                FilterOperator::Present
            } else {
                FilterOperator::Exact
            },
            value: filter.value.clone(),
        })
        .collect::<Vec<_>>();
    let items = query_primitives(app.workspace(), &registry, primitive_type, &filters).await?;

    Ok(QueryOutput {
        primitive_type: primitive_type.to_owned(),
        applied_filters: filters
            .iter()
            .map(|filter| match filter.operator {
                FilterOperator::Present => format!("{} is present", filter.field),
                FilterOperator::Exact => format!("{}={}", filter.field, filter.value),
            })
            .collect(),
        count: items.len(),
        items,
        summary_fields: primitive_definition
            .fields
            .iter()
            .filter(|field| {
                matches!(
                    field.name.as_str(),
                    "status" | "role" | "team_ids" | "client_id" | "org_id" | "account_owner"
                ) || field.repeated
            })
            .map(|field| field.name.clone())
            .collect(),
    })
}
