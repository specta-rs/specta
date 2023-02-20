use crate::{CustomDataType, DataType, NamedCustomDataType};

/// this is used internally to represent the types.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct ObjectField {
    pub key: &'static str,
    pub optional: bool,
    pub flatten: bool,
    pub ty: DataType,
}

/// this is used internally to represent the types.
#[derive(Debug, Clone, PartialEq, Default)]
#[allow(missing_docs)]
pub struct ObjectType {
    pub generics: Vec<&'static str>,
    pub fields: Vec<ObjectField>,
    pub tag: Option<&'static str>,
}

impl ObjectType {
    pub fn to_anonymous(self) -> DataType {
        self.into()
    }

    pub fn to_named(self, name: &'static str) -> DataType {
        DataType::Object(crate::CustomDataType::Named(NamedCustomDataType {
            name,
            item: self,
            ..Default::default()
        }))
    }
}

impl From<ObjectType> for DataType {
    fn from(t: ObjectType) -> Self {
        Self::Object(CustomDataType::Anonymous(t))
    }
}
