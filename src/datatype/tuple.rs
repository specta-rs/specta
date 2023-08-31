use std::borrow::Cow;

use crate::{DataType, GenericType, NamedDataType, NamedDataTypeItem};

/// A regular tuple
///
/// Represented in Rust as `(...)` and in TypeScript as `[...]`.
/// Be aware `()` is treated specially as `null` in Typescript.
#[derive(Debug, Clone, PartialEq)]
pub struct TupleType {
    pub(crate) fields: Vec<DataType>,
    pub(crate) generics: Vec<GenericType>,
}

impl TupleType {
    /// convert a [`TupleType`] to an anonymous [`DataType`].
    pub fn to_anonymous(self) -> DataType {
        DataType::Tuple(self)
    }

    /// convert a [`TupleType`] to a named [`NamedDataType`].
    ///
    /// This can easily be converted to a [`DataType`] by putting it inside the [DataType::Named] variant.
    pub fn to_named(self, name: impl Into<Cow<'static, str>>) -> NamedDataType {
        NamedDataType {
            name: name.into(),
            comments: vec![],
            deprecated: None,
            ext: None,
            item: NamedDataTypeItem::Tuple(self),
        }
    }

    pub fn fields(&self) -> impl Iterator<Item = &DataType> {
        self.fields.iter()
    }

    pub fn generics(&self) -> impl Iterator<Item = &GenericType> {
        self.generics.iter()
    }
}

impl From<TupleType> for DataType {
    fn from(t: TupleType) -> Self {
        t.to_anonymous()
    }
}
