use core::fmt;
use std::{
    borrow::{Borrow, Cow},
    fmt::Display,
    marker::PhantomData,
};

use crate::Type;

use super::DataType;

/// A generic ("placeholder") argument to a Specta-enabled type.
///
/// A generic does not hold a specific type instead it acts as a slot where a type can be provided when referencing this type.
///
/// A `GenericType` holds the identifier of the generic. Eg. Given a generic type `struct A<T>(T);` the generics will be represented as `vec![GenericType("A".into())]`
#[derive(Debug, Clone, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct Generic(pub(crate) Cow<'static, str>);

impl Display for Generic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

// TODO: Deref instead?
impl Borrow<str> for Generic {
    fn borrow(&self) -> &str {
        &self.0
    }
}

impl From<Cow<'static, str>> for Generic {
    fn from(value: Cow<'static, str>) -> Self {
        Self(value)
    }
}

impl From<Generic> for DataType {
    fn from(t: Generic) -> Self {
        Self::Generic(t)
    }
}

/// A generic placeholder.
pub trait ConstGenericPlaceholder {
    const PLACEHOLDER: &'static str;
}

/// A placeholder for a generic type.
///
/// # How this works?
///
/// When you use the [`Type`](crate::Type) derive macro on a type we transform all generics to be a `Generic<T>` before resolving the types.
/// This ensures we have placeholders to the correct generic when exporting.
///
/// This major downside to this approach is that if you have custom generic bounds, they might not be implemented by `Generic<T>`.
///
/// TODO: Show detailed transformation.
///
// TODO: We should replace the `Generic `trait with `const P: &'static str` if we do a Specta v3.
pub struct GenericPlaceholder<T: ConstGenericPlaceholder>(PhantomData<T>);

impl<T: ConstGenericPlaceholder> fmt::Debug for GenericPlaceholder<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(T::PLACEHOLDER)
    }
}

impl<T: ConstGenericPlaceholder> Default for GenericPlaceholder<T> {
    fn default() -> Self {
        panic!("Can't construct a generic type without a placeholder");
    }
}

impl<T: ConstGenericPlaceholder> Clone for GenericPlaceholder<T> {
    fn clone(&self) -> Self {
        unreachable!();
    }
}

impl<T: ConstGenericPlaceholder> PartialEq for GenericPlaceholder<T> {
    fn eq(&self, _: &Self) -> bool {
        unreachable!();
    }
}

impl<T: ConstGenericPlaceholder> std::hash::Hash for GenericPlaceholder<T> {
    fn hash<H: std::hash::Hasher>(&self, _: &mut H) {
        unreachable!();
    }
}

impl<T: ConstGenericPlaceholder> Type for GenericPlaceholder<T> {
    fn definition(_: &mut crate::TypeCollection) -> DataType {
        DataType::Generic(Generic(Cow::Borrowed(T::PLACEHOLDER)))
    }
}
