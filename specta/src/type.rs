use crate::{
    datatype::{
        reference::Reference,
        DataType
    },
    TypeCollection,
};

mod impls;
mod macros;
// TODO: We don't care much about these cause they are gonna go so this will do for now.
#[cfg(feature = "derive")]
mod legacy_impls;

/// Provides runtime type information that can be fed into a language exporter to generate a type definition in another language.
/// Avoid implementing this trait yourself where possible and use the [`Type`](derive@crate::Type) macro instead.
///
/// This should be only implemented via the [`Type`](derive@crate::Type) macro.
pub trait Type {
    /// TODO
    fn definition(type_map: &mut TypeCollection) -> DataType;
}

/// represents a type that can be converted into [NamedDataType].
/// This will be implemented for all types with the [Type] derive macro.
///
/// This should be only implemented via the [`Type`](derive@crate::Type) macro.
pub trait NamedType: Type {
    /// TODO
    fn reference(type_map: &mut TypeCollection) -> Reference;
}

/// A marker trait for compile-time validation of which types can be flattened.
pub trait Flatten: Type {}
