use std::{borrow::Cow, error};

use crate::{Types, datatype::DataType};

/// Error type returned by [`Format`] callbacks.
pub type FormatError = Box<dyn error::Error + Send + Sync + 'static>;

/// The format is used to inform Specta how the Serialize/Deserialization layer handles types.
///
/// This allows them to rewrite the collected types and encountered datatypes to apply format-specific macro attributes or behaviour.
///
/// Currently we have support for:
///  - [serde](https://docs.rs/specta) via [`specta-serde`](https://docs.rs/specta-serde)
///
pub trait Format {
    /// Apply a map function to the full [`Types`] collection.
    ///
    /// Returns [`Cow::Borrowed`] when no changes are needed, or
    /// [`Cow::Owned`] when the formatter produces a transformed collection.
    fn map_types(&'_ self, types: &Types) -> std::result::Result<Cow<'_, Types>, FormatError>;

    /// Map an individual [`DataType`] with access to the surrounding [`Types`].
    ///
    /// Returns [`Cow::Borrowed`] when no changes are needed, or
    /// [`Cow::Owned`] when the formatter produces a transformed datatype.
    fn map_type(
        &'_ self,
        types: &Types,
        ty: &DataType,
    ) -> std::result::Result<Cow<'_, DataType>, FormatError>;
}

impl<T: Format + ?Sized> Format for &T {
    fn map_types(&'_ self, types: &Types) -> std::result::Result<Cow<'_, Types>, FormatError> {
        (**self).map_types(types)
    }

    fn map_type(
        &'_ self,
        types: &Types,
        ty: &DataType,
    ) -> std::result::Result<Cow<'_, DataType>, FormatError> {
        (**self).map_type(types, ty)
    }
}

impl<T: Format + ?Sized> Format for Box<T> {
    fn map_types(&'_ self, types: &Types) -> std::result::Result<Cow<'_, Types>, FormatError> {
        (**self).map_types(types)
    }

    fn map_type(
        &'_ self,
        types: &Types,
        ty: &DataType,
    ) -> std::result::Result<Cow<'_, DataType>, FormatError> {
        (**self).map_type(types, ty)
    }
}

// Assert dyn-safety
const _: Option<&dyn Format> = None;
