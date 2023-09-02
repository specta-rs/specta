use std::borrow::Cow;

use crate::{
    datatype::DataType, ExportError, GenericType, ImplLocation, NamedDataType, StructNamedFields,
    StructUnnamedFields,
};

#[derive(Debug, Clone, PartialEq)]
pub struct UntaggedEnum {
    pub(crate) variants: Vec<EnumVariant>,
    pub(crate) generics: Vec<GenericType>,
}

impl UntaggedEnum {
    pub fn variants(&self) -> impl Iterator<Item = &EnumVariant> {
        self.variants.iter()
    }

    pub fn generics(&self) -> impl Iterator<Item = &GenericType> {
        self.generics.iter()
    }
}

impl Into<EnumType> for UntaggedEnum {
    fn into(self) -> EnumType {
        EnumType::Untagged(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TaggedEnum {
    pub(crate) variants: Vec<(Cow<'static, str>, EnumVariant)>,
    pub(crate) generics: Vec<GenericType>,
    pub(crate) repr: EnumRepr,
}

impl TaggedEnum {
    pub fn variants(&self) -> impl Iterator<Item = &(Cow<'static, str>, EnumVariant)> {
        self.variants.iter()
    }

    pub fn generics(&self) -> impl Iterator<Item = &GenericType> {
        self.generics.iter()
    }

    pub fn repr(&self) -> &EnumRepr {
        &self.repr
    }
}

impl Into<EnumType> for TaggedEnum {
    fn into(self) -> EnumType {
        EnumType::Tagged(self)
    }
}

/// Enum type which dictates how the enum is represented.
///
/// The tagging refers to the [Serde concept](https://serde.rs/enum-representations.html).
///
/// [`Untagged`](EnumType::Untagged) is here rather than in [`EnumRepr`] as it is the only enum representation that does not have tags on its variants.
/// Separating it allows for better typesafety since `variants` doesn't have to be a [`Vec`] of tuples.
#[derive(Debug, Clone, PartialEq)]
pub enum EnumType {
    Untagged(UntaggedEnum),
    Tagged(TaggedEnum),
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

    pub fn generics(&self) -> &Vec<GenericType> {
        match self {
            Self::Untagged(UntaggedEnum { generics, .. }) => generics,
            Self::Tagged(TaggedEnum { generics, .. }) => generics,
        }
    }

    pub(crate) fn variants_len(&self) -> usize {
        match self {
            Self::Untagged(UntaggedEnum { variants, .. }) => variants.len(),
            Self::Tagged(TaggedEnum { variants, .. }) => variants.len(),
        }
    }

    /// An enum may contain variants which are invalid and will cause a runtime errors during serialize/deserialization.
    /// This function will filter them out so types can be exported for valid variants.
    pub fn make_flattenable(&mut self, impl_location: ImplLocation) -> Result<(), ExportError> {
        match self {
            Self::Untagged(UntaggedEnum { variants, .. }) => {
                variants.iter().try_for_each(|v| match v {
                    EnumVariant::Unit => Ok(()),
                    EnumVariant::Named(_) => Ok(()),
                    EnumVariant::Unnamed(_) => Err(ExportError::InvalidType(
                        impl_location,
                        "`EnumRepr::Untagged` with `EnumVariant::Unnamed` is invalid!",
                    )),
                })?;
            }
            Self::Tagged(TaggedEnum { variants, repr, .. }) => {
                variants.iter().try_for_each(|(_, v)| {
                    match repr {
                        EnumRepr::External => match v {
                            EnumVariant::Unit => Err(ExportError::InvalidType(
                                impl_location,
                                "`EnumRepr::External` with ` EnumVariant::Unit` is invalid!",
                            )),
                            EnumVariant::Unnamed(v) => match v {
                                StructUnnamedFields { fields, .. } if fields.len() == 1 => Ok(()),
                                StructUnnamedFields { .. } => Err(ExportError::InvalidType(
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
///
/// Does not contain [`Untagged`](EnumType::Untagged) as that is handled by [`EnumType`].
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub enum EnumRepr {
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
#[allow(missing_docs)]
pub enum EnumVariant {
    Unit,
    // TODO: Should these be holding the `struct` types or have their own???
    Named(StructNamedFields),
    Unnamed(StructUnnamedFields),
}

impl EnumVariant {
    /// Get the [`DataType`](crate::DataType) of the variant.
    pub fn data_type(&self) -> DataType {
        match self {
            Self::Unit => unreachable!("Unit enum variants have no type!"), // TODO: Remove unreachable in type system + avoid following clones
            Self::Unnamed(tuple_type) => DataType::Struct(tuple_type.clone().into()),
            Self::Named(object_type) => DataType::Struct(object_type.clone().into()),
        }
    }
}
