use std::{
    borrow::Borrow,
    collections::{btree_map, BTreeMap},
    fmt,
};

use crate::{datatype::NamedDataType, DataType, NamedType, SpectaID};

/// Define a set of types which can be exported together.
///
/// While exporting a type will add all of the types it depends on to the collection.
/// You can also construct your own collection to easily export a set of types together.
#[derive(Default, Clone, PartialEq)]
pub struct TypeCollection {
    // `None` indicates that the entry is a placeholder. It was reference and we are currently working out it's definition.
    pub(crate) map: BTreeMap<SpectaID, Option<NamedDataType>>,
    // A stack of types that are currently being flattened. This is used to detect cycles.
    pub(crate) flatten_stack: Vec<SpectaID>,
}

impl fmt::Debug for TypeCollection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TypeCollection").field(&self.map).finish()
    }
}

impl TypeCollection {
    /// Register a type with the collection.
    pub fn register<T: NamedType>(mut self) -> Self {
        let def = T::definition_named_data_type(&mut self);
        self.map.insert(T::sid(), Some(def));
        self
    }

    /// Register a type with the collection.
    pub fn register_mut<T: NamedType>(&mut self) -> &mut Self {
        let def = T::definition_named_data_type(self);
        self.map.insert(T::sid(), Some(def));
        self
    }

    /// Register a type with the collection.
    #[doc(hidden)] // TODO: Make public
    pub fn todo(&mut self, sid: SpectaID, inner: DataType) -> &mut Self {
        self.map.insert(sid, Some(NamedDataType {
            name: sid.type_name.into(),
            // TODO: How to configure this stuff?
            docs: "".into(),
            deprecated: None,
            ext: None, // TODO: Some(crate::datatype::NamedDataTypeExt { sid: (), impl_location: () })
            inner
        }));
        self
    }

    /// Insert a type into the collection.
    /// You should prefer to use `TypeCollection::register` as it ensures all invariants are met.
    ///
    /// When using this method it's the responsibility of the caller to:
    ///  - Ensure the `SpectaID` and `NamedDataType` are correctly matched.
    ///  - Ensure the same `TypeCollection` was used when calling `NamedType::definition_named_data_type`.
    /// Not honoring these rules will result in a broken collection.
    pub fn insert(&mut self, sid: SpectaID, def: NamedDataType) -> &mut Self {
        self.map.insert(sid, Some(def));
        self
    }

    /// Join another type collection into this one.
    pub fn extend(&mut self, collection: impl Borrow<Self>) -> &mut Self {
        self.map
            .extend(collection.borrow().map.iter().map(|(k, v)| (*k, v.clone())));
        self
    }

    /// Remove a type from the collection.
    pub fn remove(&mut self, sid: SpectaID) -> Option<NamedDataType> {
        self.map.remove(&sid).flatten()
    }

    #[track_caller]
    pub fn get(&self, sid: SpectaID) -> Option<&NamedDataType> {
        #[allow(clippy::bind_instead_of_map)]
        self.map.get(&sid).as_ref().and_then(|v| match v {
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
}

impl<'a> IntoIterator for &'a TypeCollection {
    type Item = (SpectaID, &'a NamedDataType);
    type IntoIter = TypeCollectionInterator<'a>;

    fn into_iter(self) -> Self::IntoIter {
        TypeCollectionInterator(self.map.iter())
    }
}

// Sealed
pub struct TypeCollectionInterator<'a>(btree_map::Iter<'a, SpectaID, Option<NamedDataType>>);

impl<'a> ExactSizeIterator for TypeCollectionInterator<'a> {}

impl<'a> Iterator for TypeCollectionInterator<'a> {
    type Item = (SpectaID, &'a NamedDataType);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (sid, ndt) = self.0.next()?;
            if let Some(ndt) = ndt {
                return Some((*sid, ndt));
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.0.clone().filter(|(_, t)| t.is_none()).count()))
    }
}
