#![forbid(unsafe_code)]

//! Transport-layer placeholders for inbox, outbox, and relay plumbing.

/// Minimal message envelope for transport stubs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransportEnvelope<'a> {
    /// Named channel used for dispatch.
    pub channel: &'a str,
    /// Opaque payload identifier or body.
    pub payload: &'a str,
}

/// Status returned by the placeholder transport hub.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispatchStatus {
    /// The envelope was queued for later delivery.
    Queued,
    /// The envelope was rejected by the transport.
    Rejected,
}

/// Placeholder transport hub for future event routing.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TransportHub;

impl TransportHub {
    /// Creates a new placeholder transport hub.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Dispatches an envelope using the placeholder contract.
    pub fn dispatch(&self, _envelope: TransportEnvelope<'_>) -> DispatchStatus {
        DispatchStatus::Queued
    }
}
