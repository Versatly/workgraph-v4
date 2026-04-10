#![forbid(unsafe_code)]

//! Normalized webhook event ingress helpers for the trigger plane.

use wg_adapter_api::{AdapterRequest, AdapterStatus, RuntimeAdapter};
use wg_types::{EventEnvelope, EventSourceKind};

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

    /// Normalizes a provider webhook payload into a trigger-plane event envelope.
    #[must_use]
    pub fn normalize_event(self, event: EventEnvelope) -> EventEnvelope {
        debug_assert_eq!(event.source, EventSourceKind::Webhook);
        event
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
