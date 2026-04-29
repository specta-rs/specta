use std::borrow::Cow;

use crate::datatype::DataType;

/// Reference to a named generic parameter.
///
/// Exporters usually render this as the generic name, such as `T`.
///
/// # Invariants
///
/// A `Generic` should only appear inside the canonical `ty` field of the
/// [`NamedDataType`](crate::datatype::NamedDataType) that declares it. Ordinary
/// [`Type::definition`](crate::Type::definition) results should use concrete
/// datatypes or references with instantiated generics.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Generic(Cow<'static, str>);

impl Generic {
    /// Builds a new generic parameter reference with the given source-level name.
    ///
    /// The same name must appear in the parent [`GenericDefinition`] list.
    pub const fn new(name: Cow<'static, str>) -> Self {
        Self(name)
    }

    /// The source-level name of this generic parameter.
    pub fn name(&self) -> &Cow<'static, str> {
        &self.0
    }

    /// Get a stable reference identifier for this generic parameter.
    pub fn reference(&self) -> Self {
        self.clone()
    }
}

impl From<Generic> for DataType {
    fn from(v: Generic) -> Self {
        DataType::Generic(v)
    }
}

/// Metadata describing a generic parameter declared by a
/// [`NamedDataType`](crate::datatype::NamedDataType).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct GenericDefinition {
    /// The source-level name of the generic parameter.
    pub name: Cow<'static, str>,
    /// An optional default type for the generic parameter.
    pub default: Option<DataType>,
}

impl GenericDefinition {
    /// Constructs metadata for a generic parameter.
    pub const fn new(name: Cow<'static, str>, default: Option<DataType>) -> Self {
        Self { name, default }
    }

    /// Get a stable reference identifier for this generic parameter.
    pub fn reference(&self) -> Generic {
        Generic::new(self.name.clone())
    }
}
