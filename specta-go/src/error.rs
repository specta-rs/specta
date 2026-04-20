use std::{error, fmt, io};

use crate::Layout;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Fmt(fmt::Error),
    Format {
        message: &'static str,
        source: specta::FormatError,
    },
    ForbiddenName {
        path: String,
        name: String,
    },
    BigIntForbidden {
        path: String,
    },
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
