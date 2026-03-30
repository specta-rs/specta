use std::{error, fmt, io};

use crate::Layout;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Fmt(fmt::Error),
    Serde(specta_serde::Error),
    ForbiddenName { path: String, name: String },
    BigIntForbidden { path: String },
    UnableToExport(Layout),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "IO error: {e}"),
            Error::Fmt(e) => write!(f, "Fmt error: {e}"),
            Error::Serde(e) => write!(f, "Serde error: {e}"),
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

impl From<specta_serde::Error> for Error {
    fn from(e: specta_serde::Error) -> Self {
        Self::Serde(e)
    }
}
