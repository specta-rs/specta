use std::{collections::HashMap, fmt};

use crate::{SpectaID, Type, datatype::NamedDataType};

/// Define a set of types which can be exported together.
///
/// While exporting a type will add all of the types it depends on to the collection.
/// You can also construct your own collection to easily export a set of types together.
#[derive(Default, Clone)]
pub struct TypeCollection(
    // `None` indicates that the entry is a placeholder.
    // It is a reference and we are currently resolving it's definition.
    pub(crate) HashMap<SpectaID, Option<NamedDataType>>,
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

    /// Register a [`NamedType`] with the collection.
    pub fn register_mut<T: Type>(&mut self) -> &mut Self {
        T::definition(self);
        self
    }

    /// Remove a type from the collection.
    #[doc(hidden)]
    #[deprecated = "https://github.com/specta-rs/specta/issues/426"]
    pub fn remove(&mut self, sid: SpectaID) -> Option<NamedDataType> {
        self.0.remove(&sid).flatten()
    }

    /// Get a type from the collection.
    #[track_caller]
    pub fn get(&self, sid: SpectaID) -> Option<&NamedDataType> {
        #[allow(clippy::bind_instead_of_map)]
        self.0.get(&sid).as_ref().and_then(|v| match v {
            Some(ndt) => Some(ndt),
            // If this method is used during type construction this case could be hit when it's actually valid
            // but all references are managed within `specta` so we can bypass this method and use `map` directly because we have `pub(crate)` access.
            None => {
                #[cfg(debug_assertions)]
                unreachable!("specta: `TypeCollection::get` found a type placeholder!");
                #[cfg(not(debug_assertions))]
                None
            }
        })
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
        v.sort_by(|x, y| x.name.cmp(&y.name).then(x.sid.cmp(&y.sid)));
        v.into_iter()
    }

    /// Return the unsorted iterator over the collection.
    pub fn into_unsorted_iter(&self) -> impl Iterator<Item = &NamedDataType> {
        self.0.iter().filter_map(|(_, ndt)| ndt.as_ref())
    }
}
