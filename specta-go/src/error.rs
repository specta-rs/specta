use std::{error, fmt, io, path::PathBuf};

use specta::datatype::RecursiveInlineType;

use crate::Layout;

/// Errors that can occur while generating Go bindings.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// A format callback failed.
    Format {
        /// The stage or datatype path where formatting failed.
        message: String,
        /// The underlying format error.
        source: specta::FormatError,
    },
    /// A Go identifier could not be generated from a Rust name.
    InvalidName {
        /// Path to the item containing the name.
        path: String,
        /// The invalid source name.
        name: String,
    },
    /// Two Rust items map to the same Go identifier.
    DuplicateName {
        /// Scope containing the collision.
        path: String,
        /// Colliding Go identifier.
        name: String,
    },
    /// A map key cannot be represented by Go's JSON encoder.
    InvalidMapKey {
        /// Path to the map.
        path: String,
        /// Description of the unsupported key.
        reason: String,
    },
    /// A named reference was absent from the supplied type collection.
    DanglingReference {
        /// Path where the reference was used.
        path: String,
        /// Debug representation of the missing reference.
        reference: String,
    },
    /// An inline reference recursively contains itself.
    RecursiveInline {
        /// Path where recursion was detected.
        path: String,
        /// Description of the inline cycle.
        cycle: RecursiveInlineType,
    },
    /// A Specta datatype has no faithful, compilable Go representation.
    UnsupportedType {
        /// Path to the unsupported datatype.
        path: String,
        /// Explanation of the Go restriction.
        reason: String,
    },
    /// The selected layout requires [`crate::Go::export_to`].
    ExportRequiresExportTo(Layout),
    /// The configured Go package name is invalid.
    InvalidPackageName(String),
    /// An output path could not be inspected.
    Metadata {
        /// Path that could not be inspected.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },
    /// An output directory could not be created.
    CreateDir {
        /// Directory that could not be created.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },
    /// A generated file could not be written.
    WriteFile {
        /// File that could not be written.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format { message, source } => write!(f, "format error at {message}: {source}"),
            Self::InvalidName { path, name } => {
                write!(f, "cannot generate a Go identifier for {name:?} at {path}")
            }
            Self::DuplicateName { path, name } => {
                write!(f, "duplicate Go identifier {name:?} at {path}")
            }
            Self::InvalidMapKey { path, reason } => {
                write!(f, "invalid Go JSON map key at {path}: {reason}")
            }
            Self::DanglingReference { path, reference } => {
                write!(f, "dangling named reference at {path}: {reference}")
            }
            Self::RecursiveInline { path, cycle } => {
                write!(f, "recursive inline type at {path}: {cycle:?}")
            }
            Self::UnsupportedType { path, reason } => {
                write!(f, "unsupported Go type at {path}: {reason}")
            }
            Self::ExportRequiresExportTo(layout) => {
                write!(f, "the {layout} layout requires Go::export_to")
            }
            Self::InvalidPackageName(name) => write!(f, "invalid Go package name {name:?}"),
            Self::Metadata { path, source } => {
                write!(f, "failed to inspect {}: {source}", path.display())
            }
            Self::CreateDir { path, source } => {
                write!(f, "failed to create {}: {source}", path.display())
            }
            Self::WriteFile { path, source } => {
                write!(f, "failed to write {}: {source}", path.display())
            }
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Format { source, .. } => Some(source.as_ref()),
            Self::Metadata { source, .. }
            | Self::CreateDir { source, .. }
            | Self::WriteFile { source, .. } => Some(source),
            _ => None,
        }
    }
}

impl Error {
    pub(crate) fn format(message: impl Into<String>, source: specta::FormatError) -> Self {
        Self::Format {
            message: message.into(),
            source,
        }
    }
}
