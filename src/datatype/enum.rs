use crate::{
    datatype::{DataType, ObjectType, TupleType},
    ExportError,
};

/// this is used internally to represent the types.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub enum EnumType {
    Untagged {
        variants: Vec<EnumVariant>,
        generics: Vec<&'static str>,
        repr: EnumRepr,
    },
    Tagged {
        variants: Vec<(&'static str, EnumVariant)>,
        generics: Vec<&'static str>,
        repr: EnumRepr,
    },
}

impl From<EnumType> for DataType {
    fn from(t: EnumType) -> Self {
        Self::Enum(t)
    }
}

impl EnumType {
    pub(crate) fn generics(&self) -> &Vec<&'static str> {
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
    pub fn make_flattenable(&mut self) -> Result<(), ExportError> {
        match self {
            Self::Untagged { variants, repr, .. } => {
                variants
                    .iter()
                    .map(|variant| Self::make_flattenable_inner(variant, repr))
                    .collect::<Result<_, _>>()?;
            }
            Self::Tagged { variants, repr, .. } => {
                variants
                    .iter()
                    .map(|(_, variant)| Self::make_flattenable_inner(variant, repr))
                    .collect::<Result<_, _>>()?;
            }
        }

        Ok(())
    }

    fn make_flattenable_inner(v: &EnumVariant, repr: &EnumRepr) -> Result<(), ExportError> {
        match repr {
            EnumRepr::External => match v {
                EnumVariant::Unit => Err(ExportError::InvalidType(
                    "`EnumRepr::External` with ` EnumVariant::Unit` is invalid!",
                )),
                EnumVariant::Unnamed(v) if v.fields.len() == 1 => Ok(()),
                EnumVariant::Unnamed(_) => Err(ExportError::InvalidType(
                    "`EnumRepr::External` with ` EnumVariant::Unnamed` containing more than a single field is invalid!",
                )),
                EnumVariant::Named(_) => Ok(()),
            },
            EnumRepr::Untagged => match v {
                EnumVariant::Unit => Ok(()),
                EnumVariant::Named(_) => Ok(()),
                EnumVariant::Unnamed(_) => Err(ExportError::InvalidType(
                    "`EnumRepr::Untagged` with ` EnumVariant::Unnamed` is invalid!",
                )),
            },
            EnumRepr::Adjacent { .. } => Ok(()),
            EnumRepr::Internal { .. } => match v {
                EnumVariant::Unit => Ok(()),
                EnumVariant::Named(_) => Ok(()),
                EnumVariant::Unnamed(_) => Err(ExportError::InvalidType(
                    "`EnumRepr::Internal` with ` EnumVariant::Unnamed` is invalid!",
                )),
            },
        }
    }
}

/// this is used internally to represent the types.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub enum EnumRepr {
    External,
    Internal {
        tag: &'static str,
    },
    Adjacent {
        tag: &'static str,
        content: &'static str,
    },
    Untagged,
}

/// this is used internally to represent the types.
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
            Self::Unit => unreachable!("Unit enum variants have no type!"), // TODO: Remove unreachable in type system
            // TODO: Avoid clone
            Self::Unnamed(tuple_type) => tuple_type.clone().into(),
            Self::Named(object_type) => object_type.clone().into(),
        }
    }
}
