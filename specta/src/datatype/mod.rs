//! Types related to working with [`DataType`]. Exposed for advanced users.

mod r#enum;
mod fields;
mod function;
mod generic;
mod list;
mod literal;
mod map;
mod named;
mod primitive;
mod reference;
mod r#struct;
mod tuple;

pub use fields::{Field, Fields, NamedFields, UnnamedFields};
pub use function::{Function, FunctionReturnType};
pub use generic::{ConstGenericPlaceholder, Generic, GenericPlaceholder};
pub use list::List;
pub use literal::Literal;
pub use map::Map;
pub use named::{DeprecatedType, NamedDataType};
pub use primitive::Primitive;
pub use r#enum::{Enum, EnumRepr, EnumVariant};
pub use r#struct::Struct;
pub use reference::Reference;
pub use tuple::Tuple;

/// Runtime type-erased representation of a Rust type.
///
/// A language exporter takes this general format and converts it into a language specific syntax.
#[derive(Debug, Clone, PartialEq)]
pub enum DataType {
    Primitive(Primitive),
    Literal(Literal),
    List(List),
    Map(Map),
    Nullable(Box<DataType>),
    Struct(Struct),
    Enum(Enum),
    Tuple(Tuple),
    Reference(reference::Reference),
    Generic(Generic),
}
