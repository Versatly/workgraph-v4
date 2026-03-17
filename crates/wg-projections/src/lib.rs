#![forbid(unsafe_code)]

//! Read-model projection placeholders for operator-facing views.

/// Describes a projection refresh request.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProjectionRequest<'a> {
    /// Named projection to refresh.
    pub view: &'a str,
    /// Subject identifier for the projection.
    pub subject: &'a str,
}

/// Status returned by the placeholder projection engine.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectionStatus {
    /// The projection is fresh.
    Fresh,
    /// The projection is stale or unavailable.
    Stale,
}

/// Placeholder projection engine.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ProjectionEngine;

impl ProjectionEngine {
    /// Creates a new placeholder projection engine.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Refreshes a projection using the placeholder contract.
    pub fn refresh(&self, _request: ProjectionRequest<'_>) -> ProjectionStatus {
        ProjectionStatus::Fresh
    }
}
