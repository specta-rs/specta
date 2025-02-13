use std::fmt::Debug;

use specta::{
    datatype::{reference::Reference, DataType},
    NamedType, Type, TypeCollection,
};

/// Cast a Rust type to a Typescript `any` type.
///
/// WARNING: When used with `Option<Any<T>>`, Typescript will not prompt you about nullability checks as `any | null` is coalesced to `any` in Typescript.
///
/// # Examples
///
/// This can be used as a type override.
/// ```rust
/// use serde::Serialize;
/// use specta::Type;
/// use specta_typescript::Any;
///
/// #[derive(Serialize, Type)]
/// pub struct Demo {
///     #[specta(type = Any)]
///     pub field: String,
/// }
/// ```
///
/// Or it can be used as a wrapper type.
/// ```rust
/// use serde::Serialize;
/// use specta::Type;
/// use specta_typescript::Any;
///
/// #[derive(Serialize, Type)]
/// pub struct Demo {
///     pub field: Any<String>,
/// }
/// ```
pub struct Any<T = ()>(T);

impl<T> Type for Any<T> {
    fn definition(types: &mut TypeCollection) -> DataType {
        types.placeholder(Self::ID);
        DataType::Reference(Reference::construct(Self::ID, [], false))
    }
}

impl<T> NamedType for Any<T> {
    const ID: specta::SpectaID = specta::internal::construct::sid(
        "Any",
        concat!("::", module_path!(), ":", line!(), ":", column!()),
    );
}

impl<T: Debug> Debug for Any<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Any").field(&self.0).finish()
    }
}

impl<T: Clone> Clone for Any<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Default> Default for Any<T> {
    fn default() -> Self {
        Self(T::default())
    }
}

#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl<T: serde::Serialize> serde::Serialize for Any<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        T::serialize(&self.0, serializer)
    }
}

/// Cast a Rust type to a Typescript `unknown` type.
///
/// # Examples
///
/// This can be used as a type override.
/// ```rust
/// use serde::Serialize;
/// use specta::Type;
/// use specta_typescript::Unknown;
///
/// #[derive(Serialize, Type)]
/// pub struct Demo {
///     #[specta(type = Unknown)]
///     pub field: String,
/// }
/// ```
///
/// Or it can be used as a wrapper type.
/// ```rust
/// use serde::Serialize;
/// use specta::Type;
/// use specta_typescript::Unknown;
///
/// #[derive(Serialize, Type)]
/// pub struct Demo {
///     pub field: Unknown<String>,
/// }
/// ```
pub struct Unknown<T = ()>(T);

impl<T> Type for Unknown<T> {
    fn definition(types: &mut TypeCollection) -> DataType {
        types.placeholder(Self::ID);
        DataType::Reference(Reference::construct(Self::ID, [], false))
    }
}

impl<T> NamedType for Unknown<T> {
    const ID: specta::SpectaID = specta::internal::construct::sid(
        "Unknown",
        concat!("::", module_path!(), ":", line!(), ":", column!()),
    );
}

impl<T: Debug> Debug for Unknown<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Any").field(&self.0).finish()
    }
}

impl<T: Clone> Clone for Unknown<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Default> Default for Unknown<T> {
    fn default() -> Self {
        Self(T::default())
    }
}

#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl<T: serde::Serialize> serde::Serialize for Unknown<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        T::serialize(&self.0, serializer)
    }
}
