//! Canonical error types shared across WorkGraph crates.

/// Canonical result type for WorkGraph operations.
pub type Result<T> = std::result::Result<T, WorkgraphError>;

/// Canonical error type used by WorkGraph foundation and kernel crates.
#[derive(Debug, thiserror::Error)]
pub enum WorkgraphError {
    /// Indicates a primitive store failure.
    #[error("store error: {0}")]
    StoreError(String),
    /// Indicates an immutable ledger failure.
    #[error("ledger error: {0}")]
    LedgerError(String),
    /// Indicates a primitive registry failure.
    #[error("registry error: {0}")]
    RegistryError(String),
    /// Indicates invalid input or failed validation.
    #[error("validation error: {0}")]
    ValidationError(String),
    /// Indicates an underlying I/O failure.
    #[error("i/o error: {0}")]
    IoError(#[from] std::io::Error),
    /// Indicates an encoding or decoding failure.
    #[error("encoding error: {0}")]
    EncodingError(String),
}

impl WorkgraphError {
    /// Returns the stable machine-readable error code for this variant.
    #[must_use]
    pub fn code(&self) -> &'static str {
        match self {
            Self::StoreError(_) => "store_error",
            Self::LedgerError(_) => "ledger_error",
            Self::RegistryError(_) => "registry_error",
            Self::ValidationError(_) => "validation_error",
            Self::IoError(_) => "io_error",
            Self::EncodingError(_) => "encoding_error",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Result, WorkgraphError};

    #[test]
    fn code_matches_each_variant() {
        let cases = [
            (WorkgraphError::StoreError("store".into()), "store_error"),
            (WorkgraphError::LedgerError("ledger".into()), "ledger_error"),
            (
                WorkgraphError::RegistryError("registry".into()),
                "registry_error",
            ),
            (
                WorkgraphError::ValidationError("validation".into()),
                "validation_error",
            ),
            (
                WorkgraphError::IoError(std::io::Error::other("io")),
                "io_error",
            ),
            (
                WorkgraphError::EncodingError("encoding".into()),
                "encoding_error",
            ),
        ];

        for (error, expected_code) in cases {
            assert_eq!(error.code(), expected_code);
        }
    }

    #[test]
    fn display_includes_variant_context() {
        let store_error = WorkgraphError::StoreError("failed to read decision".into());
        assert_eq!(
            store_error.to_string(),
            "store error: failed to read decision"
        );

        let io_error = WorkgraphError::IoError(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "permission denied",
        ));
        assert_eq!(io_error.code(), "io_error");
        assert!(io_error.to_string().contains("permission denied"));
    }

    #[test]
    fn io_error_from_std_error_preserves_source_kind() {
        let error: WorkgraphError =
            std::io::Error::new(std::io::ErrorKind::NotFound, "missing").into();

        match error {
            WorkgraphError::IoError(source) => {
                assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
                assert_eq!(source.to_string(), "missing");
            }
            other => panic!("expected IoError, got {other:?}"),
        }
    }

    #[test]
    fn result_alias_uses_workgraph_error() {
        fn fail() -> Result<()> {
            Err(WorkgraphError::ValidationError("bad title".into()))
        }

        let error = fail().expect_err("validation failure should bubble up");
        assert_eq!(error.code(), "validation_error");
    }
}
