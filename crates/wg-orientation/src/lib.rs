//! Placeholder status and briefing primitives for WorkGraph.

#![forbid(unsafe_code)]

/// Single line of status output.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct StatusLine(pub String);

/// Minimal briefing container.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Briefing {
    /// Heading that describes the briefing.
    pub heading: String,
    /// Individual status lines.
    pub items: Vec<StatusLine>,
}

impl Briefing {
    /// Returns true when the briefing has no items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}
