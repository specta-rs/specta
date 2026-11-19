//! Errors produced while exporting Rust types to Elm.

use std::{borrow::Cow, error, fmt, io, path::PathBuf};

use specta::datatype::{NamedDataType, RecursiveInlineType};

use crate::types::Ndt;

/// Error produced while exporting Rust types to Elm.
#[non_exhaustive]
pub struct Error {
    kind: ErrorKind,
    named_datatype: Option<Box<NamedDataType>>,
}

enum ErrorKind {
    /// A map key type cannot be represented as an Elm `Dict` key.
    InvalidMapKey {
        path: String,
        reason: Cow<'static, str>,
    },
    /// Attempted to export an integer which cannot be represented exactly by Elm's `Int`.
    BigIntForbidden { path: String },
    /// A type's name conflicts with a reserved keyword in Elm.
    ForbiddenName { path: String, name: &'static str },
    /// A type's name contains characters which aren't valid in an Elm type name.
    InvalidName {
        path: String,
        name: Cow<'static, str>,
    },
    /// A type's name is empty and cannot be emitted as an Elm type name.
    EmptyName { path: String },
    /// The generated module name isn't a valid Elm module name.
    InvalidModuleName { name: String },
    /// The Elm project's `elm.json` declares no source directories.
    MissingSourceDirectory,
    /// No `elm.json` could be found for the project being exported into.
    ElmJsonNotFound { path: PathBuf },
    /// Failed to read a file.
    ReadFile { path: PathBuf, source: io::Error },
    /// Failed to parse the project's `elm.json`.
    InvalidElmJson {
        path: PathBuf,
        source: serde_json::Error,
    },
    /// Failed to create an output directory.
    CreateDir { path: PathBuf, source: io::Error },
    /// Failed to write an output file.
    WriteFile { path: PathBuf, source: io::Error },
    /// The Rust type has no Elm representation.
    Unsupported {
        path: String,
        reason: Cow<'static, str>,
    },
    /// Found a named reference which is missing from the provided [`Types`](specta::Types).
    DanglingNamedReference { path: String, reference: String },
    /// Found a recursive named reference marked by core inline resolution.
    InfiniteRecursiveInlineType {
        path: String,
        reference: String,
        cycle: RecursiveInlineType,
    },
}

impl Error {
    fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            named_datatype: None,
        }
    }

    /// The named Rust type being exported when this error occurred, if known.
    pub fn named_datatype(&self) -> Option<&NamedDataType> {
        self.named_datatype.as_deref()
    }

    pub(crate) fn with_named_datatype(mut self, ndt: &Ndt) -> Self {
        self.named_datatype
            .get_or_insert_with(|| Box::new(ndt.inner().clone()));
        self
    }

    pub(crate) fn invalid_map_key(
        path: impl Into<String>,
        reason: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self::new(ErrorKind::InvalidMapKey {
            path: path.into(),
            reason: reason.into(),
        })
    }

    pub(crate) fn bigint_forbidden(path: impl Into<String>) -> Self {
        Self::new(ErrorKind::BigIntForbidden { path: path.into() })
    }

    pub(crate) fn forbidden_name(path: impl Into<String>, name: &'static str) -> Self {
        Self::new(ErrorKind::ForbiddenName {
            path: path.into(),
            name,
        })
    }

    pub(crate) fn invalid_name(
        path: impl Into<String>,
        name: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self::new(ErrorKind::InvalidName {
            path: path.into(),
            name: name.into(),
        })
    }

    pub(crate) fn empty_name(path: impl Into<String>) -> Self {
        Self::new(ErrorKind::EmptyName { path: path.into() })
    }

    pub(crate) fn invalid_module_name(name: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidModuleName { name: name.into() })
    }

    pub(crate) fn missing_source_directory() -> Self {
        Self::new(ErrorKind::MissingSourceDirectory)
    }

    pub(crate) fn elm_json_not_found(path: impl Into<PathBuf>) -> Self {
        Self::new(ErrorKind::ElmJsonNotFound { path: path.into() })
    }

    pub(crate) fn read_file(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::new(ErrorKind::ReadFile {
            path: path.into(),
            source,
        })
    }

    pub(crate) fn invalid_elm_json(path: impl Into<PathBuf>, source: serde_json::Error) -> Self {
        Self::new(ErrorKind::InvalidElmJson {
            path: path.into(),
            source,
        })
    }

    pub(crate) fn create_dir(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::new(ErrorKind::CreateDir {
            path: path.into(),
            source,
        })
    }

    pub(crate) fn write_file(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Self::new(ErrorKind::WriteFile {
            path: path.into(),
            source,
        })
    }

    /// Construct an error for a type which has no Elm representation.
    pub(crate) fn unsupported(
        path: impl Into<String>,
        reason: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self::new(ErrorKind::Unsupported {
            path: path.into(),
            reason: reason.into(),
        })
    }

    pub(crate) fn dangling_named_reference(
        path: impl Into<String>,
        reference: impl Into<String>,
    ) -> Self {
        Self::new(ErrorKind::DanglingNamedReference {
            path: path.into(),
            reference: reference.into(),
        })
    }

    pub(crate) fn infinite_recursive_inline_type(
        path: impl Into<String>,
        reference: impl Into<String>,
        cycle: RecursiveInlineType,
    ) -> Self {
        Self::new(ErrorKind::InfiniteRecursiveInlineType {
            path: path.into(),
            reference: reference.into(),
            cycle,
        })
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ErrorKind::InvalidMapKey { path, reason } => {
                write!(f, "Invalid map key at {}: {reason}", display_path(path))
            }
            ErrorKind::BigIntForbidden { path } => write!(
                f,
                "Attempted to export {} but Elm's `Int` cannot represent every value of this integer type. Cast to `i32`/`u32`, or use a string to avoid precision loss.",
                display_path(path)
            ),
            ErrorKind::ForbiddenName { path, name } => write!(
                f,
                "Attempted to export {} but the name {name:?} conflicts with a reserved keyword in Elm. Try renaming it or using `#[specta(rename = \"new name\")]`.",
                display_path(path)
            ),
            ErrorKind::InvalidName { path, name } => write!(
                f,
                "Attempted to export {} but {name:?} is not a valid Elm identifier. Try renaming it or using `#[specta(rename = \"new name\")]`.",
                display_path(path)
            ),
            ErrorKind::EmptyName { path } => write!(
                f,
                "Attempted to export {} but the Elm type name is empty. Try renaming it or using `#[specta(rename = \"new name\")]`.",
                display_path(path)
            ),
            ErrorKind::InvalidModuleName { name } => write!(
                f,
                "The module name {name:?} is not a valid Elm module name. Module names must start with an uppercase letter and contain only letters and digits."
            ),
            ErrorKind::MissingSourceDirectory => write!(
                f,
                "The Elm project's `elm.json` has no `source-directories` entry, so generated files have nowhere to go."
            ),
            ErrorKind::ElmJsonNotFound { path } => write!(
                f,
                "Couldn't find an `elm.json` in '{}' or its immediate subdirectories.",
                path.display()
            ),
            ErrorKind::ReadFile { path, source } => {
                write!(f, "Failed to read '{}': {source}", path.display())
            }
            ErrorKind::InvalidElmJson { path, source } => {
                write!(f, "Failed to parse '{}': {source}", path.display())
            }
            ErrorKind::CreateDir { path, source } => {
                write!(
                    f,
                    "Failed to create directory '{}': {source}",
                    path.display()
                )
            }
            ErrorKind::WriteFile { path, source } => {
                write!(f, "Failed to write '{}': {source}", path.display())
            }
            ErrorKind::Unsupported { path, reason } => {
                write!(f, "Attempted to export {}: {reason}", display_path(path))
            }
            ErrorKind::DanglingNamedReference { path, reference } => write!(
                f,
                "Found a reference to {reference} at {} which is missing from the provided `Types`.",
                display_path(path)
            ),
            ErrorKind::InfiniteRecursiveInlineType {
                path,
                reference,
                cycle,
            } => write!(
                f,
                "Found infinitely recursive inline reference {reference} at {}. Elm cannot expand recursive inline types.\nInline cycle:\n  {cycle:?}",
                display_path(path)
            ),
        }?;

        if let Some(ndt) = self.named_datatype() {
            write!(
                f,
                "\nRust type: {}::{} at {}:{}:{}",
                ndt.module_path,
                ndt.name,
                ndt.location.file(),
                ndt.location.line(),
                ndt.location.column()
            )?;
        }

        Ok(())
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
            ErrorKind::ReadFile { source, .. }
            | ErrorKind::CreateDir { source, .. }
            | ErrorKind::WriteFile { source, .. } => Some(source),
            ErrorKind::InvalidElmJson { source, .. } => Some(source),
            _ => None,
        }
    }
}

fn display_path(path: &str) -> Cow<'_, str> {
    if path.is_empty() {
        Cow::Borrowed("<unknown path>")
    } else {
        Cow::Owned(format!("{path:?}"))
    }
}
