use std::borrow::Cow;

use crate::SpectaID;

use super::{DataType, DeprecatedType, Fields, GenericType, NamedDataType};

/// Enum type which dictates how the enum is represented.
///
/// The tagging refers to the [Serde concept](https://serde.rs/enum-representations.html).
///
/// [`Untagged`](EnumType::Untagged) is here rather than in [`EnumRepr`] as it is the only enum representation that does not have tags on its variants.
/// Separating it allows for better typesafety since `variants` doesn't have to be a [`Vec`] of tuples.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumType {
    pub(crate) name: Cow<'static, str>,
    // Associating a SpectaID will allow exporter to lookup more detailed information about the type to provide better errors.
    pub(crate) sid: Option<SpectaID>,
    // This is used to allow `serde_json::Number` and `toml::Value` to contain BigInt numbers without an error.
    // I don't know if we should block bigints in these any types. Really I think we should but we need a good DX around overriding it on a per-type basis.
    pub(crate) skip_bigint_checks: bool,
    pub(crate) repr: EnumRepr,
    pub(crate) generics: Vec<GenericType>,
    pub(crate) variants: Vec<(Cow<'static, str>, EnumVariant)>,
}

impl EnumType {
    /// Convert a [`EnumType`] to an anonymous [`DataType`].
    pub fn to_anonymous(self) -> DataType {
        DataType::Enum(self)
    }

    /// Convert a [`EnumType`] to a named [`NamedDataType`].
    ///
    /// This can easily be converted to a [`DataType`] by putting it inside the [DataType::Named] variant.
    pub fn to_named(self, name: impl Into<Cow<'static, str>>) -> NamedDataType {
        NamedDataType {
            name: name.into(),
            docs: Cow::Borrowed(""),
            deprecated: None,
            ext: None,
            inner: DataType::Enum(self),
        }
    }

    pub fn sid(&self) -> Option<SpectaID> {
        self.sid
    }

    pub fn name(&self) -> &Cow<'static, str> {
        &self.name
    }

    pub fn repr(&self) -> &EnumRepr {
        &self.repr
    }

    pub fn variants(&self) -> &Vec<(Cow<'static, str>, EnumVariant)> {
        &self.variants
    }

    pub fn generics(&self) -> &Vec<GenericType> {
        &self.generics
    }

    pub fn skip_bigint_checks(&self) -> bool {
        self.skip_bigint_checks
    }
}

impl From<EnumType> for DataType {
    fn from(t: EnumType) -> Self {
        Self::Enum(t)
    }
}

/// Serde representation of an enum.
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
}

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
    pub fn skip(&self) -> bool {
        self.skip
    }

    pub fn docs(&self) -> &Cow<'static, str> {
        &self.docs
    }

    pub fn deprecated(&self) -> Option<&DeprecatedType> {
        self.deprecated.as_ref()
    }

    pub fn fields(&self) -> &Fields {
        &self.fields
    }
}
