#![forbid(unsafe_code)]

//! Cross-workspace federation placeholders.

/// Describes a remote workspace peer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FederationPeer<'a> {
    /// Stable workspace identifier for the remote peer.
    pub workspace: &'a str,
    /// Reachable endpoint for the peer.
    pub endpoint: &'a str,
}

/// Status returned by the placeholder federation service.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FederationStatus {
    /// The peer is reachable and ready to federate.
    Connected,
    /// The peer is not currently reachable.
    Disconnected,
}

/// Placeholder federation service for future cross-workspace exchange.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FederationService;

impl FederationService {
    /// Creates a new placeholder federation service.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Probes a remote peer using the placeholder contract.
    pub fn probe(&self, _peer: FederationPeer<'_>) -> FederationStatus {
        FederationStatus::Disconnected
    }
}
