use std::{borrow::Cow, error};

use crate::{Types, datatype::DataType};

/// Error type returned by [`Format`] callbacks.
pub type FormatError = Box<dyn error::Error + Send + Sync + 'static>;

/// Formatter callbacks for rewriting collected types and nested datatypes.
///
/// This is used by format integration crates, such as `specta-serde`, to apply
/// format-specific attributes. Exporters and frameworks can also use it for
/// targeted transformations, such as replacing a Rust integer with an
/// exporter-specific bigint representation.
///
/// # Invariants
///
/// Implementations may return [`Cow::Borrowed`] when no transformation is
/// needed. Returned owned values must remain internally consistent: references
/// in mapped datatypes should still resolve against the mapped [`Types`]
/// collection the exporter will use.
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
