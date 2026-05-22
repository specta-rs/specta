use std::{
    collections::{HashMap, hash_map},
    fmt,
};

use crate::{
    Type,
    datatype::{NamedDataType, NamedId, NamedReference, RecursiveInlineFrame},
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
    /// Registered named datatypes keyed by their stable identity.
    ///
    /// A `None` value is a placeholder for a datatype whose definition is
    /// currently being resolved. This lets recursive definitions refer to the
    /// in-progress type without re-entering resolution indefinitely.
    pub(crate) types: HashMap<NamedId, Option<NamedDataType>>,

    /// Cached count of completed entries in [`Self::types`].
    ///
    /// Placeholders are excluded. Keeping this count avoids repeatedly walking
    /// the full map when exporters ask for iterator lengths which we need for `ExactSizeIterator`.
    pub(crate) len: usize,

    /// Stack of inline named-type expansions currently being resolved.
    ///
    /// Each entry is a hash of the named type sentinel and concrete generic
    /// arguments for that inline use site. Seeing the same entry twice means an
    /// inline definition has recursively reached itself, so resolution can emit
    /// a recursive reference instead of expanding forever.
    pub(crate) stack: Vec<InlineResolutionFrame>,

    /// Whether named types discovered in the current context should be inlined.
    ///
    /// This is set while resolving fields annotated with `#[specta(inline)]` and
    /// similar container/wrapper contexts. It is temporarily cleared when
    /// building canonical named definitions so top-level registrations are not
    /// accidentally affected by a use-site inline request.
    pub(crate) should_inline: bool,

    /// Whether the current named-type definition is being built with const parameters.
    ///
    /// This remains `false` unless Specta is exporting the canonical definition
    /// for a `#[derive(Type)]` type that declares one or more const-generic
    /// parameters.
    ///
    /// Consider a type like this:
    ///
    /// ```rs
    /// #[derive(Type)]
    /// struct Demo<const N: usize> {
    ///     data: [u32; N],
    /// }
    /// ```
    ///
    /// If `impl Type for [T; N]` always exported the concrete array length, the
    /// first encountered value of `N` would be baked into the shared global
    /// definition for `Demo`, which is wrong. For example:
    ///
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
    /// For a length to differ across two instantiations of the same type, the
    /// type must either have a const parameter or have a generic parameter whose
    /// type uses a trait associated constant.
    ///
    /// Specta does not support the trait-associated-constant case here because
    /// generic `T` parameters are shadowed by virtual structs that return generic
    /// references instead of flat datatypes.
    ///
    /// Therefore, including the fixed array length is safe as long as the
    /// current resolving context has no const parameters. This is tracked at
    /// runtime, avoiding brittle scans of the user's `TokenStream` in the derive
    /// macro.
    ///
    /// `specta_util::FixedArray<N, T>` can be used to force Specta to export a
    /// fixed-length array instead of a generic `number[]` when the user knows
    /// the length is safe to include. This does not fix the core issue, but it
    /// gives the user a way to assert the specific use site is correct.
    pub(crate) has_const_params: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct InlineResolutionFrame {
    pub(crate) hash: u64,
    pub(crate) ty: RecursiveInlineFrame,
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
            self.types.values().filter_map(|ndt| ndt.as_ref()).count(),
            "Types count logic mismatch"
        );

        self.len
    }

    /// Returns `true` when the typemap has no entries at all.
    pub fn is_empty(&self) -> bool {
        debug_assert_eq!(
            self.len,
            self.types.values().filter_map(|ndt| ndt.as_ref()).count(),
            "Types count logic mismatch"
        );

        self.len == 0
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

    /// Sorts completed named datatypes into a consistent order and returns an iterator.
    ///
    /// The sort order is not guaranteed to remain identical between releases but is designed to stay stable,
    /// so that between multiple runs of the exporter you get the same type in the same order in the file.
    ///
    /// This method allocates a temporary vector to sort the collection. Prefer
    /// [`Types::into_unsorted_iter`] if the order does not matter.
    pub fn into_sorted_iter(&self) -> impl ExactSizeIterator<Item = &'_ NamedDataType> {
        let mut v = self
            .types
            .values()
            .filter_map(|ndt| ndt.as_ref())
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

    /// Mutably modifies each [`NamedDataType`] in the collection.
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

    /// Transforms each [`NamedDataType`] in the collection.
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
