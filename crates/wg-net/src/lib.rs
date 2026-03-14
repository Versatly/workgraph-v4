#![forbid(unsafe_code)]

//! Networking placeholders for peer discovery and connectivity checks.

/// Describes a peer on the WorkGraph network.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkPeer<'a> {
    /// Stable node identifier for the peer.
    pub node_id: &'a str,
    /// Reachable address for the peer.
    pub address: &'a str,
}

/// Reachability state returned by the placeholder network service.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Reachability {
    /// The peer is currently reachable.
    Reachable,
    /// The peer is currently unreachable.
    Unreachable,
}

/// Placeholder network service for discovery and connectivity probes.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NetworkService;

impl NetworkService {
    /// Creates a new placeholder network service.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Checks whether a peer is reachable.
    pub fn probe(&self, _peer: NetworkPeer<'_>) -> Reachability {
        Reachability::Unreachable
    }
}
