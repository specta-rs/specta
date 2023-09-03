use thiserror::Error;

use crate::*;

#[macro_use]
mod macros;
mod impls;
mod post_process;

pub use post_process::*;

use self::reference::Reference;

pub type Result<T> = std::result::Result<T, ExportError>;

/// Type exporting errors.
#[derive(Error, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[non_exhaustive]
pub enum ExportError {
    #[error("Atemmpted to export type defined at '{}' but encountered error: {1}", .0.as_str())]
    InvalidType(ImplLocation, &'static str),
}

/// Provides runtime type information that can be fed into a language exporter to generate a type definition in another language.
/// Avoid implementing this trait yourself where possible and use the [`Type`](derive@crate::Type) macro instead.
pub trait Type {
    /// Returns the inline definition of a type with generics substituted for those provided.
    /// This function defines the base structure of every type, and is used in both
    /// [`definition`](crate::Type::definition) and [`reference`](crate::Type::definition)
    ///
    /// Implemented internally or via the [`Type`](derive@crate::Type) macro
    fn inline(opts: DefOpts, generics: &[DataType]) -> Result<DataType>;

    /// Returns the type parameter generics of a given type.
    /// Will usually be empty except for custom types.
    ///
    /// Implemented internally or via the [`Type`](derive@crate::Type) macro
    fn definition_generics() -> Vec<GenericType> {
        vec![]
    }

    /// Small wrapper around [`inline`](crate::Type::inline) that provides
    /// [`definition_generics`](crate::Type::definition_generics)
    /// as the value for the `generics` arg.
    ///
    /// Implemented internally
    fn definition(opts: DefOpts) -> Result<DataType> {
        Self::inline(
            opts,
            &Self::definition_generics()
                .into_iter()
                .map(Into::into)
                .collect::<Vec<_>>(),
        )
    }

    /// Generates a datatype corresponding to a reference to this type,
    /// as determined by its category. Getting a reference to a type implies that
    /// it should belong in the type map (since it has to be referenced from somewhere),
    /// so the output of [`definition`](crate::Type::definition) will be put into the type map.
    fn reference(opts: DefOpts, generics: &[DataType]) -> Result<Reference> {
        reference::inline::<Self>(opts, generics)
    }
}

/// NamedType represents a type that can be converted into [NamedDataType].
/// This will be implemented for all types with the [Type] derive macro.
pub trait NamedType: Type {
    const SID: SpectaID;
    const IMPL_LOCATION: ImplLocation;

    /// this is equivalent to [Type::inline] but returns a [NamedDataType] instead.
    fn named_data_type(opts: DefOpts, generics: &[DataType]) -> Result<NamedDataType>;

    /// this is equivalent to [Type::definition] but returns a [NamedDataType] instead.
    fn definition_named_data_type(opts: DefOpts) -> Result<NamedDataType> {
        Self::named_data_type(
            opts,
            &Self::definition_generics()
                .into_iter()
                .map(Into::into)
                .collect::<Vec<_>>(),
        )
    }
}

/// Helpers for generating [Type::reference] implementations.
pub mod reference {
    use super::*;

    /// A reference datatype.
    ///
    // This type exists to force the user to use [reference::inline] or [reference::reference] which provides some extra safety.
    pub struct Reference {
        pub inner: DataType,
        pub(crate) _priv: (),
    }

    pub fn inline<T: Type + ?Sized>(opts: DefOpts, generics: &[DataType]) -> Result<Reference> {
        Ok(Reference {
            inner: T::inline(opts, generics)?,
            _priv: (),
        })
    }

    pub fn reference<T: NamedType>(
        opts: DefOpts,
        generics: &[DataType],
        reference: DataTypeReference,
    ) -> Result<Reference> {
        if opts.type_map.get(&T::SID).is_none() {
            // It's important we don't put `None` into the map here. By putting a *real* value we ensure that we don't stack overflow for recursive types when calling `named_data_type`.
            opts.type_map.entry(T::SID).or_insert(Some(NamedDataType {
                name: "placeholder".into(),
                comments: vec![],
                deprecated: None,
                ext: None,
                item: DataType::Any,
            }));

            let dt = T::named_data_type(
                DefOpts {
                    parent_inline: true,
                    type_map: opts.type_map,
                },
                generics,
            )?;
            opts.type_map.insert(T::SID, Some(dt));
        }

        Ok(Reference {
            inner: DataType::Reference(reference),
            _priv: (),
        })
    }
}

/// A marker trait for compile-time validation of which types can be flattened.
pub trait Flatten: Type {}

/// The unique Specta ID for the type.
///
/// Be aware type aliases don't exist as far as Specta is concerned as they are flattened into their inner type by Rust's trait system.
/// The Specta Type ID holds for the given properties:
///  - `T::SID == T::SID`
///  - `T::SID != S::SID`
///  - `Type<T>::SID == Type<S>::SID` (unlike std::any::TypeId)
///  - `Box<T> == Arc<T> == Rc<T>` (unlike std::any::TypeId)
///
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[doc(hidden)]
pub struct SpectaID(u64);

/// Compute an SID hash for a given type.
/// This hash function comes from https://stackoverflow.com/a/71464396
/// You should NOT use this directly. Rely on `sid!();` instead.
#[doc(hidden)]
pub const fn internal_sid_hash(
    module_path: &'static str,
    file: &'static str,
    // This is required for a unique hash because all impls generated by a `macro_rules!` will have an identical `module_path` and `file` value.
    type_name: &'static str,
) -> SpectaID {
    let mut hash = 0xcbf29ce484222325;
    let prime = 0x00000100000001B3;

    let mut bytes = module_path.as_bytes();
    let mut i = 0;

    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(prime);
        i += 1;
    }

    bytes = file.as_bytes();
    i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(prime);
        i += 1;
    }

    bytes = type_name.as_bytes();
    i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u64;
        hash = hash.wrapping_mul(prime);
        i += 1;
    }

    SpectaID(hash)
}

/// Compute an SID hash for a given type.
#[macro_export]
#[doc(hidden)]
macro_rules! sid {
    ($name:expr) => {
        $crate::sid!($name, $crate::impl_location!().as_str())
    };
     // Using `$crate_path:path` here does not work because: https://github.com/rust-lang/rust/issues/48067
    (@with_specta_path; $name:expr; $first:ident$(::$rest:ident)*) => {{
        use $first$(::$rest)*::{internal_sid_hash, impl_location};

        internal_sid_hash(
            module_path!(),
            impl_location!().as_str(),
            $name,
        )
    }};
}

/// The location of the impl block for a given type. This is used for error reporting.
/// The content of it is transparent and should be generated by the `impl_location!` macro.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[doc(hidden)]
pub struct ImplLocation(&'static str);

impl ImplLocation {
    #[doc(hidden)]
    pub const fn internal_new(s: &'static str) -> Self {
        Self(s)
    }

    /// Get the location as a string
    pub const fn as_str(&self) -> &'static str {
        self.0
    }
}

/// Compute the location for an impl block
#[macro_export]
#[doc(hidden)]
macro_rules! impl_location {
    () => {
        $crate::ImplLocation::internal_new(concat!(file!(), ":", line!(), ":", column!()))
    };
    // Using `$crate_path:path` here does not work because: https://github.com/rust-lang/rust/issues/48067
    (@with_specta_path; $first:ident$(::$rest:ident)*) => {
        $first$(::$rest)*::ImplLocation::internal_new(concat!(file!(), ":", line!(), ":", column!()))
    };
}
