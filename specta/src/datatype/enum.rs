use std::borrow::Cow;

use crate::datatype::Field;

use super::{Attributes, DataType, Deprecated, Fields, NamedFields, UnnamedFields};

/// represents a Rust [enum](https://doc.rust-lang.org/std/keyword.enum.html).
///
/// Enums are configured with a set of variants, each with a name and a type.
/// The variants can be either unit variants (no fields), tuple variants (fields in a tuple), or struct variants (fields in a struct).
///
/// An enum is also assigned a repr which follows [Serde repr semantics](https://serde.rs/enum-representations.html).
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
pub struct Enum {
    pub(crate) variants: Vec<(Cow<'static, str>, Variant)>,
    pub(crate) attributes: Attributes,
}

impl Enum {
    /// Construct a new empty enum.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get an immutable reference to the enum's variants.
    pub fn variants(&self) -> &[(Cow<'static, str>, Variant)] {
        &self.variants
    }

    /// Get a mutable reference to the enum's variants.
    pub fn variants_mut(&mut self) -> &mut Vec<(Cow<'static, str>, Variant)> {
        &mut self.variants
    }

    /// Get an immutable reference to the enum's attributes.
    pub fn attributes(&self) -> &Attributes {
        &self.attributes
    }

    /// Get a mutable reference to the enum's attributes.
    pub fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.attributes
    }

    /// Check if this enum should be serialized as a string enum.
    /// This is true when all variants are unit variants (no fields).
    pub fn is_string_enum(&self) -> bool {
        self.variants()
            .iter()
            .all(|(_, variant)| matches!(variant.fields(), Fields::Unit))
    }
}

impl From<Enum> for DataType {
    fn from(t: Enum) -> Self {
        Self::Enum(t)
    }
}

/// represents a variant of an enum.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Variant {
    /// Did the user apply a `#[serde(skip)]` or `#[specta(skip)]` attribute.
    ///
    /// You might think, well why not apply this in the macro and just not emit the variant?
    /// Well in Serde `A(String)` and `A(#[serde(skip)] (), String)` export as different Typescript types so the exporter needs runtime knowledge of this.
    pub(crate) skip: bool,
    /// Documentation comments for the field.
    pub(crate) docs: Cow<'static, str>,
    /// Deprecated attribute for the field.
    pub(crate) deprecated: Option<Deprecated>,
    /// The type of the variant.
    pub(crate) fields: Fields,
    /// Runtime attributes for this variant
    pub(crate) attributes: Attributes,
    /// Did the user apply a `#[specta(type = ...)]` or `#[specta(r#type = ...)]` attribute.
    pub(crate) type_overridden: bool,
}

impl Variant {
    /// Construct a new unit enum variant.
    pub fn unit() -> Self {
        Self {
            skip: false,
            docs: "".into(),
            deprecated: None,
            fields: Fields::Unit,
            attributes: Attributes::default(),
            type_overridden: false,
        }
    }

    /// Construct a new struct enum variant with named fields.
    pub fn named() -> VariantBuilder<NamedFields> {
        VariantBuilder {
            v: Self {
                skip: false,
                docs: "".into(),
                deprecated: None,
                fields: Fields::Named(NamedFields {
                    fields: Default::default(),
                }),
                attributes: Attributes::default(),
                type_overridden: false,
            },
            variant: NamedFields { fields: vec![] },
        }
    }

    /// Construct a new tuple enum variant without unnamed fields.
    pub fn unnamed() -> VariantBuilder<UnnamedFields> {
        VariantBuilder {
            v: Self {
                skip: false,
                docs: "".into(),
                deprecated: None,
                fields: Fields::Unnamed(UnnamedFields {
                    fields: Default::default(),
                }),
                attributes: Attributes::default(),
                type_overridden: false,
            },
            variant: UnnamedFields {
                fields: Default::default(),
            },
        }
    }

    /// Has the Serde or Specta skip attribute been applied to this variant?
    pub fn skip(&self) -> bool {
        self.skip
    }

    /// Set the skip attribute for the variant.
    pub fn set_skip(&mut self, skip: bool) {
        self.skip = skip;
    }

    /// Get an immutable reference to the documentation comments for the field.
    pub fn docs(&self) -> &Cow<'static, str> {
        &self.docs
    }

    /// Get a mutable reference to the documentation comments for the variant.
    pub fn docs_mut(&mut self) -> &mut Cow<'static, str> {
        &mut self.docs
    }

    /// Set the documentation comments for the field.
    pub fn set_docs(&mut self, docs: Cow<'static, str>) {
        self.docs = docs;
    }

    /// Get an immutable reference to the deprecated attribute for the field.
    pub fn deprecated(&self) -> Option<&Deprecated> {
        self.deprecated.as_ref()
    }

    /// Get a mutable reference to the deprecated attribute for the field.
    pub fn deprecated_mut(&mut self) -> Option<&mut Deprecated> {
        self.deprecated.as_mut()
    }

    /// Set the deprecated attribute for the field.
    pub fn set_deprecated(&mut self, deprecated: Option<Deprecated>) {
        self.deprecated = deprecated;
    }

    /// Get an immutable reference to the fields of the variant.
    pub fn fields(&self) -> &Fields {
        &self.fields
    }

    /// Get a mutable reference to the fields of the variant.
    pub fn fields_mut(&mut self) -> &mut Fields {
        &mut self.fields
    }

    // No `set_fields` cause builder API is preferred

    /// Get an immutable reference to the runtime attributes for this variant.
    pub fn attributes(&self) -> &Attributes {
        &self.attributes
    }

    /// Mutable reference to the runtime attributes for this variant.
    pub fn attributes_mut(&mut self) -> &mut Attributes {
        &mut self.attributes
    }

    /// Has the Specta type override attribute been applied to this variant?
    pub fn type_overridden(&self) -> bool {
        self.type_overridden
    }

    /// Set whether a Specta type override attribute was applied to this variant.
    pub fn set_type_overridden(&mut self, type_overridden: bool) {
        self.type_overridden = type_overridden;
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
    pub fn deprecated(mut self, reason: Deprecated) -> Self {
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

    /// Set whether the variant has a Specta type override.
    pub fn type_overridden(mut self, type_overridden: bool) -> Self {
        self.v.type_overridden = type_overridden;
        self
    }

    /// Set whether the variant has a Specta type override in-place.
    pub fn type_overridden_mut(&mut self, type_overridden: bool) {
        self.v.type_overridden = type_overridden;
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

    /// Finalize unnamed variant builder into [Variant].
    pub fn build(mut self) -> Variant {
        self.v.fields = Fields::Unnamed(self.variant);
        self.v
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

    /// Finalize named variant builder into [Variant].
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
