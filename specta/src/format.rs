use std::{borrow::Cow, error};

use crate::{Types, datatype::DataType};

/// Error type returned by [`Format`] callbacks.
pub type FormatError = Box<dyn error::Error + Send + Sync + 'static>;

/// Marker error for a formatter that cannot rewrite a node containing an
/// unresolvable [`DataType::Generic`](crate::datatype::DataType::Generic)
/// placeholder.
///
/// A formatter returns `UnresolvedGeneric` when the node is part of a generic
/// declaration and the formatter needs a concrete argument to decide the shape.
/// The shared traversal in [`Types::format`](crate::Types::format) then leaves
/// that node unrewritten and formats its children instead, so a generic
/// declaration is not rejected just because one of its nodes is abstract.
///
/// Return it directly or as the [`std::error::Error::source`] of your own
/// error; the traversal follows the source chain.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct UnresolvedGeneric;

impl std::fmt::Display for UnresolvedGeneric {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("the node contains an unresolved generic parameter")
    }
}

impl error::Error for UnresolvedGeneric {}

/// A formatting failure annotated with the declaration being formatted.
#[derive(Debug)]
pub(crate) struct FormatContext {
    context: String,
    named_datatype: Option<crate::datatype::NamedDataType>,
    source: FormatError,
}

impl std::fmt::Display for FormatContext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "while formatting {}: {}", self.context, self.source)
    }
}

impl error::Error for FormatContext {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        Some(self.source.as_ref())
    }
}

pub(crate) fn with_context(
    context: String,
    named_datatype: Option<crate::datatype::NamedDataType>,
    source: FormatError,
) -> FormatError {
    Box::new(FormatContext {
        context,
        named_datatype,
        source,
    })
}

/// The named declaration being formatted when `err` was produced, if any.
///
/// Exporters use this to attach the offending declaration to their own error
/// type, so callers keep the same diagnostics regardless of which exporter they
/// use. The declaration is reported by the shared traversal in
/// [`Types::format`](crate::Types::format).
pub fn named_datatype<'a>(
    err: &'a (dyn error::Error + 'static),
) -> Option<&'a crate::datatype::NamedDataType> {
    let mut current = Some(err);
    while let Some(err) = current {
        if let Some(context) = err.downcast_ref::<FormatContext>() {
            return context.named_datatype.as_ref();
        }
        current = err.source();
    }
    None
}

/// Whether `err`, or anything in its source chain, is [`UnresolvedGeneric`].
pub fn is_unresolved_generic(err: &(dyn error::Error + 'static)) -> bool {
    let mut current = Some(err);
    while let Some(err) = current {
        if err.is::<UnresolvedGeneric>() {
            return true;
        }
        current = err.source();
    }
    false
}

/// Applies `format` to `types` and returns the graph an exporter should render.
///
/// This is the free-function form of [`Types::format`], which documents the
/// traversal contract shared by every exporter.
pub fn format_types(types: &crate::Types, format: &dyn Format) -> std::result::Result<crate::Types, FormatError> {
    types.format(format)
}

/// The identity [`Format`]: leaves every datatype as collected.
///
/// Exporters and callers that do not transform the graph use this instead of
/// defining a private no-op formatter.
#[derive(Debug, Clone, Copy, Default)]
pub struct Identity;

impl Format for Identity {
    fn map_types<'a>(
        &'a self,
        types: &'a crate::Types,
    ) -> std::result::Result<std::borrow::Cow<'a, crate::Types>, FormatError> {
        Ok(std::borrow::Cow::Borrowed(types))
    }

    fn map_type<'a>(
        &'a self,
        _: &'a crate::Types,
        dt: &'a crate::datatype::DataType,
    ) -> std::result::Result<std::borrow::Cow<'a, crate::datatype::DataType>, FormatError> {
        Ok(std::borrow::Cow::Borrowed(dt))
    }
}

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
    fn map_types<'a>(
        &'a self,
        types: &'a Types,
    ) -> std::result::Result<Cow<'a, Types>, FormatError>;

    /// Map an individual [`DataType`] with access to the surrounding [`Types`].
    ///
    /// Returns [`Cow::Borrowed`] when no changes are needed, or
    /// [`Cow::Owned`] when the formatter produces a transformed datatype.
    fn map_type<'a>(
        &'a self,
        types: &'a Types,
        dt: &'a DataType,
    ) -> std::result::Result<Cow<'a, DataType>, FormatError>;
}

impl<T: Format + ?Sized> Format for &T {
    fn map_types<'a>(
        &'a self,
        types: &'a Types,
    ) -> std::result::Result<Cow<'a, Types>, FormatError> {
        (**self).map_types(types)
    }

    fn map_type<'a>(
        &'a self,
        types: &'a Types,
        dt: &'a DataType,
    ) -> std::result::Result<Cow<'a, DataType>, FormatError> {
        (**self).map_type(types, dt)
    }
}

impl<T: Format + ?Sized> Format for Box<T> {
    fn map_types<'a>(
        &'a self,
        types: &'a Types,
    ) -> std::result::Result<Cow<'a, Types>, FormatError> {
        (**self).map_types(types)
    }

    fn map_type<'a>(
        &'a self,
        types: &'a Types,
        dt: &'a DataType,
    ) -> std::result::Result<Cow<'a, DataType>, FormatError> {
        (**self).map_type(types, dt)
    }
}

// Assert dyn-safety
const _: Option<&dyn Format> = None;
