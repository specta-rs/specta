use crate::{
    datatype::{
        reference::{self, Reference},
        DataType, NamedDataType,
    },
    SpectaID, TypeCollection,
};

mod impls;
mod macros;
// TODO: We don't care much about these cause they are gonna go so this will do for now.
#[cfg(feature = "derive")]
mod legacy_impls;

/// TODO
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
    // TODO: Is a distinction between `Self::Definition` and `Self::Provided(Cow::Borrowed(&[]))` cause aren't they theoretically the same thing.
    #[doc(hidden)] // TODO: Probs remove this
    pub const NONE: Self = Self::Provided(&[]);
}

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
