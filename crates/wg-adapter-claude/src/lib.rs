#![forbid(unsafe_code)]

//! Claude Code adapter placeholder.

use wg_adapter_api::{AdapterRequest, AdapterStatus, RuntimeAdapter};

/// Placeholder adapter for Claude-backed runs.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ClaudeAdapter;

impl ClaudeAdapter {
    /// Stable kind string for the adapter.
    pub const KIND: &str = "claude";

    /// Creates a new placeholder adapter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl RuntimeAdapter for ClaudeAdapter {
    fn kind(&self) -> &'static str {
        Self::KIND
    }

    fn submit(&self, _request: AdapterRequest<'_>) -> AdapterStatus {
        AdapterStatus::Noop
    }
}
