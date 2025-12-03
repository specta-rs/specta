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

    /// Serde validation error.
    #[error("Serde validation error: {0}")]
    SerdeValidation(#[from] specta_serde::Error),

    /// Invalid configuration.
    #[error("Configuration error: {0}")]
    Configuration(String),
}

/// Result type alias for Swift export operations.
pub type Result<T> = std::result::Result<T, Error>;
