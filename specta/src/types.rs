use std::{
    collections::{HashMap, hash_map},
    fmt,
};

use crate::{
    Type,
    datatype::{NamedDataType, NamedId},
};

/// Define a set of types which can be exported together.
///
/// While exporting a type will add all of the types it depends on to the collection.
/// You can also construct your own collection to easily export a set of types together.
#[derive(Default, Clone)]
pub struct Types(
    // `None` indicates that the entry is a placeholder.
    // It is a reference and we are currently resolving it's definition.
    pub(crate) HashMap<NamedId, Option<NamedDataType>>,
    // The count of non-`None` items in the collection.
    // We store this to avoid expensive iteration.
    pub(crate) usize,
);

/// A wrapper around [`Types`] indicating the type graph has already been
/// transformed for a specific export format.
///
/// This is generally constructed by a format crate (for example
/// [`specta-serde`](https://docs.rs/specta-serde)) after applying
/// format-specific rewrites.
///
/// Constructing this wrapper from plain [`Types`] is explicit because the
/// conversion may change type shapes. Prefer using your format crate's
/// conversion entry points when possible.
#[derive(Debug, Clone)]
pub struct ResolvedTypes(Types);

impl fmt::Debug for Types {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Types").field(&self.0).finish()
    }
}

impl Types {
    /// Register a [`Type`] with the collection.
    pub fn register<T: Type>(mut self) -> Self {
        T::definition(&mut self);
        self
    }

    /// Register a [`Type`](crate::Type) with the collection.
    pub fn register_mut<T: Type>(&mut self) -> &mut Self {
        T::definition(self);
        self
    }

    /// Get the length of the collection.
    pub fn len(&self) -> usize {
        debug_assert_eq!(
            self.1,
            self.0.iter().filter_map(|(_, ndt)| ndt.as_ref()).count(),
            "Types count logic mismatch"
        );

        self.1
    }

    /// Check if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Merge types from another collection into this one.
    pub fn extend(&mut self, other: &Self) {
        for (id, other) in &other.0 {
            match self.0.get(id) {
                // Key doesn't exist - insert from other
                None => {
                    if other.is_some() {
                        self.1 += 1;
                    }
                    self.0.insert(id.clone(), other.clone());
                }
                // Key exists with Some - keep self (prefer self over other)
                Some(Some(_)) => {}
                // Key exists with None, but other has Some - use other (prefer Some over None)
                Some(None) if other.is_some() => {
                    self.1 += 1;
                    self.0.insert(id.clone(), other.clone());
                }
                // Key exists with None, other also None - do nothing
                Some(None) => {}
            }
        }
    }

    /// Sort the collection into a consistent order and return an iterator.
    ///
    /// The sort order is not necessarily guaranteed to be stable between versions but currently we sort by name.
    ///
    /// This method requires reallocating the map to sort the collection. You should prefer [Self::into_unsorted_iter] if you don't care about the order.
    pub fn into_sorted_iter(&self) -> impl ExactSizeIterator<Item = &'_ NamedDataType> {
        let mut v = self
            .0
            .iter()
            .filter_map(|(_, ndt)| ndt.as_ref())
            .collect::<Vec<_>>();
        assert_eq!(v.len(), self.1, "Types count logic mismatch");
        v.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then(a.module_path.cmp(&b.module_path))
                .then(a.location.cmp(&b.location))
        });
        v.into_iter()
    }

    /// Return the unsorted iterator over the collection.
    pub fn into_unsorted_iter(&self) -> impl ExactSizeIterator<Item = &NamedDataType> {
        UnsortedIter {
            iter: self.0.iter(),
            count: self.1,
        }
    }

    /// Return an mutable iterator over the type collection.
    /// Note: The order returned is unsorted.
    pub fn iter_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut NamedDataType),
    {
        for (_, ndt) in self.0.iter_mut() {
            if let Some(ndt) = ndt {
                f(ndt);
            }
        }
    }

    /// Map over the collection, transforming each `NamedDataType` with the given closure.
    /// This preserves the `ArcId` keys, ensuring that `Reference`s remain valid.
    pub fn map<F>(mut self, mut f: F) -> Self
    where
        F: FnMut(NamedDataType) -> NamedDataType,
    {
        for (_, slot) in self.0.iter_mut() {
            if let Some(ndt) = slot.take() {
                *slot = Some(f(ndt));
            }
        }
        self
    }
}

impl ResolvedTypes {
    /// Wrap already-resolved [`Types`] as [`ResolvedTypes`].
    ///
    /// This should generally be called by format crates after they finish their
    /// own transformation pass (for example `specta_serde::apply` or
    /// `specta_serde::apply_phases`).
    ///
    /// If you call this in end-user code your types may not look how you expect!
    pub fn from_resolved_types(types: Types) -> Self {
        Self(types)
    }

    /// Borrow the underlying [`Types`] collection.
    ///
    /// # Notes
    ///
    /// This does not undo format-specific resolution. If a format crate already
    /// rewrote type shapes, this still returns those rewritten shapes. It is your
    /// responsibility to ensure consumers treat these as already-resolved types.
    pub fn as_types(&self) -> &Types {
        &self.0
    }

    /// Consume [`ResolvedTypes`] and return the underlying [`Types`].
    ///
    /// # Notes
    ///
    /// This does not undo format-specific resolution. The returned [`Types`]
    /// remain whatever shape they were resolved into.
    pub fn into_types(self) -> Types {
        self.0
    }

    /// Sort the collection into a consistent order and return an iterator.
    ///
    /// The sort order is not necessarily guaranteed to be stable between versions but currently we sort by name.
    ///
    /// This method requires reallocating the map to sort the collection. You should prefer [Self::into_unsorted_iter] if you don't care about the order.
    pub fn into_sorted_iter(&self) -> impl ExactSizeIterator<Item = &'_ NamedDataType> {
        self.0.into_sorted_iter()
    }

    /// Return the unsorted iterator over the collection.
    pub fn into_unsorted_iter(&self) -> impl ExactSizeIterator<Item = &NamedDataType> {
        self.0.into_unsorted_iter()
    }

    /// Return an mutable iterator over the type collection.
    /// Note: The order returned is unsorted.
    pub fn iter_mut<F>(&mut self, f: F)
    where
        F: FnMut(&mut NamedDataType),
    {
        self.0.iter_mut(f);
    }
}

struct UnsortedIter<'a> {
    iter: hash_map::Iter<'a, NamedId, Option<NamedDataType>>,
    count: usize,
}

impl<'a> Iterator for UnsortedIter<'a> {
    type Item = &'a NamedDataType;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.find_map(|(_, ndt)| ndt.as_ref())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.count, Some(self.count))
    }
}

impl ExactSizeIterator for UnsortedIter<'_> {}
