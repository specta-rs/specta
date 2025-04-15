use std::borrow::Cow;

use crate::SpectaID;

use super::{DataType, DeprecatedType, Fields};

/// Enum type which dictates how the enum is represented.
///
/// The tagging refers to the [Serde concept](https://serde.rs/enum-representations.html).
///
/// [`Untagged`](Enum::Untagged) is here rather than in [`EnumRepr`] as it is the only enum representation that does not have tags on its variants.
/// Separating it allows for better typesafety since `variants` doesn't have to be a [`Vec`] of tuples.
#[derive(Debug, Clone, PartialEq)]
pub struct Enum {
    pub(crate) name: Cow<'static, str>,
    // Associating a SpectaID will allow exporter to lookup more detailed information about the type to provide better errors.
    pub(crate) sid: Option<SpectaID>,
    // This is used to allow `serde_json::Number` and `toml::Value` to contain BigInt numbers without an error.
    // I don't know if we should block bigints in these any types. Really I think we should but we need a good DX around overriding it on a per-type basis.
    pub(crate) skip_bigint_checks: bool,
    pub(crate) repr: EnumRepr,
    pub(crate) variants: Vec<(Cow<'static, str>, EnumVariant)>,
}

impl Enum {
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

    pub fn skip_bigint_checks(&self) -> bool {
        self.skip_bigint_checks
    }
}

impl From<Enum> for DataType {
    fn from(t: Enum) -> Self {
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
