use std::{borrow::Borrow, collections::HashMap};

use crate::{NamedDataType, NamedType, SpectaID, TypeMap};

/// Define a set of types which can be exported together
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeCollection {
    types: HashMap<SpectaID, fn(&mut TypeMap) -> NamedDataType>,
}

impl Default for TypeCollection {
    fn default() -> Self {
        Self {
            types: HashMap::new(),
        }
    }
}

impl TypeCollection {
    /// Join another type collection into this one.
    pub fn extend(&mut self, collection: impl Borrow<TypeCollection>) -> &mut Self {
        self.types.extend(collection.borrow().types.iter());
        self
    }

    /// Register a type with the collection.
    pub fn register<T: NamedType>(&mut self) -> &mut Self {
        self.types
            .insert(T::sid(), |type_map| T::definition_named_data_type(type_map));
        self
    }

    /// Export all the types in the collection.
    pub fn export(&self, mut type_map: &mut crate::TypeMap) {
        for (sid, export) in self.types.iter() {
            let dt = export(&mut type_map);
            type_map.insert(*sid, dt);
        }
    }
}
