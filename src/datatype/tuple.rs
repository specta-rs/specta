use crate::{CustomDataType, DataType};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TupleType {
    pub fields: Vec<DataType>,
    pub generics: Vec<&'static str>,
}

impl TupleType {
    pub fn to_anonymous(self) -> DataType {
        DataType::Tuple(CustomDataType::Anonymous(self))
    }

    pub fn to_named(self, name: &'static str) -> DataType {
        DataType::Tuple(CustomDataType::named(name, self))
    }
}

impl From<TupleType> for DataType {
    fn from(t: TupleType) -> Self {
        t.to_anonymous()
    }
}
