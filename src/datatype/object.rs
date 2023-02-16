use crate::DataType;

/// this is used internally to represent the types.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct ObjectField {
    pub key: &'static str,
    pub optional: bool,
    pub flatten: bool,
    pub ty: DataType,
}

/// this is used internally to represent the types.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct ObjectType {
    pub generics: Vec<&'static str>,
    pub fields: Vec<ObjectField>,
    pub tag: Option<&'static str>,
}
