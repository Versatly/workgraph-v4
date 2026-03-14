#![forbid(unsafe_code)]

//! Cursor background agent adapter placeholder.

use wg_adapter_api::{AdapterRequest, AdapterStatus, RuntimeAdapter};

/// Placeholder adapter for Cursor-managed runs.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct CursorAdapter;

impl CursorAdapter {
    /// Stable kind string for the adapter.
    pub const KIND: &str = "cursor";

    /// Creates a new placeholder adapter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl RuntimeAdapter for CursorAdapter {
    fn kind(&self) -> &'static str {
        Self::KIND
    }

    fn submit(&self, _request: AdapterRequest<'_>) -> AdapterStatus {
        AdapterStatus::Noop
    }
}
