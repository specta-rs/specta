use std::borrow::Cow;

use crate::{DataType, GenericType, NamedDataType, NamedFields, UnnamedFields};

#[derive(Debug, Clone, PartialEq)]
pub enum StructFields {
    /// A unit struct.
    ///
    /// Represented in Rust as `pub struct Unit;` and in TypeScript as `null`.
    Unit,
    /// A struct with unnamed fields.
    ///
    /// Represented in Rust as `pub struct Unit();` and in TypeScript as `[]`.
    Unnamed(UnnamedFields),
    /// A struct with named fields.
    ///
    /// Represented in Rust as `pub struct Unit {}` and in TypeScript as `{}`.
    Named(NamedFields),
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructType {
    pub(crate) name: Cow<'static, str>,
    pub(crate) generics: Vec<GenericType>,
    pub(crate) fields: StructFields,
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
            item: DataType::Struct(self),
        }
    }

    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
    }

    pub fn generics(&self) -> &Vec<GenericType> {
        &self.generics
    }

    pub fn fields(&self) -> &StructFields {
        &self.fields
    }

    pub fn tag(&self) -> Option<&Cow<'static, str>> {
        match &self.fields {
            StructFields::Unit => None,
            StructFields::Unnamed(_) => None,
            StructFields::Named(named) => named.tag.as_ref(),
        }
    }
}

impl From<StructType> for DataType {
    fn from(t: StructType) -> Self {
        t.to_anonymous()
    }
}
