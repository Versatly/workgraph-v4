//! Implementation of the `workgraph capabilities` command.

use crate::output::CapabilitiesOutput;
use crate::services::discovery::capabilities_catalog;

/// Returns structured CLI capability and workflow discovery metadata.
#[must_use]
pub fn handle() -> CapabilitiesOutput {
    let catalog = capabilities_catalog();
    CapabilitiesOutput {
        first_command: catalog.first_command,
        commands: catalog.commands,
    }
}
