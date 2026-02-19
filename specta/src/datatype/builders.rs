//! TODO: Move this somewhere else. Maybe out of core and maybe properly expose?
//!
//! TODO: Option to build types with generics???

use std::{borrow::Cow, fmt::Debug};

use crate::{
    TypeCollection,
    datatype::{
        DataType, DeprecatedType, EnumVariant, Field, Fields, Generic, NamedDataType, NamedFields,
        RuntimeAttribute, Struct, UnnamedFields,
    },
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

    /// Set runtime attributes for the unnamed fields collection.
    pub fn attributes(mut self, attributes: Vec<RuntimeAttribute>) -> Self {
        self.fields.attributes = attributes;
        self
    }

    /// Set runtime attributes for the unnamed fields collection in-place.
    pub fn attributes_mut(&mut self, attributes: Vec<RuntimeAttribute>) {
        self.fields.attributes = attributes;
    }

    /// Finalize this builder into a [`DataType`].
    pub fn build(self) -> DataType {
        DataType::Struct(Struct {
            fields: Fields::Unnamed(self.fields),
            attributes: Default::default(),
        })
    }
}

/// Builder for constructing [`EnumVariant`] values.
#[derive(Debug, Clone)]
pub struct VariantBuilder<V = ()> {
    pub(crate) v: EnumVariant,
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
    pub fn attributes(mut self, attributes: Vec<RuntimeAttribute>) -> Self {
        self.v.attributes = attributes;
        self
    }

    /// Set runtime attributes on the variant in-place.
    pub fn attributes_mut(&mut self, attributes: Vec<RuntimeAttribute>) {
        self.v.attributes = attributes;
    }
}

impl VariantBuilder<NamedFields> {
    /// Add a named field to the variant.
    pub fn field(mut self, name: impl Into<Cow<'static, str>>, field: Field) -> Self {
        match &mut self.v.fields {
            Fields::Named(f) => f.fields.push((name.into(), field)),
            _ => unreachable!(),
        }
        self
    }

    /// Add a named field to the variant and return the updated builder.
    pub fn field_mut(mut self, name: impl Into<Cow<'static, str>>, field: Field) -> Self {
        match &mut self.v.fields {
            Fields::Named(f) => f.fields.push((name.into(), field)),
            _ => unreachable!(),
        }
        self
    }

    /// Finalize this named variant builder.
    pub fn build(mut self) -> EnumVariant {
        self.v.fields = Fields::Named(self.variant);
        self.v
    }
}

impl From<VariantBuilder<NamedFields>> for EnumVariant {
    fn from(val: VariantBuilder<NamedFields>) -> Self {
        val.build()
    }
}

impl VariantBuilder<UnnamedFields> {
    /// Add an unnamed field to the variant.
    pub fn field(mut self, field: Field) -> Self {
        match &mut self.v.fields {
            Fields::Unnamed(f) => f.fields.push(field),
            _ => unreachable!(),
        }
        self
    }

    /// Add an unnamed field to the variant and return the updated builder.
    pub fn field_mut(mut self, field: Field) -> Self {
        match &mut self.v.fields {
            Fields::Unnamed(f) => f.fields.push(field),
            _ => unreachable!(),
        }
        self
    }

    /// Finalize this unnamed variant builder.
    pub fn build(mut self) -> EnumVariant {
        self.v.fields = Fields::Unnamed(self.variant);
        self.v
    }
}

impl From<VariantBuilder<UnnamedFields>> for EnumVariant {
    fn from(val: VariantBuilder<UnnamedFields>) -> Self {
        val.build()
    }
}

/// Builder for registering a runtime [`NamedDataType`].
#[derive(Debug, Clone)]
pub struct NamedDataTypeBuilder {
    pub(crate) name: Cow<'static, str>,
    pub(crate) docs: Cow<'static, str>,
    pub(crate) deprecated: Option<DeprecatedType>,
    pub(crate) module_path: Option<Cow<'static, str>>,
    pub(crate) generics: Vec<Generic>,
    pub(crate) inline: bool,
    pub(crate) inner: DataType,
}

impl NamedDataTypeBuilder {
    /// Construct a new named datatype builder.
    pub fn new(name: impl Into<Cow<'static, str>>, generics: Vec<Generic>, dt: DataType) -> Self {
        Self {
            name: name.into(),
            docs: Cow::Borrowed(""),
            deprecated: None,
            module_path: None,
            generics,
            inline: false,
            inner: dt,
        }
    }

    /// Set the module path that this type was defined in.
    ///
    /// The value for this is usually determined by [`module_path`](std::module_path). It's important you keep this in the form `edge::edge::edge::node` or `node`.
    pub fn module_path(mut self, module_path: impl Into<Cow<'static, str>>) -> Self {
        self.module_path = Some(module_path.into());
        self
    }

    /// Set Rust doc comments for this type.
    pub fn docs(mut self, docs: impl Into<Cow<'static, str>>) -> Self {
        self.docs = docs.into();
        self
    }

    /// Set deprecation metadata for this type.
    pub fn deprecated(mut self, deprecated: DeprecatedType) -> Self {
        self.deprecated = Some(deprecated);
        self
    }

    /// Mark this named type as always inlined at call sites.
    pub fn inline(mut self) -> Self {
        self.inline = true;
        self
    }

    /// Register the type in `types` and return the resulting [`NamedDataType`].
    #[track_caller]
    pub fn build(self, types: &mut TypeCollection) -> NamedDataType {
        NamedDataType::register(self, types)
    }
}
