use crate::{datatype::DataType, SpectaID, TypeCollection};

mod impls;
mod macros;
// TODO: We don't care much about these cause they are gonna go so this will do for now.
#[cfg(feature = "derive")]
mod legacy_impls;

/// Provides runtime type information that can be fed into a language exporter to generate a type definition in another language.
/// Avoid implementing this trait yourself where possible and use the [`Type`](derive@crate::Type) macro instead.
///
/// This should be only implemented via the [`Type`](derive@crate::Type) macro.
/// TODO: Discuss how to avoid custom implementations.
pub trait Type {
    /// returns a [`DataType`](crate::datatype::DataType) that represents the type.
    /// This will also register any dependent types into the [`TypeCollection`].
    fn definition(types: &mut TypeCollection) -> DataType;
}

/// represents a type that can be converted into [`NamedDataType`](crate::NamedDataType).
/// This will be implemented for all types with the [Type] derive macro.
///
/// TODO: Discuss which types this should be implemented for.
///
/// This should be only implemented via the [`Type`](derive@crate::Type) macro.
pub trait NamedType: Type {
    const ID: SpectaID;
}

/// A marker trait for compile-time validation of which types can be flattened.
pub trait Flatten: Type {}
