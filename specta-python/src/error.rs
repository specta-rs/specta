use std::{borrow::Cow, error, fmt, io, path::PathBuf};

use specta::datatype::{NamedDataType, OpaqueReference, RecursiveInlineType};

use crate::Layout;

/// A frame describing where an exporter error occurred.
#[derive(Debug, Clone)]
pub enum ErrorTraceFrame {
    /// The failure happened while expanding an inline named type.
    Inlined {
        /// The inlined type, when it can be resolved.
        named_datatype: Option<Box<NamedDataType>>,
        /// Dot-separated path at which it was inlined.
        path: String,
    },
}

/// Errors produced while exporting Python type hints.
#[derive(Debug)]
pub struct Error {
    kind: ErrorKind,
    named_datatype: Option<Box<NamedDataType>>,
    trace: Vec<ErrorTraceFrame>,
}

#[derive(Debug)]
enum ErrorKind {
    Io {
        action: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    Format {
        message: &'static str,
        path: Option<String>,
        source: specta::FormatError,
    },
    InvalidName {
        path: String,
        name: String,
    },
    ForbiddenName {
        path: String,
        name: &'static str,
    },
    DuplicateName {
        name: String,
        first: String,
        second: String,
    },
    UnsupportedOpaqueReference {
        path: String,
        reference: OpaqueReference,
    },
    DanglingReference {
        path: String,
        reference: String,
    },
    RecursiveInline {
        path: String,
        cycle: RecursiveInlineType,
    },
    UnsupportedLayout {
        layout: Layout,
        operation: &'static str,
    },
    DuplicateFile {
        path: PathBuf,
        first: String,
        second: String,
    },
    SymlinkEscape {
        path: PathBuf,
    },
    UnrepresentableIntersection {
        path: String,
    },
}

impl Error {
    fn new(kind: ErrorKind) -> Self {
        Self {
            kind,
            named_datatype: None,
            trace: Vec::new(),
        }
    }

    pub(crate) fn create_dir(path: PathBuf, source: io::Error) -> Self {
        Self::new(ErrorKind::Io {
            action: "create directory",
            path,
            source,
        })
    }

    pub(crate) fn write_file(path: PathBuf, source: io::Error) -> Self {
        Self::new(ErrorKind::Io {
            action: "write file",
            path,
            source,
        })
    }

    pub(crate) fn format(message: &'static str, source: specta::FormatError) -> Self {
        Self::new(ErrorKind::Format {
            message,
            path: None,
            source,
        })
    }

    pub(crate) fn format_at(
        message: &'static str,
        path: String,
        source: specta::FormatError,
    ) -> Self {
        Self::new(ErrorKind::Format {
            message,
            path: Some(path),
            source,
        })
    }

    pub(crate) fn invalid_name(path: String, name: impl Into<String>) -> Self {
        Self::new(ErrorKind::InvalidName {
            path,
            name: name.into(),
        })
    }

    pub(crate) fn forbidden_name(path: String, name: &'static str) -> Self {
        Self::new(ErrorKind::ForbiddenName { path, name })
    }

    pub(crate) fn duplicate_name(
        name: impl Into<String>,
        first: impl Into<String>,
        second: impl Into<String>,
    ) -> Self {
        Self::new(ErrorKind::DuplicateName {
            name: name.into(),
            first: first.into(),
            second: second.into(),
        })
    }

    pub(crate) fn unsupported_opaque_reference(path: String, reference: OpaqueReference) -> Self {
        Self::new(ErrorKind::UnsupportedOpaqueReference { path, reference })
    }

    pub(crate) fn dangling_reference(path: String, reference: String) -> Self {
        Self::new(ErrorKind::DanglingReference { path, reference })
    }

    pub(crate) fn recursive_inline(path: String, cycle: RecursiveInlineType) -> Self {
        Self::new(ErrorKind::RecursiveInline { path, cycle })
    }

