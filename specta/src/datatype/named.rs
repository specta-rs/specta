use std::borrow::Cow;

use crate::{ImplLocation, SpectaID};

use super::DataType;

/// A named type represents a non-primitive type capable of being exported as it's own named entity.
#[derive(Debug, Clone, PartialEq)]
pub struct NamedDataType {
    pub(crate) name: Cow<'static, str>,
    pub(crate) docs: Cow<'static, str>,
    pub(crate) deprecated: Option<DeprecatedType>,
    pub(crate) sid: SpectaID,
    pub(crate) impl_location: ImplLocation,
    pub(crate) inner: DataType,
}

impl NamedDataType {
    /// The name of the type
    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
    }

    /// Rust documentation comments on the type
    pub fn docs(&self) -> &Cow<'static, str> {
        &self.docs
    }

    /// The Rust deprecated comment if the type is deprecated.
    pub fn deprecated(&self) -> Option<&DeprecatedType> {
        self.deprecated.as_ref()
    }

    /// The Specta ID for the type. The value for this should come from the `sid!();` macro.
    pub fn sid(&self) -> &SpectaID {
        &self.sid
    }

    /// The code location where this type is implemented. Used for error reporting.
    pub fn impl_location(&self) -> &ImplLocation {
        &self.impl_location
    }

    /// Get the inner [`DataType`]
    pub fn ty(&self) -> &DataType {
        &self.inner
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DeprecatedType {
    /// A type that has been deprecated without a message.
    ///
    /// Eg. `#[deprecated]`
    Deprecated,
    /// A type that has been deprecated with a message and an optional `since` version.
    ///
    /// Eg. `#[deprecated = "Use something else"]` or `#[deprecated(since = "1.0.0", message = "Use something else")]`
    DeprecatedWithSince {
        since: Option<Cow<'static, str>>,
        note: Cow<'static, str>,
    },
}
