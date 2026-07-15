use std::{borrow::Cow, error, fmt, io, path::PathBuf};

use specta::datatype::{NamedDataType, OpaqueReference, RecursiveInlineType};

use crate::Layout;

/// An error produced while generating Java source.
#[derive(Debug)]
#[non_exhaustive]
pub struct Error {
    kind: ErrorKind,
    named_datatype: Option<Box<NamedDataType>>,
    trace: Vec<ErrorTraceFrame>,
}

/// Additional traversal context for an [`Error`].
#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum ErrorTraceFrame {
    /// The exporter was rendering a field, variant, or nested datatype.
    Path(String),
}

#[derive(Debug)]
enum ErrorKind {
    InvalidIdentifier {
        path: String,
        name: String,
    },
    UnsupportedType {
        path: String,
        reason: Cow<'static, str>,
    },
    UnsupportedOpaqueReference {
        path: String,
        reference: OpaqueReference,
    },
    DanglingNamedReference {
        path: String,
        reference: String,
    },
    InfiniteRecursiveInlineType {
        path: String,
        reference: String,
        cycle: RecursiveInlineType,
    },
    DuplicateTypeName {
        name: String,
        first: String,
        second: String,
    },
    Format {
        message: Cow<'static, str>,
        source: specta::FormatError,
    },
    ExportRequiresExportTo(Layout),
    OutputFilenameMismatch {
        expected: String,
        actual: PathBuf,
    },
    CreateDir {
        path: PathBuf,
        source: io::Error,
    },
    WriteFile {
        path: PathBuf,
        source: io::Error,
    },
    ReadDir {
        path: PathBuf,
        source: io::Error,
    },
    ReadFile {
        path: PathBuf,
        source: io::Error,
    },
    RemoveFile {
        path: PathBuf,
        source: io::Error,
    },
}

