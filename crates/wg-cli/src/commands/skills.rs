//! Implementation of the `workgraph skills` command.

use crate::output::SkillsOutput;
use crate::services::discovery::skills_catalog;

/// Returns structured CLI capability and workflow discovery metadata.
#[must_use]
pub fn handle() -> SkillsOutput {
    let catalog = skills_catalog();
    SkillsOutput {
        recommended_format: catalog.recommended_format,
        workflows: catalog.workflows,
        commands: catalog.commands,
    }
}
