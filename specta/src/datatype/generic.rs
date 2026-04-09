use std::{any::TypeId, borrow::Cow};

use crate::datatype::{DataType, GenericReference};

/// Metadata describing a named generic parameter on a [`NamedDataType`](crate::datatype::NamedDataType).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Generic {
    id: TypeId,
    /// The source-level name of the generic parameter.
    pub name: Cow<'static, str>,
    /// An optional default type for the generic parameter.
    pub default: Option<DataType>,
}

impl Generic {
    /// Construct metadata for a generic parameter marker type.
    pub const fn new<T: ?Sized + 'static>(
        name: Cow<'static, str>,
        default: Option<DataType>,
    ) -> Self {
        Self {
            id: TypeId::of::<T>(),
            name,
            default,
        }
    }

    /// Construct a reference to this generic parameter.
    pub const fn reference(&self) -> GenericReference {
        GenericReference { id: self.id }
    }
}
