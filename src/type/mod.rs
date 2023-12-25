#[macro_use]
mod macros;
mod impls;
mod map;
mod post_process;
mod specta_id;

pub use map::*;
pub use post_process::*;
pub use specta_id::*;

use crate::{
    reference::{self, Reference},
    DataType, NamedDataType,
};

// TODO: Break out into it's own file?
#[derive(Debug, Clone, Copy)]
pub enum Generics<'a> {
    /// The types "definition generics" will be used.
    ///
    /// These generics are Rust generics provided to the type itself when it was instantiated.
    /// You will always have these but they will always be concrete types instead of a generic.
    ///
    /// For example given `Demo<String>` the generic will become `String` and not `T`
    Definition,

    /// The generics will be substituted for those provided, if you don't provide the correct amount the definition generic will be used as a fallback
    ///
    /// TODO: Discuss the problem with this approach and why we can't solve it
    // TODO: Is the variant's name good?
    // Given this type is an input we don't care about ownership so a `Cow` is not needed.
    Provided(&'a [DataType]),
}

impl<'a> Generics<'a> {
    // TODO: Is a distrinction between `Self::Definition` and `Self::Provided(Cow::Borrowed(&[]))` cause aren't they theoretically the same thing.
    #[doc(hidden)] // TODO: Probs remove this
    pub const NONE: Self = Self::Provided(&[]);
}

/// Provides runtime type information that can be fed into a language exporter to generate a type definition in another language.
/// Avoid implementing this trait yourself where possible and use the [`Type`](derive@crate::Type) macro instead.
pub trait Type {
    // TODO: Rename this method
    /// Returns the definition of a type using the provided generics.
    ///
    /// This should be only implemented via the [`Type`](derive@crate::Type) macro.
    fn inline(type_map: &mut TypeMap, generics: Generics) -> DataType;

    /// Generates a datatype corresponding to a reference to this type,
    /// as determined by its category. Getting a reference to a type implies that
    /// it should belong in the type map (since it has to be referenced from somewhere),
    /// so the output of [`definition`](crate::Type::definition) will be put into the type map.
    fn reference(type_map: &mut TypeMap, generics: &[DataType]) -> Reference {
        reference::inline::<Self>(type_map, generics)
    }
}

/// NamedType represents a type that can be converted into [NamedDataType].
/// This will be implemented for all types with the [Type] derive macro.
pub trait NamedType: Type {
    const SID: SpectaID;
    const IMPL_LOCATION: ImplLocation; // TODO: I don't think this is used so maybe remove it?

    // TODO: Should take `Generics` instead of `&[DataType]` but I plan to remove this trait so not fixing it for now.
    /// this is equivalent to [Type::inline] but returns a [NamedDataType] instead.
    fn named_data_type(type_map: &mut TypeMap, generics: &[DataType]) -> NamedDataType;

    // TODO: Just remove this method given we removed `Type::definition`
    /// this is equivalent to [Type::definition] but returns a [NamedDataType] instead.
    fn definition_named_data_type(type_map: &mut TypeMap) -> NamedDataType;
}

/// A marker trait for compile-time validation of which types can be flattened.
pub trait Flatten: Type {}
