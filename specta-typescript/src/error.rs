use std::{borrow::Cow, error, fmt, io, panic::Location, path::PathBuf};

use specta::datatype::OpaqueReference;

use crate::{Layout, legacy::NamedLocation};

use super::legacy::ExportPath;

/// The error type for the TypeScript exporter.
#[derive(Debug)] // TODO: Should be be replaced with the `Display` impl???
#[non_exhaustive]
pub enum Error {
    /// Attempted to export a bigint type but the configuration forbids it.
    BigIntForbidden {
        /// Path to the item being exported.
        path: String,
    },
    /// Failed to validate a type is Serde compatible.
    Serde(specta_serde::Error),
    /// A type's name conflicts with a reserved keyword in Typescript.
    ForbiddenName {
        /// Path to the item being exported.
        path: String,
        /// The reserved keyword that caused the failure.
        name: &'static str,
    },
    /// A type's name contains invalid characters or is not valid.
    InvalidName {
        /// Path to the item being exported.
        path: String,
        /// The invalid name encountered during export.
        name: Cow<'static, str>,
    },
    /// Detected multiple items within the same scope with the same name.
    /// Typescript doesn't support this so we error out.
    ///
    /// Using anything other than [Layout::FlatFile] should make this basically impossible.
    DuplicateTypeName {
        // TODO: Flatten tuple into fields.
        /// The conflicting symbols (type/module/import) that share a name.
        types: (TypeOrModuleOrImport, TypeOrModuleOrImport),
        /// The duplicated name.
        name: Cow<'static, str>,
    },
    /// An filesystem IO error.
    /// This is possible when using `Typescript::export_to` when writing to a file or formatting the file.
    Io(io::Error),
    /// Failed to read a directory while exporting files.
    ReadDir {
        /// Directory path that failed to be read.
        path: PathBuf,
        /// The underlying IO error.
        source: io::Error,
    },
    /// Failed to inspect filesystem metadata while exporting files.
    Metadata {
        /// Path whose metadata lookup failed.
        path: PathBuf,
        /// The underlying IO error.
        source: io::Error,
    },
    /// Failed to remove a stale file while exporting files.
    RemoveFile {
        /// File path that failed to be removed.
        path: PathBuf,
        /// The underlying IO error.
        source: io::Error,
    },
    /// Failed to remove an empty directory while exporting files.
    RemoveDir {
        /// Directory path that failed to be removed.
        path: PathBuf,
        /// The underlying IO error.
        source: io::Error,
    },
    /// Found an opaque reference which the Typescript exporter doesn't know how to handle.
    /// You may be referencing a type which is not supported by the Typescript exporter.
    UnsupportedOpaqueReference(OpaqueReference),
    /// Found a named reference that cannot be resolved from the provided [`TypeCollection`](specta::TypeCollection).
    DanglingNamedReference {
        /// Debug identifier for the unresolved reference.
        reference: String,
    },
    /// An error occurred in your exporter framework.
    Framework(Cow<'static, str>),

    //
    //
    // TODO: Break
    //
    //
    // #[error("Attempted to export '{0}' but Specta configuration forbids exporting BigInt types (i64, u64, i128, u128) because we don't know if your se/deserializer supports it. You can change this behavior by editing your `ExportConfiguration`!")]
    /// Legacy bigint-export failure variant.
    BigIntForbiddenLegacy(ExportPath),
    // #[error("Attempted to export '{0}' but was unable to export a tagged type which is unnamed")]
    // UnableToTagUnnamedType(ExportPath),
    // #[error("Attempted to export '{1}' but was unable to due to {0} name '{2}' conflicting with a reserved keyword in Typescript. Try renaming it or using `#[specta(rename = \"new name\")]`")]
    /// Legacy reserved-name failure variant.
    ForbiddenNameLegacy(NamedLocation, ExportPath, &'static str),
    // #[error("Attempted to export '{1}' but was unable to due to {0} name '{2}' containing an invalid character")]
    /// Legacy invalid-name failure variant.
    InvalidNameLegacy(NamedLocation, ExportPath, String),
    // #[error("Attempted to export '{0}' with tagging but the type is not tagged.")]
    /// Legacy invalid-tagging failure variant.
    InvalidTaggingLegacy(ExportPath),
    // #[error("Attempted to export '{0}' with internal tagging but the variant is a tuple struct.")]
    /// Legacy internally tagged tuple-variant failure variant.
    InvalidTaggedVariantContainingTupleStructLegacy(ExportPath),
    // #[error("Unable to export type named '{0}' from locations")]
    // TODO: '{:?}' '{:?}'", .1.as_str(), .2.as_str())
    /// Legacy duplicate type name failure variant.
    DuplicateTypeNameLegacy(Cow<'static, str>, Location<'static>, Location<'static>),
    // #[error("fmt error: {0}")]
    /// Legacy formatter error.
    FmtLegacy(std::fmt::Error),
    /// Export layout is incompatible with the requested operation.
    UnableToExport(Layout),
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
            Error::BigIntForbidden { path } => writeln!(
                f,
                "Attempted to export {path:?} but Specta configuration forbids exporting BigInt types (i64, u64, i128, u128) because we don't know if your se/deserializer supports it. If your using a serializer/deserializer that natively has support for BigInt types you can disable this warning by editing your `ExportConfiguration`!"
            ),
            Error::Serde(err) => write!(f, "Detect invalid Serde type: {err}"),
            Error::ForbiddenName { path, name } => writeln!(
                f,
                "Attempted to export {path:?} but was unable to due toname {name:?} conflicting with a reserved keyword in Typescript. Try renaming it or using `#[specta(rename = \"new name\")]`"
            ),
            Error::InvalidName { path, name } => writeln!(
                f,
                "Attempted to export {path:?} but was unable to due to name {name:?} containing an invalid character. Try renaming it or using `#[specta(rename = \"new name\")]`"
            ),
            Error::DuplicateTypeName { types, name } => writeln!(
                f,
                "Detected multiple types with the same name: {name:?} in {types:?}"
            ),
            Error::Io(err) => write!(f, "IO error: {err}"),
            Error::ReadDir { path, source } => {
                write!(f, "Failed to read directory '{}': {source}", path.display())
            }
            Error::Metadata { path, source } => {
                write!(
                    f,
                    "Failed to read metadata for '{}': {source}",
                    path.display()
                )
            }
            Error::RemoveFile { path, source } => {
                write!(f, "Failed to remove file '{}': {source}", path.display())
            }
            Error::RemoveDir { path, source } => {
                write!(
                    f,
                    "Failed to remove directory '{}': {source}",
                    path.display()
                )
            }
            Error::UnsupportedOpaqueReference(r) => {
                write!(
                    f,
                    "Found unsupported opaque reference '{}'. It is not supported by the Typescript exporter.",
                    r.type_name()
                )
            }
            Error::DanglingNamedReference { reference } => {
                write!(
                    f,
                    "Found dangling named reference {reference}. The referenced type is missing from `TypeCollection`."
                )
            }
            Error::Framework(e) => {
                write!(f, "Framework error: {e}")
            }
            // TODO:
            Error::BigIntForbiddenLegacy(path) => writeln!(
                f,
                "Attempted to export {path:?} but Specta configuration forbids exporting BigInt types (i64, u64, i128, u128) because we don't know if your se/deserializer supports it. You can change this behavior by editing your `ExportConfiguration`!"
            ),
            Error::ForbiddenNameLegacy(path, name, _) => writeln!(
                f,
                "Attempted to export {path:?} but was unable to due to name {name:?} conflicting with a reserved keyword in Typescript. Try renaming it or using `#[specta(rename = \"new name\")]`"
            ),
            Error::InvalidNameLegacy(path, name, _) => writeln!(
                f,
                "Attempted to export {path:?} but was unable to due to name {name:?} containing an invalid character. Try renaming it or using `#[specta(rename = \"new name\")]`"
            ),
            Error::InvalidTaggingLegacy(path) => writeln!(
                f,
                "Attempted to export {path:?} with tagging but the type is not tagged."
            ),
            Error::InvalidTaggedVariantContainingTupleStructLegacy(path) => writeln!(
                f,
                "Attempted to export {path:?} with tagging but the variant is a tuple struct."
            ),
            Error::DuplicateTypeNameLegacy(a, b, _) => writeln!(
                f,
                "Attempted to export {a:?} but was unable to due to name {b:?} conflicting with a reserved keyword in Typescript. Try renaming it or using `#[specta(rename = \"new name\")]`"
            ),
            Error::FmtLegacy(err) => writeln!(f, "formatter: {err:?}"),
            Error::UnableToExport(layout) => writeln!(
                f,
                "Unable to export layout {layout} with the current configuration. Maybe try `Exporter::export_to` or switching to Typescript."
            ),
        }
    }
}

impl error::Error for Error {}

/// A source location used when describing duplicate names.
#[derive(Debug)]
#[non_exhaustive]
pub enum TypeOrModuleOrImport {
    /// A Rust type declaration location.
    Type(Location<'static>),
    /// A generated module path.
    Module(Cow<'static, str>),
    /// A generated import path.
    Import(Cow<'static, str>),
}

impl From<Location<'static>> for TypeOrModuleOrImport {
    fn from(location: Location<'static>) -> Self {
        TypeOrModuleOrImport::Type(location)
    }
}
