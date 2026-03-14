//! Placeholder autonomy loop primitives for WorkGraph.

#![forbid(unsafe_code)]

use wg_orientation::Briefing;

/// Minimal self-healing loop state.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AutonomyLoop {
    /// Latest briefing available to the loop.
    pub briefing: Briefing,
    /// Whether the loop is enabled.
    pub enabled: bool,
}

impl AutonomyLoop {
    /// Returns true when the loop has enough context to continue.
    #[must_use]
    pub fn should_continue(&self) -> bool {
        self.enabled && !self.briefing.is_empty()
    }
}
