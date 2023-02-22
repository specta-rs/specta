use crate::{DataType, NamedDataType, NamedDataTypeItem};

/// represent the type for a tuple.
#[derive(Debug, Clone, PartialEq, Default)]
#[allow(missing_docs)]
pub struct TupleType {
    pub fields: Vec<DataType>,
    pub generics: Vec<&'static str>,
}

impl TupleType {
    /// convert a [`TupleType`] to an anonymous [`DataType`].
    pub fn to_anonymous(self) -> DataType {
        DataType::Tuple(self)
    }

    /// convert a [`TupleType`] to a named [`NamedDataType`].
    ///
    /// This can easily be converted to a [`DataType`] by putting it inside the [DataType::Named] variant.
    pub fn to_named(self, name: &'static str) -> NamedDataType {
        NamedDataType {
            name,
            sid: None,
            impl_location: None,
            comments: &[],
            export: None,
            deprecated: None,
            item: NamedDataTypeItem::Tuple(self),
        }
    }
}

impl From<TupleType> for DataType {
    fn from(t: TupleType) -> Self {
        t.to_anonymous()
    }
}
