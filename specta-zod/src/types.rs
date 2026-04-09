use std::fmt::Debug;

use specta::{
    datatype::{DataType, Reference},
    Type, Types,
};

use crate::opaque;

/// Cast a Rust type to `z.any()`.
pub struct Any<T = ()>(T);

impl<T> Type for Any<T> {
    fn definition(_: &mut Types) -> DataType {
        DataType::Reference(Reference::opaque(opaque::Any))
    }
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

/// Cast a Rust type to `z.unknown()`.
pub struct Unknown<T = ()>(T);

impl<T> Type for Unknown<T> {
    fn definition(_: &mut Types) -> DataType {
        DataType::Reference(Reference::opaque(opaque::Unknown))
    }
}

impl<T: Debug> Debug for Unknown<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Unknown").field(&self.0).finish()
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

/// Cast a Rust type to `z.never()`.
pub struct Never<T = ()>(T);

impl<T> Type for Never<T> {
    fn definition(_: &mut Types) -> DataType {
        DataType::Reference(Reference::opaque(opaque::Never))
    }
}

impl<T: Debug> Debug for Never<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Never").field(&self.0).finish()
    }
}

impl<T: Clone> Clone for Never<T> {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl<T: Default> Default for Never<T> {
    fn default() -> Self {
        Self(T::default())
    }
}

#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
impl<T: serde::Serialize> serde::Serialize for Never<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        T::serialize(&self.0, serializer)
    }
}
