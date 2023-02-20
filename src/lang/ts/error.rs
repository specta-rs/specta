use thiserror::Error;

use crate::DataType;

#[derive(Error, Debug)]
#[allow(missing_docs)]
pub enum TsExportError {
    #[error("Failed to export type '{}' on field `{}`: {err}", .ty_name.unwrap_or_default(), .field_name.unwrap_or_default())]
    WithCtx {
        // TODO: Handle this better. Make `ty_name` non optional
        ty_name: Option<&'static str>,
        field_name: Option<&'static str>,
        err: Box<TsExportError>,
    },
    #[error("Your Specta configuration forbids exporting BigInt types (i64, u64, i128, u128) because we don't know if your se/deserializer supports it. You can change this behavior by editing your `ExportConfiguration`")]
    BigIntForbidden,
    #[error("Cannot export anonymous type. Try wrapping the type in a tuple struct which has the `ToDataType` derive macro on it.")]
    AnonymousType, // TODO: Include metadata about the type, `IMPL_LOCATION`, `NAME`, etc
    #[error("Unable to export a tagged type which is unnamed")]
    UnableToTagUnnamedType,
    #[error("You have defined a type with the name '{0}' which is a reserved name by the Typescript exporter. Try renaming it or using `#[specta(rename = \"new name\")]`")]
    ForbiddenTypeName(&'static str),
    #[error("You have defined a field '{1}' on type '{0}' which has a name that is reserved name by the Typescript exporter. Try renaming it or using `#[specta(rename = \"new name\")]`")]
    ForbiddenFieldName(String, &'static str),
    #[error("Type cannot be exported: {0:?}")]
    CannotExport(DataType), // TODO: probs just have `SID` and `IMPL_LOCATION` in this error?
    #[error("Cannot export type due to an internal error. This likely is a bug in Specta itself and not your code: {0}")]
    InternalError(&'static str),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(String),
}

impl PartialEq for TsExportError {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (
                Self::WithCtx {
                    ty_name: l_ty_name,
                    field_name: l_field_name,
                    err: l_err,
                },
                Self::WithCtx {
                    ty_name: r_ty_name,
                    field_name: r_field_name,
                    err: r_err,
                },
            ) => l_ty_name == r_ty_name && l_field_name == r_field_name && l_err == r_err,
            (Self::ForbiddenTypeName(l0), Self::ForbiddenTypeName(r0)) => l0 == r0,
            (Self::ForbiddenFieldName(l0, l1), Self::ForbiddenFieldName(r0, r1)) => {
                l0 == r0 && l1 == r1
            }
            (Self::CannotExport(l0), Self::CannotExport(r0)) => l0 == r0,
            (Self::InternalError(l0), Self::InternalError(r0)) => l0 == r0,
            (Self::Io(l0), Self::Io(r0)) => l0.to_string() == r0.to_string(), // This is a bit hacky but it will be fine for usage in unit tests!
            (Self::Other(l0), Self::Other(r0)) => l0 == r0,
            _ => core::mem::discriminant(self) == core::mem::discriminant(other),
        }
    }
}
