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

    /// Multiple Rust types render as the same top-level Swift declaration.
    #[error("Duplicate Swift type name {name:?}: {first} and {second} both export as {name}")]
    DuplicateTypeName {
        /// The colliding Swift declaration name.
        name: String,
        /// Rust identity of the first declaration.
        first: String,
        /// Rust identity of the second declaration.
        second: String,
    },

    /// Multiple members of one Rust type render as the same Swift identifier.
    #[error(
        "Duplicate Swift member name {name:?} in {type_name}: {first:?} and {second:?} both export as {name}"
    )]
    DuplicateMemberName {
        /// The Rust type whose members collide.
        type_name: String,
        /// The colliding Swift property or case name.
        name: String,
        /// Wire name of the first member.
        first: String,
        /// Wire name of the second member.
        second: String,
    },

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
        /// Context describing which format callback failed.
        message: &'static str,
        /// The underlying format error.
        source: specta::FormatError,
    },
}

impl Error {
    pub(crate) fn format(message: &'static str, source: specta::FormatError) -> Self {
        Self::Format { message, source }
    }
}
