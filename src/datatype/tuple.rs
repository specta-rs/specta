use crate::{CustomDataType, DataType, NamedCustomDataType};

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TupleType {
    pub fields: Vec<DataType>,
    pub generics: Vec<&'static str>,
}

impl TupleType {
    pub fn to_anonymous(self) -> DataType {
        self.into()
    }

    pub fn to_named(self, name: &'static str) -> DataType {
        DataType::Tuple(CustomDataType::Named(NamedCustomDataType {
            name,
            item: self,
            ..Default::default()
        }))
    }
}

impl From<TupleType> for DataType {
    fn from(t: TupleType) -> Self {
        DataType::Tuple(CustomDataType::Anonymous(t))
    }
}
