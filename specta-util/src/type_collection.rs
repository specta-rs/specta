use std::{borrow::Borrow, collections::HashMap, path::Path};

use specta::{Language, NamedDataType, NamedType, SpectaID, TypeMap};

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
    pub(crate) fn from_raw(types: HashMap<SpectaID, fn(&mut TypeMap) -> NamedDataType>) -> Self {
        Self { types }
    }

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

    /// Export all the types in the collection into the given type map.
    pub fn collect(&self, mut type_map: &mut TypeMap) {
        for (sid, export) in self.types.iter() {
            let dt = export(&mut type_map);
            type_map.insert(*sid, dt);
        }
    }

    /// TODO
    pub fn export<L: Language>(&self, language: L) -> Result<String, L::Error> {
        let mut type_map = TypeMap::default();
        self.collect(&mut type_map);
        language.export(type_map)
    }

    /// TODO
    pub fn export_to<L: Language>(
        &self,
        language: L,
        path: impl AsRef<Path>,
    ) -> Result<(), L::Error> {
        std::fs::write(path, self.export(language)?).map_err(Into::into)
    }
}
