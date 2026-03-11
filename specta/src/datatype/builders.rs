//! TODO: Move this somewhere else. Maybe out of core and maybe properly expose?
//!
//! TODO: Option to build types with generics???

use std::{borrow::Cow, fmt::Debug};

use crate::datatype::{
    Attributes, DataType, DeprecatedType, Field, Fields, NamedDataType, NamedFields, Struct,
    UnnamedFields, Variant,
};

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

/// Builder for constructing [`Variant`] values.
#[derive(Debug, Clone)]
pub struct VariantBuilder<V = ()> {
    pub(crate) v: Variant,
    pub(crate) variant: V,
}

impl<T> VariantBuilder<T> {
    /// Mark the variant as skipped.
    pub fn skip(mut self) -> Self {
        self.v.skip = true;
        self
    }

    /// Set documentation for the variant.
    pub fn docs(mut self, docs: Cow<'static, str>) -> Self {
        self.v.docs = docs;
        self
    }

    /// Set deprecation metadata for the variant.
    pub fn deprecated(mut self, reason: DeprecatedType) -> Self {
        self.v.deprecated = Some(reason);
        self
    }

    /// Set runtime attributes on the variant.
    pub fn attributes(mut self, attributes: Attributes) -> Self {
        self.v.attributes = attributes;
        self
    }

    /// Set runtime attributes on the variant in-place.
    pub fn attributes_mut(&mut self, attributes: Attributes) {
        self.v.attributes = attributes;
    }
}

impl VariantBuilder<NamedFields> {
    /// Add a named field to the variant.
    pub fn field(mut self, name: impl Into<Cow<'static, str>>, field: Field) -> Self {
        self.variant.fields.push((name.into(), field));
        self
    }

    /// Add a named field to the variant and return the updated builder.
    pub fn field_mut(mut self, name: impl Into<Cow<'static, str>>, field: Field) -> Self {
        self.variant.fields.push((name.into(), field));
        self
    }

    /// Finalize this named variant builder.
    pub fn build(mut self) -> Variant {
        self.v.fields = Fields::Named(self.variant);
        self.v
    }
}

impl From<VariantBuilder<NamedFields>> for Variant {
    fn from(val: VariantBuilder<NamedFields>) -> Self {
        val.build()
    }
}

impl VariantBuilder<UnnamedFields> {
    /// Add an unnamed field to the variant.
    pub fn field(mut self, field: Field) -> Self {
        self.variant.fields.push(field);
        self
    }

    /// Add an unnamed field to the variant and return the updated builder.
    pub fn field_mut(mut self, field: Field) -> Self {
        self.variant.fields.push(field);
        self
    }

    /// Finalize this unnamed variant builder.
    pub fn build(mut self) -> Variant {
        self.v.fields = Fields::Unnamed(self.variant);
        self.v
    }
}

impl From<VariantBuilder<UnnamedFields>> for Variant {
    fn from(val: VariantBuilder<UnnamedFields>) -> Self {
        val.build()
    }
}
