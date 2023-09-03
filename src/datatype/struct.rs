use std::borrow::Cow;

use crate::{DataType, GenericType, NamedDataType, NamedDataTypeItem};

/// A field in an [`StructType`].
#[derive(Debug, Clone, PartialEq)]
pub struct StructField {
    pub(crate) key: Cow<'static, str>,
    pub(crate) optional: bool,
    pub(crate) flatten: bool,
    pub(crate) ty: DataType,
}

impl StructField {
    pub fn key(&self) -> &Cow<'static, str> {
        &self.key
    }

    pub fn optional(&self) -> bool {
        self.optional
    }

    pub fn flatten(&self) -> bool {
        self.flatten
    }

    pub fn ty(&self) -> &DataType {
        &self.ty
    }
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
            comments: vec![],
            deprecated: None,
            ext: None,
            item: NamedDataTypeItem::Struct(self),
        }
    }

    pub fn generics(&self) -> impl Iterator<Item = &GenericType> {
        self.generics.iter()
    }

    pub fn fields(&self) -> impl Iterator<Item = &StructField> {
        self.fields.iter()
    }

    pub fn tag(&self) -> Option<&Cow<'static, str>> {
        self.tag.as_ref()
    }
}

impl From<StructType> for DataType {
    fn from(t: StructType) -> Self {
        t.to_anonymous()
    }
}
