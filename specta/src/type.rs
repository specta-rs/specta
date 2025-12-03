use crate::{SpectaID, TypeCollection, datatype::DataType};

mod impls;
mod macros;
// TODO: We don't care much about these cause they are gonna go so this will do for now.
#[cfg(feature = "derive")]
mod legacy_impls;

/// Provides runtime type information that can be fed into a language exporter to generate a type definition for another language.
/// Avoid implementing this trait yourself where possible and use the [`Type`](derive@crate::Type) macro instead.
///
/// This should be implemented by the [`Type`](derive@crate::Type) macro.
/// TODO: Discuss how to avoid custom implementations.
pub trait Type {
    const ID: Option<SpectaID>; // TODO: This is problematic

    /// returns a [`DataType`](crate::datatype::DataType) that represents the type.
    /// This will also register this and any dependent types into the [`TypeCollection`].
    fn definition(types: &mut TypeCollection) -> DataType;
}

/// A marker trait for compile-time validation of which types can be flattened.
pub trait Flatten: Type {}
