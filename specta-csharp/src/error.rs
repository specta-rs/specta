use std::{error, fmt, io, path::PathBuf};

use crate::Layout;

/// An error produced while generating C# bindings.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// A type graph or datatype formatter failed.
    Format {
        /// The formatter phase that failed.
        message: &'static str,
        /// Datatype path for a datatype-level failure.
        path: Option<String>,
        /// The underlying formatting error.
        source: specta::FormatError,
    },
    /// A C# identifier was empty or invalid.
    InvalidName {
        /// Rust path of the offending item.
        path: String,
        /// Invalid identifier.
        name: String,
    },
    /// Two exported types produce the same C# name in one scope.
    DuplicateTypeName {
        /// Colliding C# name.
        name: String,
        /// First Rust type path.
        first: String,
        /// Second Rust type path.
        second: String,
    },
    /// A named reference was not present in the supplied type collection.
    DanglingReference {
        /// Export location where the reference occurred.
        path: String,
    },
    /// A named reference resolves to a type that is intentionally not emitted.
    HiddenReference {
        /// Export location where the reference occurred.
        path: String,
        /// Rust path of the hidden named type.
        name: String,
    },
    /// A registered root has a wire shape that C# cannot declare as a named type.
    UnsupportedRoot {
        /// Rust path of the unsupported root.
        path: String,
    },
    /// An anonymous structural type cannot be declared at this C# use site.
    UnsupportedType {
        /// Export location of the unsupported type.
        path: String,
        /// Kind of structural type encountered.
        kind: &'static str,
    },
    /// A recursively inlined reference would require infinite expansion.
    RecursiveInline {
        /// Export location of the recursive inline reference.
        path: String,
    },
    /// No C# representation was configured for an opaque reference.
    UnsupportedOpaque {
        /// Export location of the opaque reference.
        path: String,
        /// Opaque Rust type name.
        name: String,
    },
    /// The operation cannot represent the configured layout.
    ExportRequiresExportTo(Layout),
    /// A filesystem operation failed.
    Io {
        /// Affected path.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },
}

impl Error {
    pub(crate) fn format(message: &'static str, source: specta::FormatError) -> Self {
        Self::Format {
            message,
            path: None,
            source,
        }
    }

    pub(crate) fn format_at(
        message: &'static str,
        path: impl Into<String>,
        source: specta::FormatError,
    ) -> Self {
        Self::Format {
            message,
            path: Some(path.into()),
            source,
        }
    }

    pub(crate) fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format {
                message,
                path,
                source,
            } => match path {
                Some(path) => write!(f, "{message} at {path}: {source}"),
                None => write!(f, "{message}: {source}"),
            },
            Self::InvalidName { path, name } => {
                write!(f, "'{name}' is not a valid C# identifier at {path}")
            }
            Self::DuplicateTypeName {
                name,
                first,
                second,
            } => write!(
                f,
                "C# type name '{name}' is produced by both {first} and {second}"
            ),
            Self::DanglingReference { path } => {
                write!(
                    f,
                    "named reference at {path} is missing from the type collection"
                )
            }
            Self::HiddenReference { path, name } => {
                write!(
                    f,
                    "named type '{name}' referenced at {path} is not exported"
                )
            }
            Self::UnsupportedRoot { path } => write!(
                f,
                "registered non-object type '{path}' cannot be declared as a C# root"
            ),
            Self::UnsupportedType { path, kind } => {
                write!(
                    f,
                    "anonymous {kind} type at {path} cannot be represented in C#"
                )
            }
            Self::RecursiveInline { path } => {
                write!(f, "recursive inline reference at {path} cannot be expanded")
            }
            Self::UnsupportedOpaque { path, name } => {
                write!(
                    f,
                    "opaque type '{name}' at {path} has no configured C# mapping"
                )
            }
            Self::ExportRequiresExportTo(layout) => {
                write!(f, "the {layout} layout requires CSharp::export_to")
            }
            Self::Io { path, source } => write!(f, "failed to access {}: {source}", path.display()),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Format { source, .. } => Some(source.as_ref()),
            Self::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}
