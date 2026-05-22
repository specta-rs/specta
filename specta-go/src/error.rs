use std::{error, fmt, io};

use crate::Layout;

#[derive(Debug)]
/// Errors that can occur during Go code generation.
pub enum Error {
    /// IO error during file operations.
    Io(io::Error),
    /// Formatting error while writing generated code.
    Fmt(fmt::Error),
    /// Custom format callback failed.
    Format {
        /// Context describing which format callback failed.
        message: &'static str,
        /// The underlying format error.
        source: specta::FormatError,
    },
    /// A generated Go identifier used a forbidden name.
    ForbiddenName {
        /// Path to the type or field containing the forbidden name.
        path: String,
        /// The forbidden Go identifier.
        name: String,
    },
    /// A BigInt value was encountered but cannot be represented in Go.
    BigIntForbidden {
        /// Path to the unsupported BigInt value.
        path: String,
    },
    /// The configured layout cannot be exported by this operation.
    UnableToExport(Layout),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "IO error: {e}"),
            Error::Fmt(e) => write!(f, "Fmt error: {e}"),
            Error::Format { message, source } => write!(f, "Format error: {message}: {source}"),
            Error::ForbiddenName { path, name } => {
                write!(f, "Forbidden name: {name} in {path}")
            }
            Error::BigIntForbidden { path } => {
                write!(f, "BigInt forbidden in {path}")
            }
            Error::UnableToExport(layout) => {
                write!(f, "Unable to export layout: {layout:?}")
            }
        }
    }
}

impl error::Error for Error {}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<fmt::Error> for Error {
    fn from(e: fmt::Error) -> Self {
        Self::Fmt(e)
    }
}

impl Error {
    pub(crate) fn format(message: &'static str, source: specta::FormatError) -> Self {
        Self::Format { message, source }
    }
}
