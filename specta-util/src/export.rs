use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock, PoisonError},
};

use specta::{datatype::NamedDataType, NamedType, SpectaID, TypeCollection};

// Global type store for collecting custom types to export.
static TYPES: OnceLock<Mutex<HashMap<SpectaID, fn(&mut TypeCollection) -> NamedDataType>>> =
    OnceLock::new();

/// Get the global type store containing all registered types.
pub fn export() -> TypeCollection {
    // TODO: Make `TYPES` should just hold a `TypeCollection` directly???
    let types = TYPES
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(PoisonError::into_inner);

    let mut map = TypeCollection::default();
    for (id, export) in types.iter() {
        let dt = export(&mut map);
        map.insert(*id, dt);
    }
    map
}

#[doc(hidden)]
pub mod internal {
    use std::sync::PoisonError;

    use super::*;

    // Called within ctor functions to register a type.
    #[doc(hidden)]
    pub fn register<T: NamedType>() {
        let mut type_map = TYPES
            .get_or_init(Default::default)
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        type_map.insert(T::sid(), |type_map| T::definition_named_data_type(type_map));
    }

    // We expose this for the macros
    pub use ctor::ctor;
}
