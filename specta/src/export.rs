use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock, PoisonError},
};

use crate::{NamedType, SpectaID, TypeCollection};

// Global type store for collecting custom types to export.
static TYPES: OnceLock<Mutex<HashMap<SpectaID, fn(&mut TypeCollection)>>> =
    OnceLock::new();

/// Get the global type store containing all registered types.
pub fn export() -> TypeCollection {
    // TODO: Make `TYPES` should just hold a `TypeCollection` directly???
    let types = TYPES
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(PoisonError::into_inner);

    let mut map = TypeCollection::default();
    for (_, export) in types.iter() {
        export(&mut map);
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
        let mut types = TYPES
            .get_or_init(Default::default)
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        types.insert(T::ID, |types| {
            // The side-effect of this is registering the type.
            T::definition(types);
        });
    }

    // We expose this for the macros
    pub use ctor::ctor;
}
