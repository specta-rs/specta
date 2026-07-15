use std::{error, fmt, io, path::PathBuf};

use crate::Layout;

/// Errors that can occur while generating Kotlin source.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// A Specta datatype has no direct Kotlin representation.
    UnsupportedType {
        /// Location in the exported type graph.
        path: String,
        /// Why Kotlin cannot express the datatype.
        reason: &'static str,
    },
    /// A name cannot be represented by a Kotlin identifier, even when escaped.
    InvalidIdentifier {
        /// Location in the exported type graph.
        path: String,
        /// Invalid identifier.
        name: String,
    },
    /// A declaration or package would shadow a root namespace used by generated Kotlin code.
    ReservedNamespace {
        /// Location of the conflicting name.
        path: String,
        /// Conflicting namespace segment.
        name: String,
    },
    /// A named reference was not present in the supplied type collection.
    DanglingReference {
        /// Location of the missing reference.
        path: String,
    },
    /// An inline recursive reference would expand forever.
    RecursiveInlineType {
        /// Location of the recursive inline expansion.
        path: String,
    },
    /// Multiple declarations would have the same Kotlin name.
    DuplicateTypeName {
        /// Colliding Kotlin declaration or filename.
        name: String,
    },
    /// Naming conversion produced duplicate identifiers in one Kotlin declaration.
    DuplicateIdentifier {
        /// Declaration containing the collision.
        path: String,
        /// Colliding generated identifier.
        name: String,
    },
    /// [`Kotlin::export`](crate::Kotlin::export) cannot return the configured layout.
    ExportRequiresExportTo(Layout),
    /// A formatter callback failed.
    Format {
        /// The stage at which formatting failed.
        message: &'static str,
        /// The underlying formatting error.
        source: specta::FormatError,
    },
    /// A directory could not be created.
    CreateDir {
        /// Directory that could not be created.
        path: PathBuf,
        /// Underlying IO error.
        source: io::Error,
    },
    /// A generated file could not be written.
    WriteFile {
        /// File that could not be written.
        path: PathBuf,
        /// Underlying IO error.
        source: io::Error,
    },
    /// A generated-file manifest could not be read.
    ReadFile {
        /// File that could not be read.
        path: PathBuf,
        /// Underlying IO error.
        source: io::Error,
    },
    /// A stale generated file could not be removed.
    RemoveFile {
        /// File that could not be removed.
        path: PathBuf,
        /// Underlying IO error.
        source: io::Error,
    },
}

impl Error {
    pub(crate) fn format(message: &'static str, source: specta::FormatError) -> Self {
        Self::Format { message, source }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedType { path, reason } => {
                write!(f, "unsupported type at {path}: {reason}")
            }
            Self::InvalidIdentifier { path, name } => {
                write!(f, "invalid Kotlin identifier '{name}' at {path}")
            }
            Self::ReservedNamespace { path, name } => {
                write!(f, "reserved Kotlin namespace '{name}' at {path}")
            }
            Self::DanglingReference { path } => write!(f, "dangling named reference at {path}"),
            Self::RecursiveInlineType { path } => {
                write!(f, "recursive inline type at {path}")
            }
            Self::DuplicateTypeName { name } => write!(f, "duplicate Kotlin type name: {name}"),
            Self::DuplicateIdentifier { path, name } => {
                write!(f, "duplicate Kotlin identifier '{name}' in {path}")
            }
            Self::ExportRequiresExportTo(layout) => {
                write!(f, "layout {layout:?} requires Kotlin::export_to")
            }
            Self::Format { message, source } => write!(f, "format error: {message}: {source}"),
            Self::CreateDir { path, source } => {
                write!(
                    f,
                    "failed to create directory '{}': {source}",
                    path.display()
                )
            }
            Self::WriteFile { path, source } => {
                write!(f, "failed to write '{}': {source}", path.display())
            }
            Self::ReadFile { path, source } => {
                write!(f, "failed to read '{}': {source}", path.display())
            }
            Self::RemoveFile { path, source } => {
                write!(f, "failed to remove '{}': {source}", path.display())
            }
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Format { source, .. } => Some(source.as_ref()),
            Self::CreateDir { source, .. }
            | Self::WriteFile { source, .. }
            | Self::ReadFile { source, .. }
            | Self::RemoveFile { source, .. } => Some(source),
            _ => None,
        }
    }
}
