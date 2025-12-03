use std::sync::{Mutex, OnceLock, PoisonError};

use crate::{Type, TypeCollection};

// Global type store for collecting custom types to export.
static TYPES: OnceLock<Mutex<Vec<fn(&mut TypeCollection)>>> = OnceLock::new();

/// Get the global type store containing all automatically registered types.
///
/// All types with the [`Type`](macro@specta::Type) macro will automatically be registered here unless they have been explicitly disabled with `#[specta(export = false)]`.
///
/// Note that when enabling the `export` feature, you will not be able to enable the `unsafe_code` lint as [`ctor`](https://docs.rs/ctor) (which is used internally) is marked unsafe.
///
pub fn export() -> TypeCollection {
    let types = TYPES
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(PoisonError::into_inner);

    let mut map = TypeCollection::default();
    for export in types.iter() {
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
    pub fn register<T: Type>() {
        let mut types = TYPES
            .get_or_init(Default::default)
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        types.push(|types| {
            // The side-effect of this is registering the type.
            T::definition(types);
        });
    }

    // We expose this for the macros
    #[cfg(feature = "export")]
    pub use ::ctor;
}
