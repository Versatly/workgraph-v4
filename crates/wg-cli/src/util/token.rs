//! Hosted credential token helpers.

use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Generates a new opaque bearer token for hosted invite credentials.
#[must_use]
pub fn generate_token() -> String {
    format!("wg_{}", Uuid::new_v4().simple())
}

/// Returns the stable SHA-256 hex digest stored for a bearer token.
#[must_use]
pub fn token_hash(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    format!("{digest:x}")
}
