use std::borrow::Cow;

use crate::{DataType, GenericType, NamedDataType, NamedDataTypeItem, TupleType};

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

#[derive(Debug, Clone, PartialEq)]
pub struct StructNamedFields {
    pub(crate) generics: Vec<GenericType>,
    pub(crate) fields: Vec<StructField>,
    pub(crate) tag: Option<Cow<'static, str>>,
}

impl StructNamedFields {
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

#[derive(Debug, Clone, PartialEq)]
pub enum StructType {
    /// A unit struct.
    ///
    /// Represented in Rust as `pub struct Unit;` and in TypeScript as `null`.
    Unit,
    /// A struct with unnamed fields.
    ///
    /// Represented in Rust as `pub struct Unit();` and in TypeScript as `[]`.
    Unnamed(TupleType),
    /// A struct with named fields.
    ///
    /// Represented in Rust as `pub struct Unit{};` and in TypeScript as `{}`.
    Named(StructNamedFields),
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

    pub fn generics(&self) -> Vec<GenericType> {
        match self {
            StructType::Unit => vec![], // TODO: Cringe this means we can't return `&Vec<_>`
            StructType::Unnamed(unnamed) => unnamed.generics.clone(),
            StructType::Named(named) => named.generics.clone(),
        }
    }

    pub fn fields(&self) -> Vec<StructField> {
        match self {
            StructType::Unit => vec![], // TODO: Cringe this means we can't return `&Vec<_>`
            StructType::Unnamed(unnamed) => unnamed
                .fields
                .clone()
                .into_iter()
                // TODO: This is a bad conversions. Refactor to avoid it!
                .map(|f| StructField {
                    key: "".into(),
                    optional: false,
                    flatten: false,
                    ty: f,
                })
                .collect(),
            StructType::Named(named) => named.fields.clone(),
        }
    }

    pub fn tag(&self) -> Option<&Cow<'static, str>> {
        match self {
            StructType::Unit => None, // TODO: Cringe this means we can't return `&Vec<_>`
            StructType::Unnamed(_) => None, // TODO: unnamed.tag.as_ref(),
            StructType::Named(named) => named.tag.as_ref(),
        }
    }
}

impl From<StructType> for DataType {
    fn from(t: StructType) -> Self {
        t.to_anonymous()
    }
}
