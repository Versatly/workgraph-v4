#![forbid(unsafe_code)]

//! Shared contracts for runtime adapters in the execution layer.

/// Describes a unit of work handed to a runtime adapter.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AdapterRequest<'a> {
    /// Stable identifier for the run being dispatched.
    pub run_id: &'a str,
}

/// Lightweight status returned by placeholder adapters.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AdapterStatus {
    /// The adapter accepted the request.
    Accepted,
    /// The adapter intentionally performed no work.
    Noop,
}

/// Shared trait for execution runtimes such as Cursor, Claude, or shell.
pub trait RuntimeAdapter {
    /// Returns the stable adapter kind used in logs and configuration.
    fn kind(&self) -> &'static str;

    /// Submits a request using the crate's minimal placeholder contract.
    fn submit(&self, request: AdapterRequest<'_>) -> AdapterStatus;
}
