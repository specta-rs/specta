use std::collections::BTreeMap;

mod r#enum;
mod literal;
mod object;
mod primitive;
mod tuple;

pub use literal::*;
pub use object::*;
pub use primitive::*;
pub use r#enum::*;
pub use tuple::*;

use crate::{ImplLocation, TypeSid};

/// A map of type definitions
pub type TypeDefs = BTreeMap<TypeSid, DataType>;

/// arguments for [Type::inline](crate::Type::inline), [Type::reference](crate::Type::reference) and [Type::definition](crate::Type::definition).
pub struct DefOpts<'a> {
    /// is the parent type inlined?
    pub parent_inline: bool,
    /// a map of types which have been visited. This prevents stack overflows when a type references itself and also allows the caller to get a list of all types in the "schema".
    pub type_map: &'a mut TypeDefs,
}

/// A wrapper around [DataTypeItem] to store general information about the type.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub enum DataType {
    // Always inlined
    Any,
    Primitive(PrimitiveType),
    Literal(LiteralType),
    List(Box<DataType>),
    Nullable(Box<DataType>),
    Record(Box<(DataType, DataType)>),
    // Named reference types
    Named(NamedDataType),
    // Anonymous Reference types
    Object(ObjectType),
    Enum(EnumType),
    Tuple(TupleType),
    // A reference type that has already been defined
    Reference(DataTypeReference),
    Generic(GenericType),
    /// Used when the type is not yet known. This allows us to avoid stack overflows.
    /// It should never be returned from the Specta functions. Doing so is classed as a bug!
    Placeholder,
}

/// TODO
#[derive(Debug, Clone, PartialEq)]
pub struct NamedDataType {
    /// The name of the type
    pub name: &'static str,
    /// The Specta ID for the type. The value for this should come from the `sid!();` macro.
    pub sid: Option<TypeSid>,
    /// The code location where this type is implemented. Used for error reporting.
    pub impl_location: Option<ImplLocation>,
    /// Rust documentation comments on the type
    pub comments: &'static [&'static str],
    /// Whether the type should export when the `export` feature is enabled.
    /// `None` will use the default which is why `false` is not just used.
    pub export: Option<bool>,
    /// The Rust deprecated comment if the type is deprecated.
    pub deprecated: Option<&'static str>,
    pub item: NamedDataTypeItem,
}

impl From<NamedDataType> for DataType {
    fn from(t: NamedDataType) -> Self {
        Self::Named(t)
    }
}

/// TODO
#[derive(Debug, Clone, PartialEq)]
pub enum NamedDataTypeItem {
    Object(ObjectType),
    Enum(EnumType),
    Tuple(TupleType),
}

/// Datatype to be put in the type map while field types are being resolved. Used in order to
/// support recursive types without causing an infinite loop.
///
/// This works since a child type that references a parent type does not care about the
/// parent's fields, only really its name. Once all of the parent's fields have been
/// resolved will the parent's definition be placed in the type map.
///
/// This doesn't account for flattening and inlining recursive types, however, which will
/// require a more complex solution since it will require multiple processing stages.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct DataTypeReference {
    pub name: &'static str,
    pub sid: TypeSid,
    pub generics: Vec<DataType>,
}

/// this is used internally to represent the types.
#[derive(Debug, Clone, PartialEq)]
#[allow(missing_docs)]
pub struct GenericType(pub &'static str);

impl From<GenericType> for DataType {
    fn from(t: GenericType) -> Self {
        Self::Generic(t)
    }
}

impl<T: Into<DataType> + 'static> From<Vec<T>> for DataType {
    fn from(t: Vec<T>) -> Self {
        DataType::Enum(EnumType {
            variants: t
                .into_iter()
                .map(|t| {
                    (
                        "This is never used before of EnumRepr::Untagged", // TODO: Fix this!
                        EnumVariant::Unnamed(TupleType {
                            fields: vec![t.into()],
                            generics: vec![],
                        }),
                    )
                })
                .collect(),
            generics: vec![],
            repr: EnumRepr::Untagged,
        })
    }
}

impl<T: Into<DataType> + 'static> From<Option<T>> for DataType {
    fn from(t: Option<T>) -> Self {
        t.map(Into::into)
            .unwrap_or_else(|| LiteralType::None.into())
    }
}

impl<'a> From<&'a str> for DataType {
    fn from(t: &'a str) -> Self {
        LiteralType::String(t.to_string()).into()
    }
}

impl From<String> for DataType {
    fn from(t: String) -> Self {
        LiteralType::String(t).into()
    }
}

pub trait ToSpectaType {
    type Type;

    fn to_specta_type(self) -> Self::Type;
}
