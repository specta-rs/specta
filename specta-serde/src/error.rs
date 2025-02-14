use std::{error, fmt};

/// Detected a type which Serde is unable to export.
// TODO: The error should show a path to the type causing the issue like the BigInt error reporting.
#[derive(Debug, PartialEq)]
pub enum Error {
    InvalidMapKey,
    InvalidInternallyTaggedEnum,
    InvalidUsageOfSkip,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::InvalidMapKey => writeln!(f, "A map key must be a 'string' or 'number' type"),
            Error::InvalidInternallyTaggedEnum => writeln!(
                f,
                "#[specta(tag = \"...\")] cannot be used with tuple variants"
            ),
            Error::InvalidUsageOfSkip => writeln!(
                f,
                "the usage of #[specta(skip)] means the type can't be serialized"
            ),
        }
    }
}

impl error::Error for Error {}
