#![forbid(unsafe_code)]

//! Contracts for external event sources and reconciliation flows.

/// Minimal event envelope shared by connector placeholders.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalEvent<'a> {
    /// Stable source identifier, such as a provider name.
    pub source: &'a str,
    /// Resource or subject associated with the event.
    pub subject: &'a str,
}

/// Polling result returned by a placeholder event source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PollStatus {
    /// The source has events available to consume.
    EventsAvailable,
    /// The source is currently idle.
    Idle,
}

/// Result of a placeholder reconciliation attempt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReconcileStatus {
    /// The event was applied to local state.
    Applied,
    /// The event was intentionally skipped.
    Skipped,
}

/// Source of externally-originated events.
pub trait EventSource {
    /// Returns the stable source kind used in configuration.
    fn source_kind(&self) -> &'static str;

    /// Polls the source using a minimal placeholder contract.
    fn poll(&self) -> PollStatus;
}

/// Reconciles external events into WorkGraph state.
pub trait Reconciler {
    /// Returns the stable reconciler kind used in configuration.
    fn reconciler_kind(&self) -> &'static str;

    /// Applies connector-specific reconciliation logic.
    fn reconcile(&self, event: ExternalEvent<'_>) -> ReconcileStatus;
}
