use std::borrow::Cow;

use crate::{DataType, GenericType, NamedDataType, NamedDataTypeItem};

/// A field in an [`StructType`].
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct StructField {
    pub key: Cow<'static, str>,
    pub optional: bool,
    pub flatten: bool,
    pub ty: DataType,
}

/// Type of a struct.
/// Could be from a struct or named enum variant.
#[derive(Debug, Clone, PartialEq)]
pub struct StructType {
    pub(crate) generics: Vec<GenericType>,
    pub(crate) fields: Vec<StructField>,
    pub(crate) tag: Option<Cow<'static, str>>,
}

impl StructType {
    /// Convert a [`StructType`] to an anonymous [`DataType`].
    pub fn to_anonymous(self) -> DataType {
        DataType::Struct(self)
    }

    /// Convert a [`StructType`] to a named [`NamedDataType`].
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
            item: NamedDataTypeItem::Struct(self),
        }
    }

    pub fn generics(&self) -> impl Iterator<Item = &GenericType> {
        self.generics.iter()
    }

    pub fn fields(&self) -> impl Iterator<Item = &StructField> {
        self.fields.iter()
    }

    pub fn tag(&self) -> &Option<Cow<'static, str>> {
        &self.tag
    }
}

impl From<StructType> for DataType {
    fn from(t: StructType) -> Self {
        t.to_anonymous()
    }
}
