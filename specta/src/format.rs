use std::{borrow::Cow, error};

use crate::{Types, datatype::DataType};

/// Error type returned by [Format] callbacks.
pub type FormatError = Box<dyn error::Error + Send + Sync + 'static>;

/// Internal formatter callbacks for rewriting collected types and nested datatypes.
///
/// This is used by [specta-serde] and other serialize/deserialization formats to apply their attributes.
/// This can also be used by frameworks to do special transformations to datatypes (like for BigInt support).
///
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct Format {
    /// Formats the full [`Types`] collection.
    ///
    /// Returns [`Cow::Borrowed`] when no changes are needed, or
    /// [`Cow::Owned`] when the formatter produces a transformed collection.
    pub format_types: for<'a> fn(&'a Types) -> std::result::Result<Cow<'a, Types>, FormatError>,
    /// Formats an individual [`DataType`] with access to the surrounding [`Types`].
    ///
    /// Returns [`Cow::Borrowed`] when no changes are needed, or
    /// [`Cow::Owned`] when the formatter produces a transformed datatype.
    pub format_dt:
        for<'a> fn(&'a Types, &'a DataType) -> std::result::Result<Cow<'a, DataType>, FormatError>,
}

impl Format {
    /// Creates a formatter from collection and datatype callbacks.
    pub const fn new(
        format_types: for<'a> fn(&'a Types) -> std::result::Result<Cow<'a, Types>, FormatError>,
        format_dt: for<'a> fn(
            &'a Types,
            &'a DataType,
        ) -> std::result::Result<Cow<'a, DataType>, FormatError>,
    ) -> Self {
        Self {
            format_types,
            format_dt,
        }
    }
}
