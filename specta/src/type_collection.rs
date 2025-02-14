use std::{
    borrow::Borrow,
    collections::{btree_map, BTreeMap},
    fmt,
    sync::atomic::AtomicU64,
};

use crate::{
    datatype::{NamedDataType, Reference},
    DataType, NamedType, SpectaID,
};

/// Define a set of types which can be exported together.
///
/// While exporting a type will add all of the types it depends on to the collection.
/// You can also construct your own collection to easily export a set of types together.
#[derive(Default, Clone, PartialEq)]
pub struct TypeCollection {
    // `None` indicates that the entry is a placeholder. It was reference and we are currently working out it's definition.
    pub(crate) map: BTreeMap<SpectaID, Option<NamedDataType>>,
    // #[cfg(feature = "serde_json")]
    // pub(crate) constants: BTreeMap<Cow<'static, str>, serde_json::Value>,
}

impl fmt::Debug for TypeCollection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TypeCollection").field(&self.map).finish()
    }
}

impl TypeCollection {
    /// Register a type with the collection.
    pub fn register<T: NamedType>(mut self) -> Self {
        T::definition(&mut self);
        self
    }

    /// Register a type with the collection.
    pub fn register_mut<T: NamedType>(&mut self) -> &mut Self {
        T::definition(self);
        self
    }

    /// Declare a custom type with the collection.
    #[doc(hidden)] // TODO: This isn't stable yet
    pub fn declare(&mut self, mut ndt: NamedDataType) -> Reference {
        // TODO: Proper id's
        static ID: AtomicU64 = AtomicU64::new(0);
        let sid = SpectaID {
            type_name: "virtual",
            hash: ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
        };

        // TODO: Do this and do it on `NamedDataTypeBuilder`
        // ndt.ext = Some(...);

        match &mut ndt.inner {
            // DataType::Nullable(data_type) => todo!(), // TODO: Recurse down?
            // DataType::Reference(reference) => todo!(),
            DataType::Struct(s) => {
                s.sid = Some(sid);
            }
            DataType::Enum(e) => {
                e.sid = Some(sid);
            }
            _ => {}
        }

        // TODO: If we wanna support generics via this API we will need a `ReferenceFactory`
        let reference = Reference {
            sid,
            generics: Default::default(),
            inline: false,
        };

        self.map.insert(sid, Some(ndt));

        reference
    }

    // TODO: `declare_mut`???

    // /// TODO
    // pub fn reference(&mut self, sid: SpectaID) -> Reference {
    //     // if self.map.get(&sid).is_none() {
    //     //     self.map.entry(sid).or_insert(None);
    //     //     let dt = T::definition_named_data_type(self);
    //     //     self.map.insert(sid, Some(dt));
    //     // }

    //     Reference { sid }

    // }

    //
    // #[doc(hidden)] // TODO: Make public
    // pub fn todo(&mut self, sid: SpectaID, inner: DataType) -> &mut Self {
    //     self.map.insert(sid, Some(NamedDataType {
    //         name: sid.type_name.into(),
    //         // TODO: How to configure this stuff?
    //         docs: "".into(),
    //         deprecated: None,
    //         ext: None, // TODO: Some(crate::datatype::NamedDataTypeExt { sid: (), impl_location: () })
    //         inner
    //     }));
    //     self
    // }

    // TODO: Implement in exporter and uncomment these
    // #[cfg(feature = "serde_json")]
    // pub fn constant<T: serde::Serialize>(self, name: impl Into<Cow<'static, str>>, value: T) -> Self {
    //     self.constants.insert(name.into(), serde_json::to_value(value).unwrap()); // TODO: Error handling
    //     self
    // }

    // #[cfg(feature = "serde_json")]
    // pub fn constant_mut<T: serde::Serialize>(&mut self, name: impl Into<Cow<'static, str>>, value: T) -> &mut Self {
    //     self.constants.insert(name.into(), serde_json::to_value(value).unwrap()); // TODO: Error handling
    //     self
    // }

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

    /// TODO
    ///
    #[doc(hidden)] // TODO: Should we stablise this? If we do we need to stop it causing panics
    pub fn placeholder(&mut self, sid: SpectaID) -> &mut Self {
        self.map.insert(sid, None);
        self
    }

    /// Join another type collection into this one.
    pub fn extend(mut self, collection: impl Borrow<Self>) -> Self {
        self.map
            .extend(collection.borrow().map.iter().map(|(k, v)| (*k, v.clone())));
        self
    }

    /// Join another type collection into this one.
    pub fn extend_mut(&mut self, collection: impl Borrow<Self>) -> &mut Self {
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
                // TODO: Probs bring this back???
                // #[cfg(debug_assertions)]
                // unreachable!("specta: `TypeCollection::get` found a type placeholder!");
                // #[cfg(not(debug_assertions))]
                None
            }
        })
    }

    pub fn len(&self) -> usize {
        self.map.len()
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
        let len = self.0.clone().filter(|(_, t)| t.is_some()).count();
        (len, Some(len))
    }
}
