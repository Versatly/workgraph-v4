#![forbid(unsafe_code)]

//! HTTP webhook adapter placeholder.

use wg_adapter_api::{AdapterRequest, AdapterStatus, RuntimeAdapter};

/// Placeholder adapter for webhook-triggered runs.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct WebhookAdapter;

impl WebhookAdapter {
    /// Stable kind string for the adapter.
    pub const KIND: &str = "webhook";

    /// Creates a new placeholder adapter.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl RuntimeAdapter for WebhookAdapter {
    fn kind(&self) -> &'static str {
        Self::KIND
    }

    fn submit(&self, _request: AdapterRequest<'_>) -> AdapterStatus {
        AdapterStatus::Noop
    }
}
