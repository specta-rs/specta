use std::{borrow::Cow, error, fmt, io, panic::Location, path::PathBuf};

use specta::datatype::OpaqueReference;

use crate::Layout;

/// The error type for the Zod exporter.
#[non_exhaustive]
pub struct Error {
    kind: ErrorKind,
}

type FrameworkSource = Box<dyn error::Error + Send + Sync + 'static>;

#[allow(dead_code)]
enum ErrorKind {
    BigIntForbidden {
        path: String,
    },
    InvalidName {
        path: String,
        name: Cow<'static, str>,
    },
    ForbiddenName {
        path: String,
        name: Cow<'static, str>,
    },
    DuplicateTypeName {
        name: Cow<'static, str>,
        first: String,
        second: String,
    },
    Io(io::Error),
    ReadDir {
        path: PathBuf,
        source: io::Error,
    },
    Metadata {
        path: PathBuf,
        source: io::Error,
    },
    RemoveFile {
        path: PathBuf,
        source: io::Error,
    },
    RemoveDir {
        path: PathBuf,
        source: io::Error,
    },
    UnsupportedOpaqueReference(OpaqueReference),
    DanglingNamedReference {
        reference: String,
    },
    UnresolvedGenericReference {
        reference: String,
    },
    Framework {
        message: Cow<'static, str>,
        source: FrameworkSource,
    },
    UnableToExport(Layout),
}

impl Error {
    /// Construct an error for framework-specific logic.
    pub fn framework(
        message: impl Into<Cow<'static, str>>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self {
            kind: ErrorKind::Framework {
                message: message.into(),
                source: source.into(),
            },
        }
    }

    pub(crate) fn bigint_forbidden(path: String) -> Self {
        Self {
            kind: ErrorKind::BigIntForbidden { path },
        }
    }

    pub(crate) fn invalid_name(path: String, name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            kind: ErrorKind::InvalidName {
                path,
                name: name.into(),
            },
        }
    }

    pub(crate) fn forbidden_name(path: String, name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            kind: ErrorKind::ForbiddenName {
                path,
                name: name.into(),
            },
        }
    }

    pub(crate) fn duplicate_type_name(
        name: Cow<'static, str>,
        first: Location<'static>,
        second: Location<'static>,
    ) -> Self {
        Self {
            kind: ErrorKind::DuplicateTypeName {
                name,
                first: format_location(first),
                second: format_location(second),
            },
        }
    }

    pub(crate) fn read_dir(path: PathBuf, source: io::Error) -> Self {
        Self {
            kind: ErrorKind::ReadDir { path, source },
        }
    }

    pub(crate) fn metadata(path: PathBuf, source: io::Error) -> Self {
        Self {
            kind: ErrorKind::Metadata { path, source },
        }
    }

    pub(crate) fn remove_file(path: PathBuf, source: io::Error) -> Self {
        Self {
            kind: ErrorKind::RemoveFile { path, source },
        }
    }

    pub(crate) fn remove_dir(path: PathBuf, source: io::Error) -> Self {
        Self {
            kind: ErrorKind::RemoveDir { path, source },
        }
    }

    pub(crate) fn unsupported_opaque_reference(reference: OpaqueReference) -> Self {
        Self {
            kind: ErrorKind::UnsupportedOpaqueReference(reference),
        }
    }

    pub(crate) fn dangling_named_reference(reference: String) -> Self {
        Self {
            kind: ErrorKind::DanglingNamedReference { reference },
        }
    }

    pub(crate) fn unresolved_generic_reference(reference: String) -> Self {
        Self {
            kind: ErrorKind::UnresolvedGenericReference { reference },
        }
    }

    pub(crate) fn unable_to_export(layout: Layout) -> Self {
        Self {
            kind: ErrorKind::UnableToExport(layout),
        }
    }

    pub(crate) fn format(
        message: impl Into<Cow<'static, str>>,
        source: crate::FormatError,
    ) -> Self {
        Self {
            kind: ErrorKind::Framework {
                message: message.into(),
                source,
            },
        }
    }
}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self {
            kind: ErrorKind::Io(error),
        }
    }
}

impl From<std::fmt::Error> for Error {
    fn from(source: std::fmt::Error) -> Self {
        Self {
            kind: ErrorKind::Framework {
                message: Cow::Borrowed("Formatting error"),
                source: Box::new(source),
            },
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ErrorKind::BigIntForbidden { path } => write!(
                f,
                "Attempted to export {path:?} but Specta configuration forbids exporting BigInt types (i64, u64, i128, u128) because serializer compatibility is unknown. Configure `BigIntExportBehavior` to allow this."
            ),
            ErrorKind::InvalidName { path, name } => write!(
                f,
                "Attempted to export {path:?} but was unable to due to name {name:?} containing an invalid character. Try renaming it or using `#[specta(rename = \"new_name\")]`"
            ),
            ErrorKind::ForbiddenName { path, name } => write!(
                f,
                "Attempted to export {path:?} but was unable to due to name {name:?} being a reserved keyword in TypeScript. Try renaming it or using `#[specta(rename = \"new_name\")]`"
            ),
            ErrorKind::DuplicateTypeName {
                name,
                first,
                second,
            } => write!(
                f,
                "Detected multiple types with the same name: {name:?} at {first} and {second}"
            ),
            ErrorKind::Io(err) => write!(f, "IO error: {err}"),
            ErrorKind::ReadDir { path, source } => {
                write!(f, "Failed to read directory '{}': {source}", path.display())
            }
            ErrorKind::Metadata { path, source } => {
                write!(
                    f,
                    "Failed to read metadata for '{}': {source}",
                    path.display()
                )
            }
            ErrorKind::RemoveFile { path, source } => {
                write!(f, "Failed to remove file '{}': {source}", path.display())
            }
            ErrorKind::RemoveDir { path, source } => {
                write!(
                    f,
                    "Failed to remove directory '{}': {source}",
                    path.display()
                )
            }
            ErrorKind::UnsupportedOpaqueReference(reference) => write!(
                f,
                "Found unsupported opaque reference '{}'. It is not supported by the Zod exporter.",
                reference.type_name()
            ),
            ErrorKind::DanglingNamedReference { reference } => write!(
                f,
                "Found dangling named reference {reference}. The referenced type is missing from the resolved type collection."
            ),
            ErrorKind::UnresolvedGenericReference { reference } => write!(
                f,
                "Found unresolved generic reference {reference}. The generic is missing from the active named type scope."
            ),
            ErrorKind::Framework { message, source } => {
                let source = source.to_string();
                if message.is_empty() && source.is_empty() {
                    write!(f, "Framework error")
                } else if source.is_empty() {
                    write!(f, "Framework error: {message}")
                } else {
                    write!(f, "Framework error: {message}: {source}")
                }
            }
            ErrorKind::UnableToExport(layout) => {
                write!(
                    f,
                    "Unable to export layout {layout} with `Zod::export`. Use `Zod::export_to` or change layout."
                )
            }
        }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self, f)
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match &self.kind {
            ErrorKind::Io(error) => Some(error),
            ErrorKind::ReadDir { source, .. }
            | ErrorKind::Metadata { source, .. }
            | ErrorKind::RemoveFile { source, .. }
            | ErrorKind::RemoveDir { source, .. } => Some(source),
            ErrorKind::Framework { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}

fn format_location(location: Location<'static>) -> String {
    format!(
        "{}:{}:{}",
        location.file(),
        location.line(),
        location.column()
    )
}
