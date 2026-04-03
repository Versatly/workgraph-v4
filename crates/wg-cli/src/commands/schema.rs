//! Implementation of the `workgraph schema` command.

use anyhow::bail;

use crate::app::AppContext;
use crate::output::{AGENT_SCHEMA_VERSION, SchemaOutput};
use crate::services::discovery::cli_schema;

/// Returns structured command and envelope schema metadata.
///
/// # Errors
///
/// Returns an error when a requested primitive type is unknown.
pub async fn handle(
    app: &AppContext,
    primitive_type: Option<&str>,
) -> anyhow::Result<SchemaOutput> {
    let registry = app.load_registry().await?;
    if let Some(primitive_type) = primitive_type
        && registry.get_type(primitive_type).is_none()
    {
        bail!("unknown primitive type '{primitive_type}'");
    }

    let schema = cli_schema(AGENT_SCHEMA_VERSION, &registry, primitive_type);
    Ok(SchemaOutput {
        schema_version: schema.schema_version,
        envelope_fields: schema.envelope_fields,
        primitive_types: schema.primitive_types,
    })
}
