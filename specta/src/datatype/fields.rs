//! Field types are used by both enums and structs.

use crate::datatype::Struct;

use super::{Attributes, DataType, Deprecated};
use std::borrow::Cow;

/// Data stored within an enum variant or struct.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Fields {
    /// Unit struct.
    ///
    /// Represented in Rust as `pub struct Unit;` and in TypeScript as `null`.
    Unit,
    /// Struct with unnamed fields.
    ///
    /// Represented in Rust as `pub struct Unit();` and in TypeScript as `[]`.
    Unnamed(UnnamedFields),
    /// Struct with named fields.
    ///
    /// Represented in Rust as `pub struct Unit {}` and in TypeScript as `{}`.
    Named(NamedFields),
}

/// Field metadata for a struct field or enum variant field.
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Field {
    /// Did the user apply a `#[specta(optional)]` attribute.
    pub optional: bool,
    /// Did the user apply a `#[serde(flatten)]` attribute.
    pub flatten: bool,
    /// Deprecated attribute for the field.
    pub deprecated: Option<Deprecated>,
    /// Documentation comments for the field.
    pub docs: Cow<'static, str>,
    /// Should we inline the definition of this type.
    pub inline: bool,
    /// Did the user apply a `#[specta(type = ...)]` or `#[specta(r#type = ...)]` attribute.
    pub type_overridden: bool,
    /// Runtime attributes for this field.
    pub attributes: Attributes,
    /// Type for the field. Is optional if `#[serde(skip)]` or `#[specta(skip)]` was applied.
    ///
    /// You might think, well why not apply this in the macro and just not emit the variant?
    /// Well in Serde `A(String)` and `A(#[serde(skip)] (), String)` export as different Typescript types so the exporter needs runtime knowledge of this.
    pub ty: Option<DataType>,
}

impl Field {
    /// Construct a new field with the given type.
    ///
    /// You can skip the requirement on providing a [`DataType`] by using [`Field::default`]
    pub fn new(ty: DataType) -> Self {
        Field {
            optional: false,
            flatten: false,
            deprecated: None,
            docs: "".into(),
            inline: false,
            type_overridden: false,
            ty: Some(ty),
            attributes: Attributes::default(),
        }
    }
}

/// The fields of an unnamed enum variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct UnnamedFields {
    pub fields: Vec<Field>,
}

/// The fields of an named enum variant or a struct.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct NamedFields {
    pub fields: Vec<(Cow<'static, str>, Field)>,
}

#[derive(Debug, Clone)]
/// Builder for constructing [`DataType::Struct`] values.
pub struct StructBuilder<F = ()> {
    pub(crate) fields: F,
}

impl StructBuilder<NamedFields> {
    /// Add a named field.
    pub fn field(mut self, name: impl Into<Cow<'static, str>>, field: Field) -> Self {
        self.fields.fields.push((name.into(), field));
        self
    }

    /// Add a named field in-place.
    pub fn field_mut(&mut self, name: impl Into<Cow<'static, str>>, field: Field) {
        self.fields.fields.push((name.into(), field));
    }

    /// Finalize this builder into a [`DataType`].
    pub fn build(self) -> DataType {
        DataType::Struct(Struct {
            fields: Fields::Named(self.fields),
            attributes: Default::default(),
        })
    }
}

impl StructBuilder<UnnamedFields> {
    /// Add an unnamed field.
    pub fn field(mut self, field: Field) -> Self {
        self.fields.fields.push(field);
        self
    }

    /// Add an unnamed field in-place.
    pub fn field_mut(&mut self, field: Field) {
        self.fields.fields.push(field);
    }

    /// Finalize this builder into a [`DataType`].
    pub fn build(self) -> DataType {
        DataType::Struct(Struct {
            fields: Fields::Unnamed(self.fields),
            attributes: Default::default(),
        })
    }
}
