#[macro_use]
mod macros;
mod impls;
mod map;
mod post_process;
mod specta_id;

pub use map::*;
pub use post_process::*;
pub use specta_id::*;

use crate::{reference, DataType, NamedDataType, TypeImpl};

use self::reference::Reference;

/// Provides runtime type information that can be fed into a language exporter to generate a type definition in another language.
/// Avoid implementing this trait yourself where possible and use the [`Type`](derive@crate::Type) macro instead.
pub trait Type {
    /// Returns the inline definition of a type with generics substituted for those provided.
    /// This function defines the base structure of every type, and is used in both
    /// [`definition`](crate::Type::definition) and [`reference`](crate::Type::definition)
    ///
    /// Implemented internally or via the [`Type`](derive@crate::Type) macro
    fn inline(type_map: &mut TypeMap, generics: &[DataType]) -> TypeImpl;

    // TODO: Remove this in favor of it being an implementation detail of `inline`/`reference`???
    /// Small wrapper around [`inline`](crate::Type::inline) that provides
    /// [`definition_generics`](crate::Type::definition_generics)
    /// as the value for the `generics` arg.
    ///
    /// If your type is generic you *must* override the default implementation!
    // TODO: TypeImpl
    fn definition(type_map: &mut TypeMap) -> DataType {
        // TODO: Remove this default impl?
        // Self::inline(type_map, &[])
        todo!();
    }

    /// Generates a datatype corresponding to a reference to this type,
    /// as determined by its category. Getting a reference to a type implies that
    /// it should belong in the type map (since it has to be referenced from somewhere),
    /// so the output of [`definition`](crate::Type::definition) will be put into the type map.
    fn reference(type_map: &mut TypeMap, generics: &[DataType]) -> Reference {
        reference::inline::<Self>(type_map, generics)
    }
}

// TODO: This probs needs to go now because it's not relevant.
/// NamedType represents a type that can be converted into [NamedDataType].
/// This will be implemented for all types with the [Type] derive macro.
pub trait NamedType: Type {
    const SID: SpectaID;
    const IMPL_LOCATION: ImplLocation; // TODO: I don't think this is used so maybe remove it?

    /// this is equivalent to [Type::inline] but returns a [NamedDataType] instead.
    fn named_data_type(type_map: &mut TypeMap, generics: &[DataType]) -> NamedDataType;

    /// this is equivalent to [Type::definition] but returns a [NamedDataType] instead.
    fn definition_named_data_type(type_map: &mut TypeMap) -> NamedDataType;
}

/// A marker trait for compile-time validation of which types can be flattened.
pub trait Flatten: Type {}
