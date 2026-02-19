//! Types related to working with [`DataType`]. Exposed for advanced users.

mod attrs;
mod builders;
mod r#enum;
mod fields;
mod function;
mod generic;
mod list;
mod map;
mod named;
mod primitive;
mod reference;
mod r#struct;
mod tuple;

pub use attrs::{RuntimeAttribute, RuntimeLiteral, RuntimeMeta, RuntimeNestedMeta, RuntimeValue};
pub use builders::{NamedDataTypeBuilder, StructBuilder, VariantBuilder};
pub use r#enum::{Enum, EnumVariant};
pub use fields::{
    Field, Fields, NamedFields, NonSkipField, UnnamedFields, skip_fields, skip_fields_named,
};
pub use function::{Function, FunctionReturnType};
pub use generic::{ConstGenericPlaceholder, Generic, GenericPlaceholder};
pub use list::List;
pub use map::Map;
pub use named::{DeprecatedType, NamedDataType};
pub use primitive::Primitive;
pub use reference::{NamedReference, OpaqueReference, Reference};
pub use r#struct::Struct;
pub use tuple::Tuple;

pub(crate) use reference::NamedId;

/// Runtime type-erased representation of a Rust type.
///
/// A language exporter takes this general format and converts it into a language specific syntax.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DataType {
    /// A primitive scalar type like integers, floats, booleans, chars, or strings.
    Primitive(Primitive),
    /// A sequential collection type.
    List(List),
    /// A map/dictionary type.
    Map(Map),
    /// A nullable wrapper around another type.
    Nullable(Box<DataType>),
    /// A struct type with named, unnamed, or unit fields.
    Struct(Struct),
    /// An enum type.
    Enum(Enum),
    /// A tuple type.
    Tuple(Tuple),
    /// A reference to another named or opaque type.
    Reference(Reference),
    /// A generic placeholder type parameter.
    Generic(Generic),
}
