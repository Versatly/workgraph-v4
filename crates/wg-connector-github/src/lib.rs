#![forbid(unsafe_code)]

//! GitHub connector placeholder for trigger-plane event normalization.

use wg_connector_api::{EventSource, ExternalEvent, PollStatus, ReconcileStatus, Reconciler};

/// Placeholder connector for GitHub-originated trigger events.
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

    fn reconcile(&self, event: ExternalEvent) -> ReconcileStatus {
        match event.event_name.as_str() {
            "pull_request" | "push" => ReconcileStatus::Applied,
            _ => ReconcileStatus::Skipped,
        }
    }
}
