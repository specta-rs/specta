//! Types related to working with [`DataType`]. Exposed for advanced users.

mod attributes;
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

pub use attributes::Attributes;
pub use r#enum::{Enum, Variant, VariantBuilder};
pub use fields::{Field, Fields, NamedFields, StructBuilder, UnnamedFields};
pub use function::Function;
pub use generic::{Generic, GenericDefinition};
pub use list::List;
// pub use literal::Literal;
pub use map::Map;
pub use named::{Deprecated, NamedDataType, inline};
pub use primitive::Primitive;
pub use reference::{NamedReference, NamedReferenceType, OpaqueReference, Reference};
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
    /// A struct type with named, unnamed, or unit fields.
    Struct(Struct),
    /// An enum type.
    Enum(Enum),
    /// A tuple type.
    Tuple(Tuple),
    /// A nullable wrapper around another type.
    Nullable(Box<DataType>),
    /// A placeholder for a generic type defined on the parent [`NamedDataType`].
    /// Rendered as `T`. These should never be returned from [`Type::definition`], they should only appear in [`NamedDataType`]'s `ty` field.
    Generic(Generic),
    /// A reference to another named or opaque type.
    Reference(Reference),
}
