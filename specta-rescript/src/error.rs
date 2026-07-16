use thiserror::Error;

/// The error type for the ReScript exporter.
#[derive(Debug, Error)]
pub enum Error {
    /// ReScript does not support this type.
    #[error("Unsupported type: {0}")]
    UnsupportedType(String),

    /// Invalid type usage for ReScript output.
    #[error("Invalid type: {0}")]
    InvalidType(String),

    /// A record label cannot be represented in ReScript source.
    #[error("Invalid ReScript record label: {0}")]
    InvalidRecordLabel(String),

    /// Multiple fields map to the same ReScript record label.
    #[error("Duplicate ReScript record label: {0}")]
    DuplicateRecordLabel(String),

    /// A type name cannot be represented in ReScript source.
    #[error("Invalid ReScript type name: {0}")]
    InvalidTypeName(String),

    /// A variant constructor cannot be represented in ReScript source.
    #[error("Invalid ReScript variant constructor: {0}")]
    InvalidVariantConstructor(String),

    /// A polymorphic variant tag cannot be represented in ReScript source.
    #[error("Invalid ReScript polymorphic variant tag: {0}")]
    InvalidPolymorphicVariant(String),

    /// Multiple Rust types map to the same ReScript type name.
    #[error("Duplicate ReScript type name '{name}' for '{first}' and '{second}'")]
    DuplicateTypeName {
        /// Conflicting ReScript type name.
        name: String,
        /// First Rust type path.
        first: String,
        /// Second Rust type path.
        second: String,
    },

    /// IO error during file operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Format or serde error.
    #[error("Format error: {message}: {source}")]
    Format {
        /// The exporter operation which failed.
        message: &'static str,
        /// The underlying Specta formatting error.
        source: specta::FormatError,
    },

    /// Serde validation error.
    #[error("Serde validation error: {0}")]
    SerdeValidation(#[from] specta_serde::Error),
}

impl Error {
    pub(crate) fn format(message: &'static str, source: specta::FormatError) -> Self {
        Self::Format { message, source }
    }
}

pub type Result<T> = std::result::Result<T, Error>;
