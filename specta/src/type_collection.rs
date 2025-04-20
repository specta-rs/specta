use std::{
    collections::HashMap,
    fmt,
    sync::atomic::{AtomicU64, Ordering},
};

use crate::{
    builder::NamedDataTypeBuilder,
    datatype::{NamedDataType, Reference},
    DataType, NamedType, SpectaID,
};

/// Define a set of types which can be exported together.
///
/// While exporting a type will add all of the types it depends on to the collection.
/// You can also construct your own collection to easily export a set of types together.
#[derive(Default)]
pub struct TypeCollection {
    // `None` indicates that the entry is a placeholder. It was reference and we are currently working out it's definition.
    pub(crate) map: HashMap<SpectaID, Option<NamedDataType>>,
    pub(crate) virtual_sid: AtomicU64,
}

impl fmt::Debug for TypeCollection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TypeCollection").field(&self.map).finish()
    }
}

impl TypeCollection {
    /// Register a [`NamedType`] with the collection.
    pub fn register<T: NamedType>(mut self) -> Self {
        T::definition(&mut self);
        self
    }

    /// Register a [`NamedType`] with the collection.
    pub fn register_mut<T: NamedType>(&mut self) -> &mut Self {
        T::definition(self);
        self
    }

    /// Declare a runtime defined type with the collection.
    ///
    /// Each [`Reference`] that is returned from a call to this function will be unique.
    /// You should only call this once and reuse the [`Reference`] if you intend to point to the same type.
    ///
    /// This method will return an error if the type_map is full. This will happen after `u64::MAX` calls to this method.
    pub fn declare(&mut self, ndt: NamedDataTypeBuilder) -> Result<Reference, ()> {
        let sid = crate::specta_id::r#virtual(saturating_add(&self.virtual_sid, 1));
        self.map.insert(
            sid,
            Some(NamedDataType {
                name: ndt.name,
                docs: ndt.docs,
                deprecated: ndt.deprecated,
                sid,
                module_path: ndt.module_path,
                location: ndt.location,
                generics: ndt.generics,
                inner: ndt.inner,
            }),
        );

        Ok(Reference {
            sid,
            generics: Default::default(), // TODO: We need this to be configurable.
            inline: false,
        })
    }

    /// Remove a type from the collection.
    pub fn remove(&mut self, sid: SpectaID) -> Option<NamedDataType> {
        self.map.remove(&sid).flatten()
    }

    /// Get a type from the collection.
    #[track_caller]
    pub fn get(&self, sid: SpectaID) -> Option<&NamedDataType> {
        #[allow(clippy::bind_instead_of_map)]
        self.map.get(&sid).as_ref().and_then(|v| match v {
            Some(ndt) => Some(ndt),
            // If this method is used during type construction this case could be hit when it's actually valid
            // but all references are managed within `specta` so we can bypass this method and use `map` directly because we have `pub(crate)` access.
            None => {
                // TODO: Probs bring this back???
                // #[cfg(debug_assertions)]
                // unreachable!("specta: `TypeCollection::get` found a type placeholder!");
                // #[cfg(not(debug_assertions))]
                None
            }
        })
    }

    /// Get the length of the collection.
    pub fn len(&self) -> usize {
        self.map.iter().filter_map(|(_, ndt)| ndt.as_ref()).count()
    }

    /// Sort the collection into a consistent order and return an iterator.
    ///
    /// The sort order is not necessarily guaranteed to be stable between versions but currently we sort by name.
    ///
    /// This method requires reallocating the map to sort the collection. You should prefer [Self::into_unsorted_iter] if you don't care about the order.
    pub fn into_sorted_iter(&self) -> impl Iterator<Item = NamedDataType> {
        let mut v = self
            .map
            .iter()
            .filter_map(|(_, ndt)| ndt.clone())
            .collect::<Vec<_>>();
        v.sort_by(|x, y| x.name.cmp(&y.name).then(x.sid.0.cmp(&y.sid.0)));
        v.into_iter()
    }

    /// Return the unsorted iterator over the collection.
    pub fn into_unsorted_iter(&self) -> impl Iterator<Item = &NamedDataType> {
        self.map.iter().filter_map(|(_, ndt)| ndt.as_ref())
    }

    /// Experimental: should we stabilise this? It's being used by `specta_typescript::Any`
    /// TODO: If we stablize this, we need to stop it from causing panics.
    #[doc(hidden)]
    pub fn placeholder(&mut self, sid: SpectaID) -> &mut Self {
        self.map.insert(sid, None);
        self
    }
}

fn saturating_add(atomic: &AtomicU64, value: u64) -> u64 {
    let mut current = atomic.load(Ordering::Relaxed);
    loop {
        let new_value = current.saturating_add(value);
        match atomic.compare_exchange_weak(current, new_value, Ordering::SeqCst, Ordering::Relaxed)
        {
            Ok(_) => break new_value,
            Err(previous) => current = previous,
        }
    }
}
