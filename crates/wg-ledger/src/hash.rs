//! Hash computation for immutable ledger entries.

use sha2::{Digest, Sha256};
use wg_error::{Result, WorkgraphError};
use wg_types::LedgerEntry;

/// Computes the canonical SHA-256 hash for a ledger entry payload.
///
/// The entry's own `hash` field is excluded from the hash material. The
/// previous hash, timestamp, actor, mutation metadata, and optional note are
/// all included.
///
/// # Errors
///
/// Returns an encoding error when the hash material cannot be serialized.
pub(crate) fn compute_entry_hash(entry: &LedgerEntry) -> Result<String> {
    let hash_material = serde_json::json!({
        "actor": &entry.actor,
        "fields_changed": &entry.fields_changed,
        "note": &entry.note,
        "op": entry.op,
        "prev_hash": &entry.prev_hash,
        "primitive_id": &entry.primitive_id,
        "primitive_type": &entry.primitive_type,
        "ts": &entry.ts,
    });

    let encoded = serde_json::to_vec(&hash_material).map_err(|error| {
        WorkgraphError::EncodingError(format!("failed to serialize ledger hash material: {error}"))
    })?;

    Ok(format!("{:x}", Sha256::digest(encoded)))
}
