use std::borrow::Cow;

use crate::builder::VariantBuilder;

use super::{DataType, DeprecatedType, Fields, NamedFields, UnnamedFields};

/// represents a Rust [enum](https://doc.rust-lang.org/std/keyword.enum.html).
///
/// Enums are configured with a set of variants, each with a name and a type.
/// The variants can be either unit variants (no fields), tuple variants (fields in a tuple), or struct variants (fields in a struct).
///
/// An enum is also assigned a repr which follows [Serde repr semantics](https://serde.rs/enum-representations.html).
#[derive(Default, Debug, Clone, PartialEq)]
pub struct Enum {
    pub(crate) repr: Option<EnumRepr>,
    pub(crate) variants: Vec<(Cow<'static, str>, EnumVariant)>,
}

impl Enum {
    /// Construct a new empty enum.
    pub fn new() -> Self {
        Self::default()
    }

    /// Get an immutable reference to the enum's representation.
    pub fn repr(&self) -> Option<&EnumRepr> {
        self.repr.as_ref()
    }

    /// Get a mutable reference to the enum's representation.
    pub fn repr_mut(&mut self) -> Option<&mut EnumRepr> {
        self.repr.as_mut()
    }

    /// Set the enum's representation.
    pub fn set_repr(&mut self, repr: EnumRepr) {
        self.repr = Some(repr);
    }

    /// Get an immutable reference to the enum's variants.
    pub fn variants(&self) -> &[(Cow<'static, str>, EnumVariant)] {
        &self.variants
    }

    /// Get a mutable reference to the enum's variants.
    pub fn variants_mut(&mut self) -> &mut Vec<(Cow<'static, str>, EnumVariant)> {
        &mut self.variants
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

/// Serde representation of an enum.
/// Refer to the [Serde documentation](https://serde.rs/enum-representations.html) for more information.
#[derive(Debug, Clone, PartialEq)]
pub enum EnumRepr {
    Untagged,
    External,
    Internal {
        tag: Cow<'static, str>,
    },
    Adjacent {
        tag: Cow<'static, str>,
        content: Cow<'static, str>,
    },
    /// String enum representation for unit-only enums with serde rename_all
    String {
        rename_all: Option<Cow<'static, str>>,
    },
}

impl EnumRepr {
    /// Check if this is a string enum representation
    pub fn is_string(&self) -> bool {
        matches!(self, EnumRepr::String { .. })
    }

    /// Get the rename_all inflection for string enums
    pub fn rename_all(&self) -> Option<&str> {
        match self {
            EnumRepr::String { rename_all } => rename_all.as_deref(),
            _ => None,
        }
    }
}

/// represents a variant of an enum.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariant {
    /// Did the user apply a `#[serde(skip)]` or `#[specta(skip)]` attribute.
    ///
    /// You might think, well why not apply this in the macro and just not emit the variant?
    /// Well in Serde `A(String)` and `A(#[serde(skip)] (), String)` export as different Typescript types so the exporter needs runtime knowledge of this.
    pub(crate) skip: bool,
    /// Documentation comments for the field.
    pub(crate) docs: Cow<'static, str>,
    /// Deprecated attribute for the field.
    pub(crate) deprecated: Option<DeprecatedType>,
    /// The type of the variant.
    pub(crate) fields: Fields,
}

impl EnumVariant {
    /// Construct a new unit enum variant.
    pub fn unit() -> Self {
        Self {
            skip: false,
            docs: "".into(),
            deprecated: None,
            fields: Fields::Unit,
        }
    }

    /// Construct a new struct enum variant with named fields.
    pub fn named() -> VariantBuilder<NamedFields> {
        VariantBuilder {
            v: Self::unit(),
            variant: NamedFields {
                fields: vec![],
                tag: None,
            },
        }
    }

    /// Construct a new tuple enum variant without unnamed fields.
    pub fn unnamed() -> VariantBuilder<UnnamedFields> {
        VariantBuilder {
            v: Self::unit(),
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
    pub fn deprecated(&self) -> Option<&DeprecatedType> {
        self.deprecated.as_ref()
    }

    /// Get a mutable reference to the deprecated attribute for the field.
    pub fn deprecated_mut(&mut self) -> Option<&mut DeprecatedType> {
        self.deprecated.as_mut()
    }

    /// Set the deprecated attribute for the field.
    pub fn set_deprecated(&mut self, deprecated: Option<DeprecatedType>) {
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

    /// Set the fields of the variant.
    pub fn set_fields(&mut self, fields: Fields) {
        self.fields = fields;
    }
}
