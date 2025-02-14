//! Types related to working with [`DataType`]. Exposed for advanced users.

mod r#enum;
mod fields;
mod function;
mod generic;
mod inline;
mod list;
mod literal;
mod map;
mod named;
mod primitive;
pub mod reference; // TODO
mod r#struct;
mod tuple;

pub use fields::{Field, Fields, NamedFields, UnnamedFields};
pub use function::{Function, FunctionReturnType};
pub use generic::{ConstGenericPlaceholder, Generic, GenericPlaceholder};
pub use inline::{inline, inline_and_flatten, inline_and_flatten_ndt};
pub use list::List;
pub use literal::Literal;
pub use map::Map;
pub use named::{DeprecatedType, NamedDataType};
pub use primitive::Primitive;
pub use r#enum::{Enum, EnumRepr, EnumVariant};
pub use r#struct::Struct;
pub use tuple::Tuple;

/// Runtime type-erased representation of a Rust type.
///
/// A language exporter takes this general format and converts it into a language specific syntax.
#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    // Always inlined
    Primitive(Primitive),
    Literal(Literal),
    /// Either a `Set` or a `Vec`
    List(List),
    Map(Map),
    Nullable(Box<DataType>),
    // Anonymous Reference types
    Struct(Struct),
    Enum(Enum),
    Tuple(Tuple),
    // A reference type that has already been defined
    Reference(reference::Reference),
    Generic(Generic),
}

impl DataType {
    pub fn generics(&self) -> Option<&Vec<Generic>> {
        match self {
            Self::Struct(s) => Some(s.generics()),
            Self::Enum(e) => Some(e.generics()),
            _ => None,
        }
    }
}
