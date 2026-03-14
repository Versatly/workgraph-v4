//! Placeholder thread lifecycle primitives for WorkGraph.

#![forbid(unsafe_code)]

/// Identifies a thread in placeholder APIs.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct ThreadId(pub String);

impl ThreadId {
    /// Creates a new thread identifier.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
}

/// Describes the current lifecycle state of a thread.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThreadState {
    /// The thread exists but has not started execution.
    #[default]
    Draft,
    /// The thread is currently active.
    Active,
    /// The thread is no longer accepting new work.
    Closed,
}

/// Minimal placeholder handle for a thread.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ThreadHandle {
    /// Stable identifier for the thread.
    pub id: ThreadId,
    /// Current lifecycle state.
    pub state: ThreadState,
}

impl ThreadHandle {
    /// Returns a copy of the handle marked active.
    #[must_use]
    pub fn activate(mut self) -> Self {
        self.state = ThreadState::Active;
        self
    }
}
