//! Field types are used by both enums and structs.

use crate::datatype::Struct;

use super::{Attributes, DataType, Deprecated};
use std::borrow::Cow;

/// Field layout for a struct or enum variant.
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

/// Metadata for a struct field or enum variant field.
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Field {
    /// Whether the field was marked optional, for example with
    /// `#[specta(optional)]`.
    pub optional: bool,
    /// Deprecated attribute for the field.
    pub deprecated: Option<Deprecated>,
    /// Documentation comments for the field.
    pub docs: Cow<'static, str>,
    /// Runtime attributes for this field.
    pub attributes: Attributes,
    /// Type for the field.
    ///
    /// This is `None` when the field was skipped with an attribute such as
    /// `#[serde(skip)]` or `#[specta(skip)]`. Exporters should preserve enough
    /// information to distinguish skipped fields from absent fields because some
    /// serialization formats still let skipped fields affect layout.
    ///
    /// You might think, well why not apply this in the macro and just not emit the variant?
    /// Well in Serde `A(String)` and `A(#[serde(skip)] (), String)` export as different Typescript types so the exporter needs runtime knowledge of this.
    pub ty: Option<DataType>,
}

impl Field {
    /// Constructs a new non-optional field with the given type.
    ///
    /// Use [`Field::default`] to construct skipped-field metadata where `ty` is
    /// initially `None`.
    pub fn new(ty: DataType) -> Self {
        Field {
            optional: false,
            deprecated: None,
            docs: "".into(),
            ty: Some(ty),
            attributes: Attributes::default(),
        }
    }
}

/// Fields for an unnamed tuple struct or tuple enum variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct UnnamedFields {
    /// Field metadata in source order.
    pub fields: Vec<Field>,
}

/// Fields for a named struct or struct enum variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct NamedFields {
    /// Field names and metadata in source order.
    pub fields: Vec<(Cow<'static, str>, Field)>,
}

#[derive(Debug, Clone)]
/// Builder for constructing [`DataType::Struct`] values.
///
/// The type parameter tracks whether the builder is currently collecting named
/// or unnamed fields.
pub struct StructBuilder<F = ()> {
    pub(crate) fields: F,
}

impl StructBuilder<NamedFields> {
    /// Adds a named field and returns the updated builder.
    pub fn field(mut self, name: impl Into<Cow<'static, str>>, field: Field) -> Self {
        self.fields.fields.push((name.into(), field));
        self
    }

    /// Adds a named field in-place.
    pub fn field_mut(&mut self, name: impl Into<Cow<'static, str>>, field: Field) {
        self.fields.fields.push((name.into(), field));
    }

    /// Finalizes this builder into a [`DataType::Struct`] with named fields.
    pub fn build(self) -> DataType {
        DataType::Struct(Struct {
            fields: Fields::Named(self.fields),
            attributes: Default::default(),
        })
    }
}

impl StructBuilder<UnnamedFields> {
    /// Adds an unnamed field and returns the updated builder.
    pub fn field(mut self, field: Field) -> Self {
        self.fields.fields.push(field);
        self
    }

    /// Adds an unnamed field in-place.
    pub fn field_mut(&mut self, field: Field) {
        self.fields.fields.push(field);
    }

    /// Finalizes this builder into a [`DataType::Struct`] with unnamed fields.
    pub fn build(self) -> DataType {
        DataType::Struct(Struct {
            fields: Fields::Unnamed(self.fields),
            attributes: Default::default(),
        })
    }
}
