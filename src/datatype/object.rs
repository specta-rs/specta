use crate::{DataType, NamedDataType, NamedDataTypeItem};

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
    /// TODO
    pub fn to_anonymous(self) -> DataType {
        DataType::Object(self)
    }

    /// TODO
    pub fn to_named(self, name: &'static str) -> NamedDataType {
        NamedDataType {
            name,
            sid: None,
            impl_location: None,
            comments: &[],
            export: None,
            deprecated: None,
            item: NamedDataTypeItem::Object(self),
        }
    }
}

impl From<ObjectType> for DataType {
    fn from(t: ObjectType) -> Self {
        t.to_anonymous()
    }
}
