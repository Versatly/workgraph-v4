#![forbid(unsafe_code)]

//! Agent-to-agent signaling placeholders.

/// Minimal signal message routed between peers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SignalMessage<'a> {
    /// Sender identifier for the signal.
    pub from: &'a str,
    /// Recipient identifier for the signal.
    pub to: &'a str,
    /// Opaque signal payload.
    pub payload: &'a str,
}

/// Delivery result returned by the placeholder signal bus.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignalStatus {
    /// The signal was accepted for delivery.
    Delivered,
    /// The signal was intentionally ignored.
    Ignored,
}

/// Placeholder signaling bus for agent communication.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SignalBus;

impl SignalBus {
    /// Creates a new placeholder signal bus.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }

    /// Sends a signal using the placeholder contract.
    pub fn send(&self, _message: SignalMessage<'_>) -> SignalStatus {
        SignalStatus::Delivered
    }
}
