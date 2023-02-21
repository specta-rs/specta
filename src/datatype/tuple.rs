use crate::{DataType, NamedDataType, NamedDataTypeItem};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TupleType {
    pub fields: Vec<DataType>,
    pub generics: Vec<&'static str>,
}

impl TupleType {
    pub fn to_anonymous(self) -> DataType {
        DataType::Tuple(self)
    }

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
