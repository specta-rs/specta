use std::borrow::Cow;

use crate::datatype::DataType;

/// Metadata describing a named generic parameter on a [`NamedDataType`](crate::datatype::NamedDataType).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Generic {
    /// The source-level name of the generic parameter.
    pub name: Cow<'static, str>,
    /// An optional default type for the generic parameter.
    pub default: Option<DataType>,
}

impl Generic {
    /// Construct metadata for a generic parameter marker type.
    pub const fn new(name: Cow<'static, str>, default: Option<DataType>) -> Self {
        Self { name, default }
    }
}
