use crate::{DataType, NamedDataType, NamedDataTypeItem};

/// represent a the type for a field in an [ObjectType].
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct ObjectField {
    pub key: &'static str,
    pub optional: bool,
    pub flatten: bool,
    pub ty: DataType,
}

/// represent the type for an object.
#[derive(Debug, Clone, PartialEq, Default)]
#[allow(missing_docs)]
pub struct ObjectType {
    pub generics: Vec<&'static str>,
    pub fields: Vec<ObjectField>,
    pub tag: Option<&'static str>,
}

impl ObjectType {
    /// convert a [`ObjectType`] to an anonymous [`DataType`].
    pub fn to_anonymous(self) -> DataType {
        DataType::Object(self)
    }

    /// convert a [`ObjectType`] to a named [`NamedDataType`].
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
            item: NamedDataTypeItem::Object(self),
        }
    }
}

impl From<ObjectType> for DataType {
    fn from(t: ObjectType) -> Self {
        t.to_anonymous()
    }
}
