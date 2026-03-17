//! Placeholder mission orchestration primitives for WorkGraph.

#![forbid(unsafe_code)]

/// Tracks a mission's lifecycle in the placeholder API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MissionStatus {
    /// The mission exists but has not started yet.
    #[default]
    Planned,
    /// The mission is currently in progress.
    Running,
    /// The mission finished successfully.
    Completed,
}

/// Minimal mission plan representation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MissionPlan {
    /// Human-readable name for the mission.
    pub name: String,
    /// Current lifecycle status.
    pub status: MissionStatus,
}

impl MissionPlan {
    /// Returns a copy of the plan marked as running.
    #[must_use]
    pub fn start(mut self) -> Self {
        self.status = MissionStatus::Running;
        self
    }
}
