use std::{borrow::Cow, error, fmt, io};

use specta::ImplLocation;

use crate::legacy::NamedLocation;

use super::legacy::ExportPath;

/// The error type for the TypeScript exporter.
#[derive(Debug)]
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
    Io(String),
    //
    //
    // TODO: Break
    //
    //
    // #[error("Attempted to export '{0}' but Specta configuration forbids exporting BigInt types (i64, u64, i128, u128) because we don't know if your se/deserializer supports it. You can change this behavior by editing your `ExportConfiguration`!")]
    BigIntForbiddenLegacy(ExportPath),
    // #[error("Attempted to export '{0}' but was unable to export a tagged type which is unnamed")]
    // UnableToTagUnnamedType(ExportPath),
    // #[error("Attempted to export '{1}' but was unable to due to {0} name '{2}' conflicting with a reserved keyword in Typescript. Try renaming it or using `#[specta(rename = \"new name\")]`")]
    ForbiddenNameLegacy(NamedLocation, ExportPath, &'static str),
    // #[error("Attempted to export '{1}' but was unable to due to {0} name '{2}' containing an invalid character")]
    InvalidNameLegacy(NamedLocation, ExportPath, String),
    // #[error("Attempted to export '{0}' with tagging but the type is not tagged.")]
    InvalidTaggingLegacy(ExportPath),
    // #[error("Attempted to export '{0}' with internal tagging but the variant is a tuple struct.")]
    InvalidTaggedVariantContainingTupleStructLegacy(ExportPath),
    // #[error("Unable to export type named '{0}' from locations")]
    // TODO: '{:?}' '{:?}'", .1.as_str(), .2.as_str())
    DuplicateTypeNameLegacy(Cow<'static, str>, ImplLocation, ImplLocation),
    // #[error("IO error: {0}")]
    // Io(#[from] std::io::Error),
    // #[error("fmt error: {0}")]
    FmtLegacy(std::fmt::Error),
    // #[error("Failed to export '{0}' due to error: {1}")]
    // Other(ExportPath, String),
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

impl From<std::fmt::Error> for Error {
    fn from(e: std::fmt::Error) -> Self {
        Self::FmtLegacy(e)
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
            _ => todo!(),
        }
    }
}

impl error::Error for Error {}

// TODO: This `impl` is cringe
impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        true // TODO: Fix this
    }
}
