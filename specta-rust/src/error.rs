use std::{error, fmt, io, path::PathBuf};

use crate::Layout;

/// An error produced while generating Rust source.
#[derive(Debug)]
#[non_exhaustive]
pub enum Error {
    /// A format callback failed.
    Format {
        /// The callback being evaluated.
        message: &'static str,
        /// Path within the exported type graph, when known.
        path: String,
        /// The underlying formatter error.
        source: specta::FormatError,
    },
    /// An identifier cannot be represented as a Rust identifier.
    InvalidIdentifier {
        /// Path within the exported type graph.
        path: String,
        /// Invalid identifier.
        name: String,
    },
    /// Two exported types would have the same name in the configured layout.
    DuplicateTypeName {
        /// Conflicting generated name.
        name: String,
        /// Rust paths of the conflicting types.
        types: (String, String),
    },
    /// A named reference was not present in the supplied type collection.
    DanglingReference {
        /// Path within the exported type graph.
        path: String,
        /// Debug representation of the missing reference.
        reference: String,
    },
    /// An opaque exporter-specific reference is unsupported.
    UnsupportedOpaque {
        /// Path within the exported type graph.
        path: String,
        /// Rust name of the opaque state.
        type_name: &'static str,
    },
    /// A structural intersection has no equivalent Rust type expression.
    UnsupportedIntersection {
        /// Path within the exported type graph.
        path: String,
    },
    /// Serde derives were configured over a type graph whose serde
    /// representation has already been lowered into its shape.
    UnsupportedSerdeLowering {
        /// Path within the exported type graph.
        path: String,
    },
    /// A primitive requires an unstable Rust language feature.
    UnsupportedPrimitive {
        /// Path within the exported type graph.
        path: String,
        /// Unsupported primitive name.
        primitive: &'static str,
    },
    /// A generic parameter is not used by the generated declaration.
    UnusedGeneric {
        /// Generated declaration path.
        path: String,
        /// Unused generic parameter.
        name: String,
    },
    /// A generated module and type occupy the same Rust namespace.
    ModuleTypeCollision {
        /// Module scope containing the collision.
        path: String,
        /// Conflicting identifier.
        name: String,
    },
    /// A recursive inline reference could not be reduced to a named Rust type.
    RecursiveInline {
        /// Path within the exported type graph.
        path: String,
        /// Debug representation of the cycle.
        cycle: String,
    },
    /// [`Rust::export`](crate::Rust::export) cannot represent the configured layout.
    ExportRequiresExportTo(Layout),
    /// A filesystem operation failed.
    Io {
        /// Operation being attempted.
        action: &'static str,
        /// Affected path.
        path: PathBuf,
        /// Underlying I/O error.
        source: io::Error,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Format {
                message,
                path,
                source,
            } if path.is_empty() => {
                write!(f, "{message}: {source}")
            }
            Self::Format {
                message,
                path,
                source,
            } => write!(f, "{message} at {path}: {source}"),
            Self::InvalidIdentifier { path, name } => {
                write!(f, "'{name}' at {path} is not a valid Rust identifier")
            }
            Self::DuplicateTypeName {
                name,
                types: (a, b),
            } => {
                write!(f, "types '{a}' and '{b}' both export as '{name}'")
            }
            Self::DanglingReference { path, reference } => {
                write!(f, "dangling named reference {reference} at {path}")
            }
            Self::UnsupportedOpaque { path, type_name } => {
                write!(
                    f,
                    "opaque reference '{type_name}' at {path} is unsupported by the Rust exporter"
                )
            }
            Self::UnsupportedIntersection { path } => {
                write!(
                    f,
                    "structural intersection at {path} has no Rust equivalent"
                )
            }
            Self::UnsupportedSerdeLowering { path } => {
                write!(
                    f,
                    "serde derives at {path} cannot reproduce the source wire shape: \
                     its serde representation was already lowered into the type graph. \
                     Export with `Identity` instead of a serialization format so the \
                     container attributes survive"
                )
            }
            Self::UnsupportedPrimitive { path, primitive } => {
                write!(
                    f,
                    "primitive '{primitive}' at {path} requires unstable Rust"
                )
            }
            Self::UnusedGeneric { path, name } => {
                write!(
                    f,
                    "generic parameter '{name}' is unused by generated type {path}"
                )
            }
            Self::ModuleTypeCollision { path, name } => {
                write!(
                    f,
                    "module and type '{name}' collide in Rust namespace '{path}'"
                )
            }
            Self::RecursiveInline { path, cycle } => {
                write!(
                    f,
                    "recursive inline reference {cycle} at {path} cannot be rendered"
                )
            }
            Self::ExportRequiresExportTo(layout) => {
                write!(f, "layout '{layout}' requires Rust::export_to")
            }
            Self::Io {
                action,
                path,
                source,
            } => {
                write!(f, "failed to {action} '{}': {source}", path.display())
            }
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

impl Error {
    pub(crate) fn format(
        message: &'static str,
        path: impl Into<String>,
        source: specta::FormatError,
    ) -> Self {
        Self::Format {
            message,
            path: path.into(),
            source,
        }
    }

    pub(crate) fn io(action: &'static str, path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::Io {
            action,
            path: path.into(),
            source,
        }
    }
}
