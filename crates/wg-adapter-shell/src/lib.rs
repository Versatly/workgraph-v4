#![forbid(unsafe_code)]

//! Generic shell subprocess adapter placeholder.

use wg_adapter_api::{AdapterRequest, AdapterStatus, RuntimeAdapter};

/// Placeholder adapter for shell-backed runs.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ShellAdapter;

impl ShellAdapter {
    /// Stable kind string for the adapter.
    pub const KIND: &str = "shell";

    /// Creates a new placeholder adapter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl RuntimeAdapter for ShellAdapter {
    fn kind(&self) -> &'static str {
        Self::KIND
    }

    fn submit(&self, _request: AdapterRequest<'_>) -> AdapterStatus {
        AdapterStatus::Noop
    }
}
