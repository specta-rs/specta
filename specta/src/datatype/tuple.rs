use std::borrow::Cow;

use crate::{DataType, NamedDataType};

/// A regular tuple
///
/// Represented in Rust as `(...)` and in TypeScript as `[...]`.
/// Be aware `()` is treated specially as `null` in Typescript.
#[derive(Debug, Clone, PartialEq)]
pub struct TupleType {
    pub(crate) elements: Vec<DataType>,
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
            docs: Cow::Borrowed(""),
            deprecated: None,
            ext: None,
            inner: DataType::Tuple(self),
        }
    }

    pub fn elements(&self) -> &Vec<DataType> {
        &self.elements
    }
}

impl From<TupleType> for DataType {
    fn from(t: TupleType) -> Self {
        t.to_anonymous()
    }
}
