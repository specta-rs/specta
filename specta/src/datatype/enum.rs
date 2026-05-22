use std::borrow::Cow;

use crate::datatype::Field;

use super::{Attributes, DataType, Deprecated, Fields, NamedFields, UnnamedFields};

/// Runtime representation of a Rust [`enum`](https://doc.rust-lang.org/std/keyword.enum.html).
///
/// Enums are configured with a set of variants, each with a name and a type.
/// The variants can be either unit variants (no fields), tuple variants (fields in a tuple), or struct variants (fields in a struct).
///
/// Each variant has a name and one of the layouts described by [`Fields`].
/// Format integrations may use [`Enum::attributes`] to record representation
/// metadata, such as Serde's enum representation attributes.
#[derive(Default, Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Enum {
    /// Named variants in source order.
    pub variants: Vec<(Cow<'static, str>, Variant)>,
    /// Macro attributes applied to the enum container.
    pub attributes: Attributes,
}

impl From<Enum> for DataType {
    fn from(t: Enum) -> Self {
        Self::Enum(t)
    }
}

/// Runtime representation of a single enum variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Variant {
    /// Whether the variant was skipped with an attribute such as
    /// `#[serde(skip)]` or `#[specta(skip)]`.
    ///
    /// You might think, well why not apply this in the macro and just not emit the variant?
    /// Well in Serde `A(String)` and `A(#[serde(skip)] (), String)` export as different Typescript types so the exporter needs runtime knowledge of this.
    pub skip: bool,
    /// Documentation comments for the variant.
    pub docs: Cow<'static, str>,
    /// Deprecated metadata for the variant.
    pub deprecated: Option<Deprecated>,
    /// The type of the variant.
    pub fields: Fields,
    /// Runtime attributes for this variant.
    pub attributes: Attributes,
}

impl Variant {
    /// Constructs a unit enum variant.
    pub fn unit() -> Self {
        Self {
            skip: false,
            docs: "".into(),
            deprecated: None,
            fields: Fields::Unit,
            attributes: Attributes::default(),
        }
    }

    /// Starts building a struct enum variant with named fields.
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

    /// Starts building a tuple enum variant with unnamed fields.
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
    /// Marks the variant as skipped.
    pub fn skip(mut self) -> Self {
        self.v.skip = true;
        self
    }

    /// Sets documentation for the variant.
    pub fn docs(mut self, docs: Cow<'static, str>) -> Self {
        self.v.docs = docs;
        self
    }

    /// Sets deprecation metadata for the variant.
    pub fn deprecated(mut self, reason: Deprecated) -> Self {
        self.v.deprecated = Some(reason);
        self
    }

    /// Sets runtime attributes on the variant.
    pub fn attributes(mut self, attributes: Attributes) -> Self {
        self.v.attributes = attributes;
        self
    }

    /// Sets runtime attributes on the variant in-place.
    pub fn attributes_mut(&mut self, attributes: Attributes) {
        self.v.attributes = attributes;
    }
}

impl VariantBuilder<UnnamedFields> {
    /// Adds an unnamed field to the variant.
    pub fn field(mut self, field: Field) -> Self {
        self.variant.fields.push(field);
        self
    }

    /// Adds an unnamed field to the variant and returns the updated builder.
    pub fn field_mut(mut self, field: Field) -> Self {
        self.variant.fields.push(field);
        self
    }

    /// Finalizes the unnamed variant builder into a [`Variant`].
    pub fn build(mut self) -> Variant {
        self.v.fields = Fields::Unnamed(self.variant);
        self.v
    }
}

impl VariantBuilder<NamedFields> {
    /// Adds a named field to the variant.
    pub fn field(mut self, name: impl Into<Cow<'static, str>>, field: Field) -> Self {
        self.variant.fields.push((name.into(), field));
        self
    }

    /// Adds a named field to the variant and returns the updated builder.
    pub fn field_mut(mut self, name: impl Into<Cow<'static, str>>, field: Field) -> Self {
        self.variant.fields.push((name.into(), field));
        self
    }

    /// Finalizes the named variant builder into a [`Variant`].
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