impl Error {
    pub(crate) fn invalid_identifier(path: impl Into<String>, name: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidIdentifier {
            path: path.into(),
            name: name.into(),
        })
    }

    pub(crate) fn unsupported(
        path: impl Into<String>,
        reason: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self::new(ErrorKind::UnsupportedType {
            path: path.into(),
            reason: reason.into(),
        })
    }

    pub(crate) fn unsupported_opaque(path: impl Into<String>, reference: OpaqueReference) -> Self {
        Self::new(ErrorKind::UnsupportedOpaqueReference {
            path: path.into(),
            reference,
        })
    }

    pub(crate) fn dangling_reference(
        path: impl Into<String>,
        reference: impl Into<String>,
    ) -> Self {
        Self::new(ErrorKind::DanglingNamedReference {
            path: path.into(),
            reference: reference.into(),
        })
    }

    pub(crate) fn recursive_inline(
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

    pub(crate) fn duplicate_type(
        name: impl Into<String>,
        first: impl Into<String>,
        second: impl Into<String>,
    ) -> Self {
        Self::new(ErrorKind::DuplicateTypeName {
            name: name.into(),
            first: first.into(),
            second: second.into(),
        })
    }

    pub(crate) fn format(
        message: impl Into<Cow<'static, str>>,
        source: specta::FormatError,
    ) -> Self {
        Self::new(ErrorKind::Format {
            message: message.into(),
            source,
        })
    }

    pub(crate) fn export_requires_export_to(layout: Layout) -> Self {
        Self::new(ErrorKind::ExportRequiresExportTo(layout))
    }

    pub(crate) fn output_filename_mismatch(expected: impl Into<String>, actual: PathBuf) -> Self {
        Self::new(ErrorKind::OutputFilenameMismatch {
            expected: expected.into(),
            actual,
        })
    }

    pub(crate) fn create_dir(path: PathBuf, source: io::Error) -> Self {
        Self::new(ErrorKind::CreateDir { path, source })
    }

    pub(crate) fn write_file(path: PathBuf, source: io::Error) -> Self {
        Self::new(ErrorKind::WriteFile { path, source })
    }

    pub(crate) fn read_dir(path: PathBuf, source: io::Error) -> Self {
        Self::new(ErrorKind::ReadDir { path, source })
    }

    pub(crate) fn read_file(path: PathBuf, source: io::Error) -> Self {
        Self::new(ErrorKind::ReadFile { path, source })
    }

    pub(crate) fn remove_file(path: PathBuf, source: io::Error) -> Self {
        Self::new(ErrorKind::RemoveFile { path, source })
    }

    fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            named_datatype: None,
            trace: Vec::new(),
        }
    }

    pub(crate) fn with_named_datatype(mut self, ndt: &NamedDataType) -> Self {
        self.named_datatype
            .get_or_insert_with(|| Box::new(ndt.clone()));
        self
    }

    pub(crate) fn with_path(mut self, path: impl Into<String>) -> Self {
        self.trace.push(ErrorTraceFrame::Path(path.into()));
        self
    }

    /// The named Rust type being exported when this error occurred, if known.
    pub fn named_datatype(&self) -> Option<&NamedDataType> {
        self.named_datatype.as_deref()
    }

    /// Exporter traversal context, from the innermost frame outwards.
    pub fn trace(&self) -> &[ErrorTraceFrame] {
        &self.trace
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ErrorKind::InvalidIdentifier { path, name } => {
                write!(
                    f,
                    "'{name}' at {path} cannot be represented as a Java identifier"
                )
            }
            ErrorKind::UnsupportedType { path, reason } => {
                write!(f, "unsupported datatype at {path}: {reason}")
            }
            ErrorKind::UnsupportedOpaqueReference { path, reference } => {
                write!(f, "unsupported opaque reference '{reference:?}' at {path}")
            }
            ErrorKind::DanglingNamedReference { path, reference } => {
                write!(
                    f,
                    "named reference '{reference}' at {path} is not registered"
                )
            }
            ErrorKind::InfiniteRecursiveInlineType {
                path,
                reference,
                cycle,
            } => {
                write!(
                    f,
                    "recursive inline reference '{reference}' at {path}: {cycle:?}"
                )
            }
            ErrorKind::DuplicateTypeName {
                name,
                first,
                second,
            } => write!(
                f,
                "duplicate Java type name '{name}' generated for '{first}' and '{second}'"
            ),
            ErrorKind::Format { message, source } => write!(f, "{message}: {source}"),
            ErrorKind::ExportRequiresExportTo(layout) => {
                write!(f, "layout {layout} requires Java::export_to")
            }
            ErrorKind::OutputFilenameMismatch { expected, actual } => write!(
                f,
                "flat-file output '{}' must be named '{expected}.java' to match its public class",
                actual.display()
            ),
            ErrorKind::CreateDir { path, source } => {
                write!(
                    f,
                    "failed to create directory '{}': {source}",
                    path.display()
                )
            }
            ErrorKind::WriteFile { path, source } => {
                write!(
                    f,
                    "failed to write Java file '{}': {source}",
                    path.display()
                )
            }
            ErrorKind::ReadDir { path, source } => {
                write!(f, "failed to read directory '{}': {source}", path.display())
            }
            ErrorKind::ReadFile { path, source } => {
                write!(f, "failed to read Java file '{}': {source}", path.display())
            }
            ErrorKind::RemoveFile { path, source } => {
                write!(
                    f,
                    "failed to remove stale Java file '{}': {source}",
                    path.display()
                )
            }
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match &self.kind {
            ErrorKind::Format { source, .. } => Some(source.as_ref()),
            ErrorKind::CreateDir { source, .. }
            | ErrorKind::WriteFile { source, .. }
            | ErrorKind::ReadDir { source, .. }
            | ErrorKind::ReadFile { source, .. }
            | ErrorKind::RemoveFile { source, .. } => Some(source),
            _ => None,
        }
    }
}
