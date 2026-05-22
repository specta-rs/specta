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

    /// IO error during file operations.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Circular reference detected during topological sort.
    #[error("Topological sort error: {0}")]
    TopoSort(String),

    /// Format or serde error.
    #[error("Format error: {message}: {source}")]
    Format {
        message: &'static str,
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
