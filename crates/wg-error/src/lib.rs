//! Shared error types and result aliases for WorkGraph.

use std::fmt;
use std::io;

use thiserror::Error;

/// Standard result alias for WorkGraph crates.
pub type Result<T> = std::result::Result<T, WorkgraphError>;

/// Stable high-level error code for callers and telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// Input did not satisfy API requirements.
    InvalidInput,
    /// Filesystem or stream IO failure.
    Io,
    /// Serialization or parse failure.
    Encoding,
    /// Requested resource was not found.
    NotFound,
    /// Validation rule failed.
    Validation,
    /// Duplicate/conflicting data.
    Conflict,
    /// Ledger chain integrity issue.
    Integrity,
}

impl ErrorCode {
    /// Returns the stable machine-readable code string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::InvalidInput => "invalid_input",
            Self::Io => "io",
            Self::Encoding => "encoding",
            Self::NotFound => "not_found",
            Self::Validation => "validation",
            Self::Conflict => "conflict",
            Self::Integrity => "integrity",
        }
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Top-level WorkGraph error variants.
#[derive(Debug, Error)]
pub enum WorkgraphError {
    /// IO failure.
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    /// YAML serialization/parsing failure.
    #[error("yaml error: {0}")]
    Yaml(#[from] serde_yaml::Error),
    /// JSON serialization/parsing failure.
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    /// Markdown frontmatter is malformed or missing.
    #[error("invalid markdown frontmatter")]
    InvalidFrontmatter,
    /// Invalid filter expression.
    #[error("invalid filter expression: {0}")]
    InvalidFilter(String),
    /// Primitive type is unknown.
    #[error("invalid primitive type: {0}")]
    InvalidPrimitiveType(String),
    /// Primitive does not exist.
    #[error("primitive not found: {primitive_type}/{id}")]
    NotFound {
        /// Primitive type name.
        primitive_type: String,
        /// Primitive ID.
        id: String,
    },
    /// Duplicate primitive type registration.
    #[error("duplicate primitive type: {0}")]
    DuplicateType(String),
    /// Generic validation failure.
    #[error("validation error: {0}")]
    Validation(String),
    /// Hash chain or ledger integrity failure.
    #[error("integrity error: {0}")]
    Integrity(String),
}

impl WorkgraphError {
    /// Returns the high-level error code for this error.
    #[must_use]
    pub const fn code(&self) -> ErrorCode {
        match self {
            Self::Io(_) => ErrorCode::Io,
            Self::Yaml(_) | Self::Json(_) | Self::InvalidFrontmatter => ErrorCode::Encoding,
            Self::InvalidFilter(_) | Self::InvalidPrimitiveType(_) => ErrorCode::InvalidInput,
            Self::NotFound { .. } => ErrorCode::NotFound,
            Self::DuplicateType(_) => ErrorCode::Conflict,
            Self::Validation(_) => ErrorCode::Validation,
            Self::Integrity(_) => ErrorCode::Integrity,
        }
    }
}
