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
#[non_exhaustive]
pub struct Enum {
    /// Enums named variants
    pub variants: Vec<(Cow<'static, str>, Variant)>,
    /// Macro attributes applied to the enum container.
    pub attributes: Attributes,
}

impl From<Enum> for DataType {
    fn from(t: Enum) -> Self {
        Self::Enum(t)
    }
}

/// represents a variant of an enum.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Variant {
    /// Did the user apply a `#[serde(skip)]` or `#[specta(skip)]` attribute.
    ///
    /// You might think, well why not apply this in the macro and just not emit the variant?
    /// Well in Serde `A(String)` and `A(#[serde(skip)] (), String)` export as different Typescript types so the exporter needs runtime knowledge of this.
    pub skip: bool,
    /// Documentation comments for the field.
    pub docs: Cow<'static, str>,
    /// Deprecated attribute for the field.
    pub deprecated: Option<Deprecated>,
    /// The type of the variant.
    pub fields: Fields,
    /// Runtime attributes for this variant
    pub attributes: Attributes,
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
            },
            variant: UnnamedFields {
                fields: Default::default(),
            },
        }
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
