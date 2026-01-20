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

pub use attrs::{RuntimeAttribute, RuntimeLiteral, RuntimeMeta, RuntimeNestedMeta};
pub use builders::{NamedDataTypeBuilder, StructBuilder, VariantBuilder};
pub use fields::{
    skip_fields, skip_fields_named, Field, Fields, NamedFields, NonSkipField, UnnamedFields,
};
pub use function::{Function, FunctionReturnType};
pub use generic::{ConstGenericPlaceholder, Generic, GenericPlaceholder};
pub use list::List;
pub use map::Map;
pub use named::{DeprecatedType, NamedDataType};
pub use primitive::Primitive;
pub use r#enum::{Enum, EnumVariant};
pub use r#struct::Struct;
pub use reference::{ArcId, Reference};
pub use tuple::Tuple;

/// Runtime type-erased representation of a Rust type.
///
/// A language exporter takes this general format and converts it into a language specific syntax.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DataType {
    Primitive(Primitive),
    List(List),
    Map(Map),
    Nullable(Box<DataType>),
    Struct(Struct),
    Enum(Enum),
    Tuple(Tuple),
    Reference(Reference),
    Generic(Generic),
}
