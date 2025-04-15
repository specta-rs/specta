use std::{borrow::Cow, panic::Location};

use crate::SpectaID;

use super::{DataType, Generic};

/// A named type represents a non-primitive type capable of being exported as it's own named entity.
#[derive(Debug, Clone, PartialEq)]
pub struct NamedDataType {
    pub(crate) name: Cow<'static, str>,
    pub(crate) docs: Cow<'static, str>,
    pub(crate) deprecated: Option<DeprecatedType>,
    pub(crate) sid: SpectaID,
    pub(crate) module_path: Cow<'static, str>,
    pub(crate) location: Location<'static>,
    pub(crate) generics: Vec<Generic>,
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

    /// The code location where this type is implemented
    pub fn location(&self) -> Location {
        self.location
    }

    /// The Rust path of the module where this type is defined
    pub fn module_path(&self) -> &str {
        &self.module_path
    }

    /// The generics that are defined on this type
    pub fn generics(&self) -> &Vec<Generic> {
        &self.generics
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
