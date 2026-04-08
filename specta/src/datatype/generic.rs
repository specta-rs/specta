use std::{any::TypeId, borrow::Cow};

use crate::datatype::{DataType, GenericReference};

pub enum GenericType {
    Type,
    Const { ty: DataType },
}

// TODO: Sealing fields
pub struct Generic {
    id: TypeId,
    pub name: Cow<'static, str>,
    pub default: Option<DataType>,
    pub inner: GenericType,
}

impl Generic {
    pub fn new<T: 'static>(name: Cow<'static, str>, default: Option<DataType>) -> Self {
        Self {
            id: TypeId::of::<T>(),
            name,
            default,
            inner: GenericType::Type,
        }
    }

    pub fn new_const<T: 'static>(
        name: Cow<'static, str>,
        ty: DataType,
        default: Option<DataType>,
    ) -> Self {
        Self {
            id: TypeId::of::<T>(),
            name,
            default,
            inner: GenericType::Const { ty },
        }
    }

    pub fn into_reference(self) -> GenericReference {
        GenericReference { id: self.id }
    }
}
