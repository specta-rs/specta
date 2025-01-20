use std::{error, fmt};

/// Detected a type which Serde is unable to export.
// TODO: The error should show a path to the type causing the issue like the BigInt error reporting.
#[derive(Debug, PartialEq)]
pub enum Error {
    // #[error("A map key must be a 'string' or 'number' type")]
    InvalidMapKey,
    // #[error("#[specta(tag = \"...\")] cannot be used with tuple variants")]
    InvalidInternallyTaggedEnum,
    // #[error("the usage of #[specta(skip)] means the type can't be serialized")]
    InvalidUsageOfSkip,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

impl error::Error for Error {}
