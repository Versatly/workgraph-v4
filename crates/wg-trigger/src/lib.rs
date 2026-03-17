//! Placeholder trigger evaluation primitives for WorkGraph.

#![forbid(unsafe_code)]

use wg_policy::{PolicyCheck, PolicyDecision};

/// Minimal trigger definition used for scaffolding.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TriggerDefinition {
    /// Human-readable instruction associated with the trigger.
    pub instruction: String,
}

impl TriggerDefinition {
    /// Returns true when the trigger passes the supplied policy check.
    #[must_use]
    pub fn is_allowed(&self, policy: &PolicyCheck) -> bool {
        !self.instruction.is_empty() && matches!(policy.evaluate(), PolicyDecision::Allow)
    }
}
