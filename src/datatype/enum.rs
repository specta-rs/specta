use crate::datatype::{DataType, ObjectType, TupleType};

/// this is used internally to represent the types.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct EnumType {
    // TODO: Change this so based on `EnumRepr` we either have or don't have a variant name. // TODO
    pub variants: Vec<(&'static str, EnumVariant)>,
    pub generics: Vec<&'static str>,
    pub repr: EnumRepr,
}

impl From<EnumType> for DataType {
    fn from(t: EnumType) -> Self {
        Self::Enum(t)
    }
}

impl EnumType {
    /// An enum may contain variants which are invalid and will cause a runtime errors during serialize/deserialization.
    /// This function will filter them out so types can be exported for valid variants.
    pub fn make_flattenable(&mut self) {
        self.variants.retain(|(_, v)| match self.repr {
            EnumRepr::External => match v {
                EnumVariant::Unnamed(v) if v.fields.len() == 1 => true,
                EnumVariant::Named(_) => true,
                _ => false,
            },
            EnumRepr::Untagged => matches!(v, EnumVariant::Unit | EnumVariant::Named(_)),
            EnumRepr::Adjacent { .. } => true,
            EnumRepr::Internal { .. } => {
                matches!(v, EnumVariant::Unit | EnumVariant::Named(_))
            }
        });
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
            Self::Unit => unreachable!("Unit enum variants have no type!"),
            // TODO: Avoid clone
            Self::Unnamed(tuple_type) => tuple_type.clone().into(),
            Self::Named(object_type) => object_type.clone().into(),
        }
    }
}
