use std::borrow::Cow;

use crate::{
    datatype::{DataType, ObjectType, TupleType},
    ExportError, ImplLocation,
};

/// Enum type which dictates how the enum is represented.
///
/// The tagging refers to the [Serde concept](https://serde.rs/enum-representations.html).
///
/// [`Untagged`](EnumType::Untagged) is here rather than in [`EnumRepr`] as it is the only enum representation that does not have tags on its variants.
/// Separating it allows for better typesafety since `variants` doesn't have to be a [`Vec`] of tuples.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub enum EnumType {
    Untagged {
        variants: Vec<EnumVariant>,
        generics: Vec<Cow<'static, str>>,
    },
    Tagged {
        variants: Vec<(Cow<'static, str>, EnumVariant)>,
        generics: Vec<Cow<'static, str>>,
        repr: EnumRepr,
    },
}

impl From<EnumType> for DataType {
    fn from(t: EnumType) -> Self {
        Self::Enum(t)
    }
}

impl EnumType {
    pub(crate) fn generics(&self) -> &Vec<Cow<'static, str>> {
        match self {
            Self::Untagged { generics, .. } => generics,
            Self::Tagged { generics, .. } => generics,
        }
    }

    pub(crate) fn variants_len(&self) -> usize {
        match self {
            Self::Untagged { variants, .. } => variants.len(),
            Self::Tagged { variants, .. } => variants.len(),
        }
    }

    /// An enum may contain variants which are invalid and will cause a runtime errors during serialize/deserialization.
    /// This function will filter them out so types can be exported for valid variants.
    pub fn make_flattenable(&mut self, impl_location: ImplLocation) -> Result<(), ExportError> {
        match self {
            Self::Untagged { variants, .. } => {
                variants.iter().try_for_each(|v| match v {
                    EnumVariant::Unit => Ok(()),
                    EnumVariant::Named(_) => Ok(()),
                    EnumVariant::Unnamed(_) => Err(ExportError::InvalidType(
                        impl_location,
                        "`EnumRepr::Untagged` with ` EnumVariant::Unnamed` is invalid!",
                    )),
                })?;
            }
            Self::Tagged { variants, repr, .. } => {
                variants.iter().try_for_each(|(_, v)| {
                    match repr {
                        EnumRepr::External => match v {
                            EnumVariant::Unit => Err(ExportError::InvalidType(
                                impl_location,
                                "`EnumRepr::External` with ` EnumVariant::Unit` is invalid!",
                            )),
                            EnumVariant::Unnamed(v) => match v {
                                TupleType::Unnamed =>  Ok(()),
                                TupleType::Named { fields, .. } if fields.len() == 1 => Ok(()),
                                TupleType::Named { .. } => Err(ExportError::InvalidType(
                                    impl_location,
                                    "`EnumRepr::External` with ` EnumVariant::Unnamed` containing more than a single field is invalid!",
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
                                "`EnumRepr::Internal` with ` EnumVariant::Unnamed` is invalid!",
                            )),
                        },
                    }
                })?;
            }
        }

        Ok(())
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
    Named(ObjectType),
    Unnamed(TupleType),
}

impl EnumVariant {
    /// Get the [`DataType`](crate::DataType) of the variant.
    pub fn data_type(&self) -> DataType {
        match self {
            Self::Unit => unreachable!("Unit enum variants have no type!"), // TODO: Remove unreachable in type system + avoid following clones
            Self::Unnamed(tuple_type) => tuple_type.clone().into(),
            Self::Named(object_type) => object_type.clone().into(),
        }
    }
}
