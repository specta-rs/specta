use std::borrow::Cow;

use crate::{
    datatype::DataType, ExportError, GenericType, ImplLocation, NamedDataType, NamedFields,
    UnnamedFields,
};

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
            item: DataType::Enum(self),
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

    /// An enum may contain variants which are invalid and will cause a runtime errors during serialize/deserialization.
    /// This function will filter them out so types can be exported for valid variants.
    pub fn make_flattenable(&mut self, impl_location: ImplLocation) -> Result<(), ExportError> {
        match &self.repr {
            EnumRepr::Untagged => {
                self.variants.iter().try_for_each(|(_, v)| match v {
                    EnumVariant::Unit => Ok(()),
                    EnumVariant::Named(_) => Ok(()),
                    EnumVariant::Unnamed(_) => Err(ExportError::InvalidType(
                        impl_location,
                        "`EnumRepr::Untagged` with `EnumVariant::Unnamed` is invalid!",
                    )),
                })?;
            }
            repr => {
                self.variants.iter().try_for_each(|(_, v)| {
                    match repr {
                        EnumRepr::Untagged => Ok(()),
                        EnumRepr::External => match v {
                            EnumVariant::Unit => Err(ExportError::InvalidType(
                                impl_location,
                                "`EnumRepr::External` with ` EnumVariant::Unit` is invalid!",
                            )),
                            EnumVariant::Unnamed(v) => match v {
                                UnnamedFields { fields, .. } if fields.len() == 1 => Ok(()),
                                UnnamedFields { .. } => Err(ExportError::InvalidType(
                                    impl_location,
                                    "`EnumRepr::External` with `EnumVariant::Unnamed` containing more than a single field is invalid!",
                                )),
                            },
                            EnumVariant::Named(_) => Ok(()),
                        },
                        EnumRepr::Adjacent { .. } => Ok(()),
                        EnumRepr::Internal { .. } => match v {
                            EnumVariant::Unit => Ok(()),
                            EnumVariant::Named(_) => Ok(()),
                            EnumVariant::Unnamed(_) => Err(ExportError::InvalidType(
                                impl_location,
                                "`EnumRepr::Internal` with `EnumVariant::Unnamed` is invalid!",
                            )),
                        },
                    }
                })?;
            }
        }

        Ok(())
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
