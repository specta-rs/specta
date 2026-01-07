use std::{collections::HashMap, fmt};

use crate::{
    Type,
    datatype::{ArcId, NamedDataType},
};

/// Define a set of types which can be exported together.
///
/// While exporting a type will add all of the types it depends on to the collection.
/// You can also construct your own collection to easily export a set of types together.
#[derive(Default, Clone)]
pub struct TypeCollection(
    // `None` indicates that the entry is a placeholder.
    // It is a reference and we are currently resolving it's definition.
    pub(crate) HashMap<ArcId, Option<NamedDataType>>,
);

impl fmt::Debug for TypeCollection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TypeCollection").field(&self.0).finish()
    }
}

impl TypeCollection {
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
        self.0.iter().filter_map(|(_, ndt)| ndt.as_ref()).count()
    }

    /// Check if the collection is empty.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Sort the collection into a consistent order and return an iterator.
    ///
    /// The sort order is not necessarily guaranteed to be stable between versions but currently we sort by name.
    ///
    /// This method requires reallocating the map to sort the collection. You should prefer [Self::into_unsorted_iter] if you don't care about the order.
    pub fn into_sorted_iter(&self) -> impl Iterator<Item = NamedDataType> {
        let mut v = self
            .0
            .iter()
            .filter_map(|(_, ndt)| ndt.clone())
            .collect::<Vec<_>>();
        v.sort_by(|a, b| {
            a.name
                .cmp(&b.name)
                .then(a.module_path.cmp(&b.module_path))
                .then(a.location.cmp(&b.location))
        });
        v.into_iter()
    }

    /// Return the unsorted iterator over the collection.
    pub fn into_unsorted_iter(&self) -> impl Iterator<Item = &NamedDataType> {
        self.0.iter().filter_map(|(_, ndt)| ndt.as_ref())
    }

    /// Map over the collection, transforming each `NamedDataType` with the given closure.
    /// This preserves the `ArcId` keys, ensuring that `Reference`s remain valid.
    pub fn map<F>(mut self, mut f: F) -> Self
    where
        F: FnMut(NamedDataType) -> NamedDataType,
    {
        for (_, ndt) in self.0.iter_mut() {
            if let Some(named_data_type) = ndt.take() {
                *ndt = Some(f(named_data_type));
            }
        }
        self
    }
}
