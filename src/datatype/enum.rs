use std::borrow::Cow;

use crate::{datatype::DataType, GenericType, NamedDataType, NamedFields, UnnamedFields};

/// Enum type which dictates how the enum is represented.
///
/// The tagging refers to the [Serde concept](https://serde.rs/enum-representations.html).
///
/// [`Untagged`](EnumType::Untagged) is here rather than in [`EnumRepr`] as it is the only enum representation that does not have tags on its variants.
/// Separating it allows for better typesafety since `variants` doesn't have to be a [`Vec`] of tuples.
#[derive(Debug, Clone, PartialEq)]
pub struct EnumType {
    pub(crate) name: Cow<'static, str>,
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
            comments: vec![],
            deprecated: None,
            ext: None,
            inner: DataType::Enum(self),
        }
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

// TODO: Refactor into this instead
// pub struct EnumVariant {
//     name: Cow<'static, str>,
//     deprecated: Option<DeprecatedTy>,
//     comments: Vec<Cow<'static, str>>,
//     variant: EnumVariantTy,
// }

/// Type of an [`EnumType`] variant.
#[derive(Debug, Clone, PartialEq)]
pub enum EnumVariant {
    /// A unit enum variant
    /// Eg. `Variant`
    Unit,
    /// The enum variant contains named fields.
    /// Eg. `Variant { a: u32 }`
    Named(NamedFields),
    /// The enum variant contains unnamed fields.
    /// Eg. `Variant(u32, String)`
    Unnamed(UnnamedFields),
}
