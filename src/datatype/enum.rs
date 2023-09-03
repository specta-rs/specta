use std::borrow::Cow;

use crate::{
    datatype::DataType, ExportError, GenericType, ImplLocation, NamedDataType, StructNamedFields,
    StructUnnamedFields,
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
    pub(crate) taging: EnumTag,
    pub(crate) variants: Vec<EnumVariant>,
    pub(crate) generics: Vec<GenericType>,
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

    pub fn taging(&self) -> &EnumTag {
        &self.taging
    }

    pub fn variants(&self) -> &Vec<EnumVariant> {
        &self.variants
    }

    pub fn generics(&self) -> &Vec<GenericType> {
        &self.generics
    }

    /// An enum may contain variants which are invalid and will cause a runtime errors during serialize/deserialization.
    /// This function will filter them out so types can be exported for valid variants.
    pub fn make_flattenable(&mut self, impl_location: ImplLocation) -> Result<(), ExportError> {
        match &self.taging {
            EnumTag::Untagged => {
                self.variants.iter().try_for_each(|v| match v {
                    EnumVariant::Unit(_) => Ok(()),
                    EnumVariant::Named(_) => Ok(()),
                    EnumVariant::Unnamed(_) => Err(ExportError::InvalidType(
                        impl_location,
                        "`EnumRepr::Untagged` with `EnumVariant::Unnamed` is invalid!",
                    )),
                })?;
            }
            EnumTag::Tagged(repr) => {
                self.variants.iter().try_for_each(|v| {
                    match repr {
                        EnumRepr::External => match v {
                            EnumVariant::Unit (_)=> Err(ExportError::InvalidType(
                                impl_location,
                                "`EnumRepr::External` with ` EnumVariant::Unit` is invalid!",
                            )),
                            EnumVariant::Unnamed(v) => match v {
                                EnumUnnamedFields { fields, .. } if fields.len() == 1 => Ok(()),
                                EnumUnnamedFields { .. } => Err(ExportError::InvalidType(
                                    impl_location,
                                    "`EnumRepr::External` with `EnumVariant::Unnamed` containing more than a single field is invalid!",
                                )),
                            },
                            EnumVariant::Named(_) => Ok(()),
                        },
                        EnumRepr::Adjacent { .. } => Ok(()),
                        EnumRepr::Internal { .. } => match v {
                            EnumVariant::Unit(_) => Ok(()),
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

#[derive(Debug, Clone, PartialEq)]
pub enum EnumTag {
    Untagged,
    Tagged(EnumRepr),
}

/// Type of an [`EnumType`] variant.
#[derive(Debug, Clone, PartialEq)]
pub enum EnumVariant {
    Unit(Cow<'static, str>),
    Named(StructNamedFields),
    Unnamed(EnumUnnamedFields),
}

// TODO: Should this be the case or should it be it's own type???
pub type EnumNamedFields = StructNamedFields;

#[derive(Debug, Clone, PartialEq)]
pub struct EnumUnnamedFields {
    pub(crate) name: Cow<'static, str>,
    pub(crate) fields: Vec<DataType>, // TODO: should use `StructField` but without `name` for flatten/inline
    pub(crate) generics: Vec<GenericType>,
}

impl EnumUnnamedFields {
    pub fn generics(&self) -> impl Iterator<Item = &GenericType> {
        self.generics.iter()
    }

    // TODO: Make this work
    // pub fn fields(&self) -> impl Iterator<Item = &StructField> {
    //     self.fields.iter()
    // }
}

impl EnumVariant {
    pub fn name(&self) -> &Cow<'static, str> {
        match self {
            Self::Unit(name) => &name,
            Self::Unnamed(v) => &v.name,
            Self::Named(v) => &v.name,
        }
    }
}
