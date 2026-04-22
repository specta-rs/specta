use std::borrow::Cow;

use crate::datatype::DataType;

/// Reference to a generic parameter.
/// This renders like `T` in the final output.
/// This should only exist in `NamedDataType.ty`, not in normal [`DataType`] values returned from [`Type::definition`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Generic(Cow<'static, str>);

impl Generic {
    /// Build a new [Generic] for a generic type parameter marker.
    /// `T` should be a unique type which identifies the generic (Eg. `pub struct GenericT;`) and must be registered on the parent [`NamedDataType`].
    pub const fn new(name: Cow<'static, str>) -> Self {
        Self(name)
    }
}

impl From<Generic> for DataType {
    fn from(v: Generic) -> Self {
        DataType::Generic(v)
    }
}

/// Metadata describing a named generic parameter on a [`NamedDataType`](crate::datatype::NamedDataType).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct GenericDefinition {
    /// The source-level name of the generic parameter.
    pub name: Cow<'static, str>,
    /// An optional default type for the generic parameter.
    pub default: Option<DataType>,
}

impl GenericDefinition {
    /// Construct metadata for a generic parameter marker type.
    pub const fn new(name: Cow<'static, str>, default: Option<DataType>) -> Self {
        Self { name, default }
    }
}