    pub(crate) fn unsupported_layout(layout: Layout, operation: &'static str) -> Self {
        Self::new(ErrorKind::UnsupportedLayout { layout, operation })
    }

    pub(crate) fn duplicate_file(
        path: PathBuf,
        first: impl Into<String>,
        second: impl Into<String>,
    ) -> Self {
        Self::new(ErrorKind::DuplicateFile {
            path,
            first: first.into(),
            second: second.into(),
        })
    }

    pub(crate) fn symlink_escape(path: PathBuf) -> Self {
        Self::new(ErrorKind::SymlinkEscape { path })
    }

    pub(crate) fn unrepresentable_intersection(path: String) -> Self {
        Self::new(ErrorKind::UnrepresentableIntersection { path })
    }

    /// The named Rust type being exported when this error occurred, if known.
    pub fn named_datatype(&self) -> Option<&NamedDataType> {
        self.named_datatype.as_deref()
    }

    /// Exporter traversal context for this error.
    pub fn trace(&self) -> &[ErrorTraceFrame] {
        &self.trace
    }

    pub(crate) fn with_named_datatype(mut self, ndt: &NamedDataType) -> Self {
        self.named_datatype
            .get_or_insert_with(|| Box::new(ndt.clone()));
        self
    }

    pub(crate) fn with_inline_trace(
        mut self,
        ndt: Option<&NamedDataType>,
        path: impl Into<String>,
    ) -> Self {
        self.trace.push(ErrorTraceFrame::Inlined {
            named_datatype: ndt.map(|ndt| Box::new(ndt.clone())),
            path: path.into(),
        });
        self
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ErrorKind::Io {
                action,
                path,
                source,
            } => {
                write!(f, "failed to {action} '{}': {source}", path.display())
            }
            ErrorKind::Format {
                message,
                path,
                source,
            } => {
                write!(f, "{message}")?;
                if let Some(path) = path {
                    write!(f, " at {path}")?;
                }
                write!(f, ": {source}")
            }
            ErrorKind::InvalidName { path, name } => {
                write!(f, "'{name}' at {path} is not a valid Python identifier")
            }
            ErrorKind::ForbiddenName { path, name } => {
                write!(f, "'{name}' at {path} is a reserved Python name")
            }
            ErrorKind::DuplicateName {
                name,
                first,
                second,
            } => write!(
                f,
                "duplicate exported Python name '{name}' for Rust types '{first}' and '{second}'"
            ),
            ErrorKind::UnsupportedOpaqueReference { path, reference } => write!(
                f,
                "unsupported opaque reference '{}' at {path}",
                reference.type_name()
            ),
            ErrorKind::DanglingReference { path, reference } => {
                write!(f, "dangling named reference {reference} at {path}")
            }
            ErrorKind::RecursiveInline { path, cycle } => {
                write!(f, "infinitely recursive inline type at {path}: {cycle:?}")
            }
            ErrorKind::UnsupportedLayout { layout, operation } => {
                write!(f, "layout {layout} cannot be used with {operation}")
            }
            ErrorKind::DuplicateFile {
                path,
                first,
                second,
            } => write!(
                f,
                "types '{first}' and '{second}' map to the same Python file '{}'",
                path.display()
            ),
            ErrorKind::SymlinkEscape { path } => write!(
                f,
                "refusing to traverse symlink '{}' in Python export directory",
                path.display()
            ),
            ErrorKind::UnrepresentableIntersection { path } => write!(
                f,
                "intersection at {path} cannot be represented as a Python type hint"
            ),
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match &self.kind {
            ErrorKind::Io { source, .. } => Some(source),
            ErrorKind::Format { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(source: io::Error) -> Self {
        Self::write_file(PathBuf::new(), source)
    }
}

pub(crate) fn display_path(path: &[Cow<'static, str>]) -> String {
    if path.is_empty() {
        "<root>".to_string()
    } else {
        path.join(".")
    }
}
