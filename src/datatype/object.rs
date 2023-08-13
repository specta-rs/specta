use std::borrow::Cow;

use crate::{DataType, NamedDataType, NamedDataTypeItem};

/// A field in an [`ObjectType`].
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct ObjectField {
    pub key: Cow<'static, str>,
    pub optional: bool,
    pub flatten: bool,
    pub ty: DataType,
}

/// Type of an object.
/// Could be from a struct or named enum variant.
#[derive(Debug, Clone, PartialEq, Default)]
#[allow(missing_docs)]
pub struct ObjectType {
    pub generics: Vec<Cow<'static, str>>,
    pub fields: Vec<ObjectField>,
    pub tag: Option<Cow<'static, str>>,
}

impl ObjectType {
    /// Convert a [`ObjectType`] to an anonymous [`DataType`].
    pub fn to_anonymous(self) -> DataType {
        DataType::Object(self)
    }

    /// Convert a [`ObjectType`] to a named [`NamedDataType`].
    ///
    /// This can easily be converted to a [`DataType`] by putting it inside the [DataType::Named] variant.
    pub fn to_named(self, name: impl Into<Cow<'static, str>>) -> NamedDataType {
        NamedDataType {
            name: name.into(),
            sid: None,
            impl_location: None,
            comments: vec![],
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
