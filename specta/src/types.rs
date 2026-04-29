use std::{
    collections::{HashMap, hash_map},
    fmt,
};

use crate::{
    Type,
    datatype::{NamedDataType, NamedId, NamedReference},
};

/// Collection of named datatypes that can be exported together.
///
/// Resolving a [`Type`] adds every named type it depends on to this collection.
/// Exporters usually receive a completed `Types` value and iterate over the
/// collected [`NamedDataType`] entries.
///
/// # Invariants
///
/// Internally, entries may temporarily be placeholders while recursive types are
/// resolving. Public iterators and [`Types::len`] expose only completed
/// [`NamedDataType`] values.
#[derive(Default, Clone)]
pub struct Types {
    // `None` indicates that the entry is a placeholder.
    // It is a reference and we are currently resolving it's definition.
    pub(crate) types: HashMap<NamedId, Option<NamedDataType>>,

    // The count of non-`None` items in the collection.
    // We store this to avoid expensive iteration.
    pub(crate) len: usize,

    // TODO
    pub(crate) stack: Vec<u64>,

    // TODO: Explain
    pub(crate) should_inline: bool,

    // TODO: Explain
    /// This variables remains false unless your exporting in the context of `#[derive(Type)]` on a type which contains one or more const-generic parameters.
    ///
    /// Say for a type like this
    /// ```rs
    /// #[derive(Type)]
    /// struct Demo<const N: usize> {
    ///     data: [u32; N],
    /// }
    /// ```
    ///
    /// If we always set the length in the `impl Type for [T; N]`, the implementation will "bake" whatever the first encountered value of `N` is into the global type definition which is wrong. For example:
    /// ```rs
    /// pub struct A {
    ///     a: Demo<1>,
    ///     b: Demo<2>,
    /// }
    /// // becomes:
    /// // export type A = { a: Demo, b: Demo }
    /// // export type Demo = { [number] }; // This is invalid for the `b` field.
    ///
    /// // and if we encounter the fields in the opposite order it changes:
    ///
    /// pub struct B {
    ///     // we flipped field definition
    ///     b: Demo<2>,
    ///     a: Demo<1>,
    /// }
    /// // becomes:
    /// // export type A = { a: Demo, b: Demo }
    /// // export type Demo = { [number, number] }; // This is invalid for the `a` field.
    /// ```
    ///
    /// One observation is that for a length to differ across two instantiations of the same type it must either:
    ///  - Have a const parameter
    ///  - Have a generic which uses a trait associated constant
    ///
    /// Now Specta doesn't and can't support a generic with a trait associated constant as the generic `T` is shadowed by a virtual struct which is used to alter the type to return a generic reference, instead of a flat datatype.
    ///
    /// So for DX we know including length is safe as long as the resolving context doesn't have any const parameters. We track this using a thread local so it's entirely runtime meaning the solution doesn't require brittle scanning of the user's `TokenStream` in the derive macro.
    ///
    /// We provide `specta_util::FixedArray<N, T>` as a helper type to force Specta to export a fixed-length array instead of a generic `number[]` if you know what your doing.
    /// This doesn't fix the core issue but it does allow the user to assert they are correct.
    ///
    pub(crate) has_const_params: bool,
}

impl fmt::Debug for Types {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Types").field(&self.types).finish()
    }
}

impl Types {
    /// Registers `T` and its named dependencies with the collection.
    ///
    /// This consumes and returns `self`, making it convenient to chain multiple
    /// registrations.
    pub fn register<T: Type>(mut self) -> Self {
        T::definition(&mut self);
        self
    }

    /// Registers `T` and its named dependencies with the collection in-place.
    pub fn register_mut<T: Type>(&mut self) -> &mut Self {
        T::definition(self);
        self
    }

    /// Gets the named datatype targeted by a [`NamedReference`].
    ///
    /// Returns `None` if the reference is unknown or currently only has an
    /// internal placeholder entry.
    pub fn get(&self, r: &NamedReference) -> Option<&NamedDataType> {
        self.types.get(&r.id)?.as_ref()
    }

    /// Returns the number of completed named datatypes in the collection.
    pub fn len(&self) -> usize {
        debug_assert_eq!(
            self.len,
            self.types
                .iter()
                .filter_map(|(_, ndt)| ndt.as_ref())
                .count(),
            "Types count logic mismatch"
        );

        self.len
    }

    /// Returns `true` when the backing collection has no entries.
    ///
    /// This is usually equivalent to `len() == 0`. During type resolution,
    /// internal placeholders can make this return `false` even before any
    /// completed [`NamedDataType`] has been inserted.
    pub fn is_empty(&self) -> bool {
        self.types.is_empty()
    }

    /// Merges types from another collection into this one.
    ///
    /// Existing completed entries in `self` are kept. A placeholder in `self` is
    /// replaced by a completed entry from `other` when available.
    pub fn extend(&mut self, other: &Self) {
        for (id, other) in &other.types {
            match self.types.get(id) {
                // Key doesn't exist - insert from other
                None => {
                    if other.is_some() {
                        self.len += 1;
                    }
                    self.types.insert(id.clone(), other.clone());
                }
                // Key exists with Some - keep self (prefer self over other)
                Some(Some(_)) => {}
                // Key exists with None, but other has Some - use other (prefer Some over None)
                Some(None) if other.is_some() => {
                    self.len += 1;
                    self.types.insert(id.clone(), other.clone());
                }
                // Key exists with None, other also None - do nothing
                Some(None) => {}
            }
        }
    }

    /// Sorts the collection into a consistent order and returns an iterator.
    ///
    /// The sort order is not necessarily guaranteed to be stable between versions but currently we sort by name.
    ///
    /// This method requires reallocating the map to sort the collection. You should prefer [Self::into_unsorted_iter] if you don't care about the order.
    pub fn into_sorted_iter(&self) -> impl ExactSizeIterator<Item = &'_ NamedDataType> {
        let mut v = self
            .types
            .iter()
            .filter_map(|(_, ndt)| ndt.as_ref())
            .collect::<Vec<_>>();
        assert_eq!(v.len(), self.len, "Types count logic mismatch");
        v.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then(a.module_path.cmp(&b.module_path))
                .then(a.location.cmp(&b.location))
        });
        v.into_iter()
    }

    /// Returns an unsorted iterator over completed named datatypes.
    pub fn into_unsorted_iter(&self) -> impl ExactSizeIterator<Item = &NamedDataType> {
        UnsortedIter {
            iter: self.types.iter(),
            count: self.len,
        }
    }

    /// Calls `f` for each completed named datatype in the collection.
    ///
    /// The iteration order is intentionally unspecified.
    pub fn iter_mut<F>(&mut self, mut f: F)
    where
        F: FnMut(&mut NamedDataType),
    {
        for (_, ndt) in self.types.iter_mut() {
            if let Some(ndt) = ndt {
                f(ndt);
            }
        }
    }

    /// Transforms each completed [`NamedDataType`] in the collection.
    ///
    /// The internal identity keys are preserved, so existing [`NamedReference`]s
    /// continue to resolve to the transformed entries.
    pub fn map<F>(mut self, mut f: F) -> Self
    where
        F: FnMut(NamedDataType) -> NamedDataType,
    {
        for (_, slot) in self.types.iter_mut() {
            if let Some(ndt) = slot.take() {
                *slot = Some(f(ndt));
            }
        }
        self
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
