use core::fmt;

use thiserror::Error;

use crate::ExportError;

use super::ExportPath;

/// Describe where the error occurred
#[derive(Error, Debug, PartialEq)]
#[allow(missing_docs)]
pub enum NamedLocation {
    Type,
    Field,
    Variant,
}

impl fmt::Display for NamedLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Type => write!(f, "type"),
            Self::Field => write!(f, "field"),
            Self::Variant => write!(f, "variant"),
        }
    }
}

#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum TsExportError {
    #[error("Attempted to export '{0}' but Specta configuration forbids exporting BigInt types (i64, u64, i128, u128) because we don't know if your se/deserializer supports it. You can change this behavior by editing your `ExportConfiguration`!")]
    BigIntForbidden(ExportPath),
    #[error("Attempted to export '{0}' but was unable to export a tagged type which is unnamed")]
    UnableToTagUnnamedType(ExportPath),
    #[error("Attempted to export '{1}' but was unable to due to {0} name '{2}' conflicting with a reserved keyword in Typescript. Try renaming it or using `#[specta(rename = \"new name\")]`")]
    ForbiddenName(NamedLocation, ExportPath, &'static str),
    #[error("Attempted to export '{0}' with tagging but the type is not tagged.")]
    InvalidTagging(ExportPath),
    #[error("Unable to export '{0}'")]
    CannotExport(ExportPath),
    #[error("Unable to export '{0}' due to an internal error. This likely is a bug in Specta itself and not your code: {0}")]
    InternalError(ExportPath, &'static str),
    #[error("Generic export error: {0}")]
    SpectaExportError(#[from] ExportError),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Failed to export '{0}' due to error: {1}")]
    Other(ExportPath, String),
}

impl PartialEq for TsExportError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::BigIntForbidden(l0), Self::BigIntForbidden(r0)) => l0 == r0,
            (Self::UnableToTagUnnamedType(l0), Self::UnableToTagUnnamedType(r0)) => l0 == r0,
            (Self::ForbiddenName(l0, l1, l2), Self::ForbiddenName(r0, r1, r2)) => {
                l0 == r0 && l1 == r1 && l2 == r2
            }
            (Self::InvalidTagging(l0), Self::InvalidTagging(r0)) => l0 == r0,
            (Self::CannotExport(l0), Self::CannotExport(r0)) => l0 == r0,
            (Self::InternalError(l0, l1), Self::InternalError(r0, r1)) => l0 == r0 && l1 == r1,
            (Self::Io(l0), Self::Io(r0)) => l0.to_string() == r0.to_string(), // This is a bit hacky but it will be fine for usage in unit tests!
            (Self::Other(l0, l1), Self::Other(r0, r1)) => l0 == r0 && l1 == r1,
            _ => false,
        }
    }
}
