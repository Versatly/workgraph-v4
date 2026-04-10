#![forbid(unsafe_code)]

//! Contracts for external event sources and reconciliation flows.

use chrono::{DateTime, Utc};
use std::collections::BTreeMap;
use wg_types::{ActorId, EventEnvelope, EventSourceKind};

/// Normalized externally-originated event used by connector placeholders.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExternalEvent {
    /// Stable event identifier used for replay-safe deduplication.
    pub id: String,
    /// Stable source identifier, such as a provider name.
    pub source: String,
    /// Stable event name emitted by the provider.
    pub event_name: String,
    /// Resource or subject associated with the event.
    pub subject: String,
    /// Optional acting actor associated with the event.
    pub actor_id: Option<ActorId>,
    /// Time the external event occurred.
    pub occurred_at: DateTime<Utc>,
    /// Normalized payload values retained for trigger matching.
    pub payload_fields: BTreeMap<String, String>,
}

impl ExternalEvent {
    /// Converts the external event into the normalized WorkGraph trigger envelope.
    #[must_use]
    pub fn into_event_envelope(self) -> EventEnvelope {
        let subject_reference = self.subject.contains('/').then_some(self.subject.clone());
        EventEnvelope {
            id: self.id,
            source: EventSourceKind::Webhook,
            event_name: Some(self.event_name),
            provider: Some(self.source),
            actor_id: self.actor_id,
            occurred_at: self.occurred_at,
            op: None,
            primitive_type: subject_reference.as_deref().and_then(|reference| {
                reference
                    .split_once('/')
                    .map(|(primitive_type, _)| primitive_type.to_owned())
            }),
            primitive_id: subject_reference
                .as_deref()
                .and_then(|reference| reference.split_once('/').map(|(_, id)| id.to_owned())),
            subject_reference,
            field_names: self.payload_fields.keys().cloned().collect(),
            payload_fields: self.payload_fields,
        }
    }
}

/// Polling result returned by a placeholder event source.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PollStatus {
    /// The source has events available to consume.
    EventsAvailable,
    /// The source is currently idle.
    Idle,
}

/// Result of a placeholder reconciliation attempt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReconcileStatus {
    /// The event was applied to local state.
    Applied,
    /// The event was intentionally skipped.
    Skipped,
}

/// Source of externally-originated events.
pub trait EventSource {
    /// Returns the stable source kind used in configuration.
    fn source_kind(&self) -> &'static str;

    /// Polls the source using a minimal placeholder contract.
    fn poll(&self) -> PollStatus;
}

/// Reconciles external events into WorkGraph state.
pub trait Reconciler {
    /// Returns the stable reconciler kind used in configuration.
    fn reconciler_kind(&self) -> &'static str;

    /// Applies connector-specific reconciliation logic.
    fn reconcile(&self, event: ExternalEvent) -> ReconcileStatus;
}
