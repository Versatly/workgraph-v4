//! Implementation of the `workgraph capabilities` command.

use crate::output::CapabilitiesOutput;
use crate::services::discovery::capabilities_catalog;

/// Returns structured CLI capability and workflow discovery metadata.
#[must_use]
pub fn handle() -> CapabilitiesOutput {
    let catalog = capabilities_catalog();
    CapabilitiesOutput {
        recommended_format: catalog.recommended_format,
        workflows: catalog.workflows,
        commands: catalog.commands,
        primitive_contracts: catalog.primitive_contracts,
    }
}
