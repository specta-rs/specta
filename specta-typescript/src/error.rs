use std::{borrow::Cow, error, fmt, io, panic::Location};

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
        types: (Location<'static>, Location<'static>),
        name: Cow<'static, str>,
    },
    /// An filesystem IO error.
    /// This is possible when using `Typescript::export_to` when writing to a file or formatting the file.
    Io(io::Error),
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
    DuplicateTypeNameLegacy(Cow<'static, str>, Location<'static>, Location<'static>),
    // #[error("fmt error: {0}")]
    FmtLegacy(std::fmt::Error),
    UnableToExport,
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Self::Io(e)
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
            // TODO:
            Error::BigIntForbiddenLegacy(path) => writeln!(f, "Attempted to export {path:?} but Specta configuration forbids exporting BigInt types (i64, u64, i128, u128) because we don't know if your se/deserializer supports it. You can change this behavior by editing your `ExportConfiguration`!"),
            Error::ForbiddenNameLegacy(path, name, _) => writeln!(f, "Attempted to export {path:?} but was unable to due to name {name:?} conflicting with a reserved keyword in Typescript. Try renaming it or using `#[specta(rename = \"new name\")]`"),
            Error::InvalidNameLegacy(path, name, _) => writeln!(f, "Attempted to export {path:?} but was unable to due to name {name:?} containing an invalid character. Try renaming it or using `#[specta(rename = \"new name\")]`"),
            Error::InvalidTaggingLegacy(path) => writeln!(f, "Attempted to export {path:?} with tagging but the type is not tagged."),
            Error::InvalidTaggedVariantContainingTupleStructLegacy(path) => writeln!(f, "Attempted to export {path:?} with tagging but the variant is a tuple struct."),
            Error::DuplicateTypeNameLegacy(a, b, _) => writeln!(f, "Attempted to export {a:?} but was unable to due to name {b:?} conflicting with a reserved keyword in Typescript. Try renaming it or using `#[specta(rename = \"new name\")]`"),
            Error::FmtLegacy(err) => writeln!(f, "formatter: {err:?}"),
            Error::UnableToExport => writeln!(f, "Unable to export type"),
        }
    }
}

impl error::Error for Error {}
