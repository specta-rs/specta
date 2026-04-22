use std::{borrow::Cow, error};

use crate::{Types, datatype::DataType};

/// Error type returned by [Format] callbacks.
pub type FormatError = Box<dyn error::Error + Send + Sync + 'static>;

/// Internal formatter callbacks for rewriting collected types and nested datatypes.
///
/// This is used by [specta-serde] and other serialize/deserialization formats to apply their attributes.
/// This can also be used by frameworks to do special transformations to datatypes (like for BigInt support).
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
