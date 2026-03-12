use std::{borrow::Cow, error, fmt, io, panic::Location, path::PathBuf};

use specta::datatype::OpaqueReference;

use crate::Layout;

use super::legacy::ExportPath;

/// The error type for the TypeScript exporter.
#[non_exhaustive]
pub struct Error {
    kind: ErrorKind,
}

type FrameworkSource = Box<dyn error::Error + Send + Sync + 'static>;

#[allow(dead_code)]
enum ErrorKind {
    /// Attempted to export a bigint type but the configuration forbids it.
    BigIntForbidden {
        path: String,
    },
    /// A type's name conflicts with a reserved keyword in Typescript.
    ForbiddenName {
        path: String,
        name: &'static str,
    },
    /// A type's name contains invalid characters or is not valid.
    InvalidName {
        path: String,
        name: Cow<'static, str>,
    },
    /// Detected multiple items within the same scope with the same name.
    /// Typescript doesn't support this so we error out.
    ///
    /// Using anything other than [Layout::FlatFile] should make this basically impossible.
    DuplicateTypeName {
        name: Cow<'static, str>,
        first: String,
        second: String,
    },
    /// An filesystem IO error.
    /// This is possible when using `Typescript::export_to` when writing to a file or formatting the file.
    Io(io::Error),
    /// Failed to read a directory while exporting files.
    ReadDir {
        path: PathBuf,
        source: io::Error,
    },
    /// Failed to inspect filesystem metadata while exporting files.
    Metadata {
        path: PathBuf,
        source: io::Error,
    },
    /// Failed to remove a stale file while exporting files.
    RemoveFile {
        path: PathBuf,
        source: io::Error,
    },
    /// Failed to remove an empty directory while exporting files.
    RemoveDir {
        path: PathBuf,
        source: io::Error,
    },
    /// Found an opaque reference which the Typescript exporter doesn't know how to handle.
    /// You may be referencing a type which is not supported by the Typescript exporter.
    UnsupportedOpaqueReference(OpaqueReference),
    /// Found a named reference that cannot be resolved from the provided [`Types`](specta::Types).
    DanglingNamedReference {
        reference: String,
    },
    /// Found a generic reference that cannot be resolved to a declared generic name.
    UnresolvedGenericReference {
        reference: String,
    },
    /// An error occurred in your exporter framework.
    Framework {
        message: Cow<'static, str>,
        source: FrameworkSource,
    },

    //
    //
    // TODO: Break
    //
    //
    BigIntForbiddenLegacy(ExportPath),
    ForbiddenNameLegacy(ExportPath, &'static str),
    InvalidNameLegacy(ExportPath, String),
    FmtLegacy(std::fmt::Error),
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

    pub(crate) fn forbidden_name_legacy(path: ExportPath, name: &'static str) -> Self {
        Self {
            kind: ErrorKind::ForbiddenNameLegacy(path, name),
        }
    }

    pub(crate) fn invalid_name_legacy(path: ExportPath, name: String) -> Self {
        Self {
            kind: ErrorKind::InvalidNameLegacy(path, name),
        }
    }

    pub(crate) fn unable_to_export(layout: Layout) -> Self {
        Self {
            kind: ErrorKind::UnableToExport(layout),
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
    fn from(error: std::fmt::Error) -> Self {
        Self {
            kind: ErrorKind::FmtLegacy(error),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ErrorKind::BigIntForbidden { path } => write!(
                f,
                "Attempted to export {path:?} but Specta configuration forbids exporting BigInt types (i64, u64, i128, u128) because we don't know if your se/deserializer supports it. If your using a serializer/deserializer that natively has support for BigInt types you can disable this warning by editing your `ExportConfiguration`!"
            ),
            ErrorKind::ForbiddenName { path, name } => write!(
                f,
                "Attempted to export {path:?} but was unable to due toname {name:?} conflicting with a reserved keyword in Typescript. Try renaming it or using `#[specta(rename = \"new name\")]`"
            ),
            ErrorKind::InvalidName { path, name } => write!(
                f,
                "Attempted to export {path:?} but was unable to due to name {name:?} containing an invalid character. Try renaming it or using `#[specta(rename = \"new name\")]`"
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
                "Found unsupported opaque reference '{}'. It is not supported by the Typescript exporter.",
                reference.type_name()
            ),
            ErrorKind::DanglingNamedReference { reference } => write!(
                f,
                "Found dangling named reference {reference}. The referenced type is missing from `Types`."
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
            ErrorKind::BigIntForbiddenLegacy(path) => write!(
                f,
                "Attempted to export {path:?} but Specta configuration forbids exporting BigInt types (i64, u64, i128, u128) because we don't know if your se/deserializer supports it. You can change this behavior by editing your `ExportConfiguration`!"
            ),
            ErrorKind::ForbiddenNameLegacy(path, name) => write!(
                f,
                "Attempted to export {path:?} but was unable to due to name {name:?} conflicting with a reserved keyword in Typescript. Try renaming it or using `#[specta(rename = \"new name\")]`"
            ),
            ErrorKind::InvalidNameLegacy(path, name) => write!(
                f,
                "Attempted to export {path:?} but was unable to due to name {name:?} containing an invalid character. Try renaming it or using `#[specta(rename = \"new name\")]`"
            ),
            ErrorKind::FmtLegacy(err) => write!(f, "formatter: {err:?}"),
            ErrorKind::UnableToExport(layout) => write!(
                f,
                "Unable to export layout {layout} with the current configuration. Maybe try `Exporter::export_to` or switching to Typescript."
            ),
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
            ErrorKind::FmtLegacy(error) => Some(error),
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
