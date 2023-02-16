use crate::datatype::{DataType, ObjectType, TupleType};

/// this is used internally to represent the types.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct EnumType {
    pub variants: Vec<EnumVariant>,
    pub generics: Vec<&'static str>,
    pub repr: EnumRepr,
}

impl EnumType {
    /// An enum may contain variants which are invalid and will cause a runtime errors during serialize/deserialization.
    /// This function will filter them out so types can be exported for valid variants.
    pub fn make_flattenable(&mut self) {
        self.variants.retain(|v| match self.repr {
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
    Unit, // (&'static str),
    Unnamed(TupleType),
    Named(ObjectType),
}

impl EnumVariant {
    // /// Get the name of the variant.
    // pub fn name(&self, ty_name: &'static str) -> &'static str {
    //     match self {
    //         Self::Unit(name) => name,
    //         Self::Unnamed(tuple_type) => tuple_type.name,
    //         Self::Named(object_type) => object_type.name,
    //     }
    //     todo!();
    // }

    /// Get the [`DataType`](crate::DataType) of the variant.
    pub fn data_type(&self) -> DataType {
        match self {
            Self::Unit => unreachable!("Unit enum variants have no type!"),
            Self::Unnamed(tuple_type) => tuple_type.clone().into(),
            Self::Named(object_type) => todo!(), // object_type.clone().into(),
        }
    }
}
