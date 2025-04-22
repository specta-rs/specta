use std::borrow::Cow;

use super::{DataType, DeprecatedType, Fields};

/// Enum type which dictates how the enum is represented.
///
/// The tagging refers to the [Serde concept](https://serde.rs/enum-representations.html).
///
/// [`Untagged`](Enum::Untagged) is here rather than in [`EnumRepr`] as it is the only enum representation that does not have tags on its variants.
/// Separating it allows for better typesafety since `variants` doesn't have to be a [`Vec`] of tuples.
#[derive(Debug, Clone, PartialEq)]
pub struct Enum {
    pub(crate) repr: EnumRepr,
    pub(crate) variants: Vec<(Cow<'static, str>, EnumVariant)>,
}

impl Enum {
    pub fn repr(&self) -> &EnumRepr {
        &self.repr
    }

    pub fn variants(&self) -> &Vec<(Cow<'static, str>, EnumVariant)> {
        &self.variants
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
