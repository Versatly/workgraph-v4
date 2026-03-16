//! Implementation of the `workgraph schema` command.

use crate::output::{AGENT_SCHEMA_VERSION, SchemaOutput};
use crate::services::discovery::cli_schema;

/// Returns structured command and envelope schema metadata.
#[must_use]
pub fn handle(command: Option<&str>) -> SchemaOutput {
    let schema = cli_schema(AGENT_SCHEMA_VERSION, command);
    SchemaOutput {
        schema_version: schema.schema_version,
        envelope_fields: schema.envelope_fields,
        commands: schema.commands,
    }
}
