#[macro_use]
mod macros;
mod impls;
mod map;
mod post_process;
mod specta_id;

use std::borrow::Cow;

pub use map::*;
pub use post_process::*;
pub use specta_id::*;

use crate::{reference::Reference, DataType, NamedDataType};

// TODO: Break out into it's own file?
pub enum Generics<'a> {
    /// The types "definition generics" will be used.
    ///
    /// These generics are Rust generics provided to the type itself when it was instantiated.
    /// You will always have these but they will always be concrete types instead of a generic.
    ///
    /// For example given `Demo<String>` the generic will become `String` and not `T`
    Definition,

    /// The generics will be substituted for those provided, if you don't provide enough the definition generic will be used as a fallback
    ///
    /// TODO: Discuss the problem with this approach and why we can't solve it
    // TODO: Is the variant's name good?
    // TODO: Should this be a `Cow` or just `&[]`. Really we don't need ownership so probs the later given the DX improvements.
    Provided(Cow<'a, [DataType]>),
}

impl<'a> Generics<'a> {
    // TODO: Is a distrinction between `None` and `Provided(Cow::Borrowed(&[]))` needed cause aren't they theoretically the same thing.
    pub const NONE: Self = Self::Provided(Cow::Borrowed(&[]));

    // TODO: is this a good name for this method, I don't know it acts the same as a Rust developer might expect.
    // TODO: This could possible be `Clone` but I think it sends the wrong message given `Cow::clone` allocates and maintain the variant.
    pub fn as_ref<'b>(&'b self) -> Generics<'b> {
        match self {
            Generics::Definition => Generics::Definition,
            Generics::Provided(generics) => Generics::Provided(Cow::Borrowed(&generics[..])),
        }
    }

    // TODO: Naming is hard
    pub fn as_cow<'b>(&'b self) -> Cow<'b, [DataType]> {
        match self {
            Generics::Definition => Cow::Borrowed(&[]),
            Generics::Provided(generics) => Cow::Borrowed(&generics[..]),
        }
    }
}

impl From<Vec<DataType>> for Generics<'_> {
    fn from(generics: Vec<DataType>) -> Self {
        Generics::Provided(Cow::Owned(generics))
    }
}

impl<'a, const N: usize> From<&'a [DataType; N]> for Generics<'a> {
    fn from(generics: &'a [DataType; N]) -> Self {
        Generics::Provided(Cow::Borrowed(&generics[..]))
    }
}

impl<'a> From<&'a [DataType]> for Generics<'a> {
    fn from(generics: &'a [DataType]) -> Self {
        Generics::Provided(Cow::Borrowed(generics))
    }
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
    // TODO: `DataTypeReference` a return type
    fn reference(type_map: &mut TypeMap, generics: Cow<[DataType]>) -> Reference {
        // reference::inline::<Self>(type_map, generics)
        todo!();
    }
}

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
