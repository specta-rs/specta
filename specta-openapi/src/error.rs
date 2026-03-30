use std::{error, fmt, io, path::PathBuf};

/// The error type for the OpenAPI exporter.
#[derive(Debug)]
#[non_exhaustive]
pub struct Error {
    kind: ErrorKind,
}

#[derive(Debug)]
enum ErrorKind {
    Io(io::Error),
    SerdeJson(serde_json::Error),
    UnsupportedOpenApiVersionForDynamicRef(String),
    DanglingNamedReference(String),
    UnresolvedGenericReference(String),
    UnsupportedOpaqueReference(String),
    InvalidMapKey(String),
    UnableToExportPath(PathBuf, io::Error),
}

impl Error {
    pub(crate) fn unsupported_openapi_version_for_dynamic_ref(version: String) -> Self {
        Self {
            kind: ErrorKind::UnsupportedOpenApiVersionForDynamicRef(version),
        }
    }

    pub(crate) fn dangling_named_reference(reference: String) -> Self {
        Self {
            kind: ErrorKind::DanglingNamedReference(reference),
        }
    }

    pub(crate) fn unresolved_generic_reference(reference: String) -> Self {
        Self {
            kind: ErrorKind::UnresolvedGenericReference(reference),
        }
    }

    pub(crate) fn unsupported_opaque_reference(reference: String) -> Self {
        Self {
            kind: ErrorKind::UnsupportedOpaqueReference(reference),
        }
    }

    pub(crate) fn invalid_map_key(reason: String) -> Self {
        Self {
            kind: ErrorKind::InvalidMapKey(reason),
        }
    }

    pub(crate) fn unable_to_export_path(path: PathBuf, source: io::Error) -> Self {
        Self {
            kind: ErrorKind::UnableToExportPath(path, source),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ErrorKind::Io(source) => write!(f, "{source}"),
            ErrorKind::SerdeJson(source) => write!(f, "{source}"),
            ErrorKind::UnsupportedOpenApiVersionForDynamicRef(version) => write!(
                f,
                "`$dynamicRef` generic handling requires OpenAPI 3.1+, but exporter is configured for {version}"
            ),
            ErrorKind::DanglingNamedReference(reference) => {
                write!(f, "Dangling named reference encountered: {reference}")
            }
            ErrorKind::UnresolvedGenericReference(reference) => {
                write!(f, "Unresolved generic reference encountered: {reference}")
            }
            ErrorKind::UnsupportedOpaqueReference(reference) => {
                write!(f, "OpenAPI exporter does not support opaque reference `{reference}`")
            }
            ErrorKind::InvalidMapKey(reason) => write!(f, "Invalid OpenAPI map key: {reason}"),
            ErrorKind::UnableToExportPath(path, source) => {
                write!(f, "Failed to export OpenAPI document to '{}': {source}", path.display())
            }
        }
    }
}

impl error::Error for Error {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match &self.kind {
            ErrorKind::Io(source) => Some(source),
            ErrorKind::SerdeJson(source) => Some(source),
            ErrorKind::UnableToExportPath(_, source) => Some(source),
            ErrorKind::UnsupportedOpenApiVersionForDynamicRef(_)
            | ErrorKind::DanglingNamedReference(_)
            | ErrorKind::UnresolvedGenericReference(_)
            | ErrorKind::UnsupportedOpaqueReference(_)
            | ErrorKind::InvalidMapKey(_) => None,
        }
    }
}

impl From<io::Error> for Error {
    fn from(value: io::Error) -> Self {
        Self {
            kind: ErrorKind::Io(value),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(value: serde_json::Error) -> Self {
        Self {
            kind: ErrorKind::SerdeJson(value),
        }
    }
}
