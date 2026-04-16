//! Error types for the Swift language exporter.

use thiserror::Error;

/// Errors that can occur during Swift code generation.
#[derive(Debug, Error)]
pub enum Error {
    /// Swift does not support this type.
    #[error("Unsupported type: {0}")]
    UnsupportedType(String),

    /// Invalid identifier for Swift.
    #[error("Invalid identifier: {0}")]
    InvalidIdentifier(String),

    /// Circular reference detected in type definitions.
    #[error("Circular reference detected")]
    CircularReference,

    /// Generic constraint error.
    #[error("Generic constraint error: {0}")]
    GenericConstraint(String),

    /// IO error during file operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Invalid configuration.
    #[error("Configuration error: {0}")]
    Configuration(String),

    /// Custom format callback failed.
    #[error("Format error: {message}: {source}")]
    Format {
        message: &'static str,
        source: crate::swift::FormatError,
    },
}

/// Result type alias for Swift export operations.
pub type Result<T> = std::result::Result<T, Error>;

impl Error {
    pub(crate) fn format(message: &'static str, source: crate::swift::FormatError) -> Self {
        Self::Format { message, source }
    }
}
