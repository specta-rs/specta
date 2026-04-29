use std::{borrow::Cow, error, fmt, io, panic::Location, path::PathBuf};

use specta::datatype::OpaqueReference;

use crate::Layout;

use super::legacy::ExportPath;

/// The error type for the TypeScript exporter.
///
/// ## BigInt Forbidden
///
/// Specta Typescript intentionally forbids exporting BigInt-style Rust integer types.
/// This includes [usize], [isize], [i64], [u64], [u128], [i128] and [f128].
///
/// This guard exists because `JSON.parse` will truncate large integers to fit into a JavaScript `number` type so we explicitly forbid exporting them.
///
/// If you encounter this error, there are a few common migration paths (in order of preference):
///
/// 1. Use a smaller integer types (any of `u8`/`i8`/`u16`/`i16`/`u32`/`i32`/`f64`).
///    - Only possible when the biggest integer you need to represent is small enough to be represented by a `number` in JS.
///    - This approach forces your application code to handle overflow/underflow values explicitly
///    - Downside is that it can introduce annoying glue code and doesn't actually work if your need large values.
///
/// 2. Serialize the value as a string
///     - This can be done using `#[specta(type = String)]` combined with a Serde `#[serde(with = "...")]` attribute.
///     - Downside is that it can introduce annoying glue code, both on in Rust and in JS as you will need to turn it back into a `new BigInt(myString)` in JS.
///
/// 3. Use a Specta-based framework
///     - Frameworks like [Tauri Specta](https://github.com/specta-rs/tauri-specta) and [TauRPC](https://github.com/MatsDK/TauRPC) take care of this for you.
///     - They use special internals to preserve the values and make use of [`specta-tags`](http://docs.rs/specta-tags) for generating glue-code automatically.
///
/// 4. UNSAFE: Accept precision loss
///     - Accept that large numbers may be deserialized differently and use `#[specta(type = f64)]` to bypass this warning on a per-field basis.
///     - This can't be set globally as it is designed intentionally to introduce friction, as you are accepting the risk of data loss which is not okay.
///
#[non_exhaustive]
pub struct Error {
    kind: ErrorKind,
}

type FrameworkSource = Box<dyn error::Error + Send + Sync + 'static>;
const BIGINT_DOCS_URL: &str =
    "https://docs.rs/specta-typescript/latest/specta_typescript/struct.Error.html#bigint-forbidden";

#[allow(dead_code)]
enum ErrorKind {
    InvalidMapKey {
        path: String,
        reason: Cow<'static, str>,
    },
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
    /// Found a named reference that cannot be resolved from the provided
    /// [`Types`](specta::Types).
    DanglingNamedReference {
        reference: String,
    },
    /// An error occurred in your exporter framework.
    Framework {
        message: Cow<'static, str>,
        source: FrameworkSource,
    },
    /// An error occurred in a format callback.
    Format {
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
    pub(crate) fn invalid_map_key(
        path: impl Into<String>,
        reason: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            kind: ErrorKind::InvalidMapKey {
                path: path.into(),
                reason: reason.into(),
            },
        }
    }

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

    /// Construct an error for custom format callbacks.
    pub(crate) fn format(
        message: impl Into<Cow<'static, str>>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self {
            kind: ErrorKind::Format {
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
            ErrorKind::InvalidMapKey { path, reason } => {
                write!(f, "Invalid map key at '{path}': {reason}")
            }
            ErrorKind::BigIntForbidden { path } => write!(
                f,
                "Attempted to export {path:?} but Specta forbids exporting BigInt-style types (usize, isize, i64, u64, i128, u128) to avoid precision loss. See {BIGINT_DOCS_URL} for a full explanation."
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
                "Found dangling named reference {reference}. The referenced type is missing from the resolved type collection."
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
            ErrorKind::Format { message, source } => {
                let source = source.to_string();
                if message.is_empty() && source.is_empty() {
                    write!(f, "Format error")
                } else if source.is_empty() {
                    write!(f, "Format error: {message}")
                } else {
                    write!(f, "Format error: {message}: {source}")
                }
            }
            ErrorKind::BigIntForbiddenLegacy(path) => write!(
                f,
                "Attempted to export {path:?} but Specta forbids exporting BigInt-style types (usize, isize, i64, u64, i128, u128) to avoid precision loss. See {BIGINT_DOCS_URL} for a full explanation."
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
            ErrorKind::Framework { source, .. } | ErrorKind::Format { source, .. } => {
                Some(source.as_ref())
            }
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
