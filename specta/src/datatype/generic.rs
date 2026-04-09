use std::{any::TypeId, borrow::Cow};

use crate::datatype::{DataType, GenericReference};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Generic {
    id: TypeId,
    pub name: Cow<'static, str>,
    pub default: Option<DataType>,
}

impl Generic {
    pub fn new<T: 'static>(name: Cow<'static, str>, default: Option<DataType>) -> Self {
        Self {
            id: TypeId::of::<T>(),
            name,
            default,
        }
    }

    pub fn into_reference(self) -> GenericReference {
        GenericReference { id: self.id }
    }
}
