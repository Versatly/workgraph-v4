#![forbid(unsafe_code)]

//! GitHub connector placeholder for webhooks and reconciliation.

use wg_connector_api::{EventSource, ExternalEvent, PollStatus, ReconcileStatus, Reconciler};

/// Placeholder connector for GitHub-originated events.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GithubConnector;

impl GithubConnector {
    /// Stable kind string for the connector.
    pub const KIND: &str = "github";

    /// Creates a new placeholder connector.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl EventSource for GithubConnector {
    fn source_kind(&self) -> &'static str {
        Self::KIND
    }

    fn poll(&self) -> PollStatus {
        PollStatus::Idle
    }
}

impl Reconciler for GithubConnector {
    fn reconciler_kind(&self) -> &'static str {
        Self::KIND
    }

    fn reconcile(&self, _event: ExternalEvent<'_>) -> ReconcileStatus {
        ReconcileStatus::Skipped
    }
}
