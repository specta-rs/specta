use std::{borrow::Cow, io, path::PathBuf};

use specta::datatype::{OpaqueReference, RecursiveInlineType};

/// Error returned by the JSON Schema exporter.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Failed to create the parent directory for an exported schema.
    #[error("failed to create directory '{}': {source}", path.display())]
    CreateDir {
        /// Directory which could not be created.
        path: PathBuf,
        /// Source filesystem error.
        source: io::Error,
    },

    /// Failed to write an exported schema.
    #[error("failed to write JSON Schema to '{}': {source}", path.display())]
    WriteFile {
        /// File which could not be written.
        path: PathBuf,
        /// Source filesystem error.
        source: io::Error,
    },

    /// JSON serialization failed.
    #[error("JSON serialization error: {0}")]
    Json(#[from] serde_json::Error),

    /// A format integration failed while rewriting the Specta type graph.
    #[error("Format error: {message}: {source}")]
    Format {
        /// Operation that failed.
        message: &'static str,
        /// Source format error.
        source: specta::FormatError,
    },

    /// A named reference was not present in the provided [`specta::Types`] collection.
    #[error("dangling named reference at {path}: {reference}")]
    DanglingNamedReference {
        /// Schema path being rendered.
        path: String,
        /// Debug representation of the unresolved reference.
        reference: String,
    },

    /// A map key cannot be represented as JSON object property names.
    #[error("invalid map key at {path}: {reason}")]
    InvalidMapKey {
        /// Schema path being rendered.
        path: String,
        /// Why the key is unsupported.
        reason: Cow<'static, str>,
    },

    /// An opaque exporter-specific type was encountered.
    #[error("unsupported opaque reference at {path}: {reference:?}")]
    UnsupportedOpaqueReference {
        /// Schema path being rendered.
        path: String,
        /// Opaque reference.
        reference: OpaqueReference,
    },

    /// A recursive inline type cannot be represented anonymously.
    #[error("recursive inline type at {path}: {cycle:?}")]
    InfiniteRecursiveInlineType {
        /// Schema path being rendered.
        path: String,
        /// Recursive inline cycle.
        cycle: RecursiveInlineType,
    },

    /// Anonymous inline rendering exceeded the recursion limit.
    #[error("inline recursion limit exceeded at {path}")]
    InlineRecursionLimitExceeded {
        /// Schema path being rendered.
        path: String,
    },

    /// The requested root definition is not present in the exported document.
    #[error("definition '{definition}' was not found")]
    MissingDefinition {
        /// Requested definition key.
        definition: String,
    },

    /// Multiple named datatypes map to the same JSON Schema definition key.
    #[error("duplicate JSON Schema definition key '{key}' for '{first}' and '{second}'")]
    DuplicateDefinitionName {
        /// Conflicting JSON Schema definition key.
        key: String,
        /// First Rust type path.
        first: String,
        /// Second Rust type path.
        second: String,
    },

    /// A recursive generic changes its arguments on every expansion.
    #[error("expanding recursive generic '{type_path}' at {path}")]
    ExpandingRecursiveGeneric {
        /// Schema path being rendered.
        path: String,
        /// Recursive Rust type path.
        type_path: String,
    },

    /// Draft 7 cannot close a non-mergeable intersection with dynamic object keys.
    #[error("cannot close Draft 7 intersection with dynamic properties at {path}")]
    UnsupportedClosedIntersection {
        /// Schema path being rendered.
        path: String,
    },
}

impl Error {
    pub(crate) fn format(message: &'static str, source: specta::FormatError) -> Self {
        Self::Format { message, source }
    }

    pub(crate) fn dangling(path: impl Into<String>, reference: impl Into<String>) -> Self {
        Self::DanglingNamedReference {
            path: path.into(),
            reference: reference.into(),
        }
    }
}
