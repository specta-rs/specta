use core::fmt;
use std::{borrow::Cow, error, io};

use specta::ImplLocation;

/// The error type for the TypeScript exporter.
#[derive(Debug, PartialEq)]
#[non_exhaustive]
pub enum Error {
    /// Attempted to export a bigint type but the configuration forbids it.
    BigIntForbidden {
        path: String,
    },
    /// Failed to validate a type is Serde compatible.
    Serde(specta_serde::Error),
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
    /// Detected multiple types with the same name.
    DuplicateTypeName {
        types: (ImplLocation, ImplLocation),
        name: Cow<'static, str>,
    },
    /// An filesystem IO error.
    /// This is possible when using `Typescript::export_to` when writing to a file or formatting the file.
    // We cast `std::io::Error` `String` so we can have `PartialEq`
    Io(String)
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e.to_string())
    }
}

impl From<specta_serde::Error> for Error {
    fn from(e: specta_serde::Error) -> Self {
        Self::Serde(e)
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::BigIntForbidden { path } => writeln!(f, "Attempted to export {path:?} but Specta configuration forbids exporting BigInt types (i64, u64, i128, u128) because we don't know if your se/deserializer supports it. You can change this behavior by editing your `ExportConfiguration`!"),
            Error::Serde(err) => write!(f, "Detect invalid Serde type: {err}"),
            Error::ForbiddenName { path, name } => writeln!(f, "Attempted to export {path:?} but was unable to due toname {name:?} conflicting with a reserved keyword in Typescript. Try renaming it or using `#[specta(rename = \"new name\")]`"),
            Error::InvalidName { path, name } => writeln!(f, "Attempted to export {path:?} but was unable to due to name {name:?} containing an invalid character. Try renaming it or using `#[specta(rename = \"new name\")]`"),
            Error::DuplicateTypeName { types, name } => writeln!(f, "Detected multiple types with the same name: {name:?} in {types:?}"),
            Error::Io(err) => write!(f, "IO error: {err}"),
        }
    }
}

impl error::Error for Error {}

// // #[error("Attempted to export '{0}' but was unable to export a tagged type which is unnamed")]
// // UnableToTagUnnamedType(ExportPath),
// #[error("Attempted to export '{1}' but was unable to due to {0} name '{2}' conflicting with a reserved keyword in Typescript. Try renaming it or using `#[specta(rename = \"new name\")]`")]
// #[error("Attempted to export '{1}' but was unable to due to {0} name '{2}' containing an invalid character")]
// InvalidName(NamedLocation, ExportPath, String),
// #[error("Attempted to export '{0}' with tagging but the type is not tagged.")]
// InvalidTagging(ExportPath),
// #[error("Attempted to export '{0}' with internal tagging but the variant is a tuple struct.")]
// InvalidTaggedVariantContainingTupleStruct(ExportPath),
// #[error("Unable to export type named '{0}' from locations '{:?}' '{:?}'", .1.as_str(), .2.as_str())]
// DuplicateTypeName(Cow<'static, str>, ImplLocation, ImplLocation),
// #[error("IO error: {0}")]
// Io(#[from] std::io::Error),
// #[error("fmt error: {0}")]
// Fmt(#[from] std::fmt::Error),
// #[error("Failed to export '{0}' due to error: {1}")]
// Other(ExportPath, String),
