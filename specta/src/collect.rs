use std::sync::{Mutex, OnceLock, PoisonError};

use crate::{Type, Types};

// Global type store for collecting custom types to export.
//
// We intentionally store functions over a `Types` directly to ensure any internal panics aren't done in CTOR.
#[allow(clippy::type_complexity)]
static TYPES: OnceLock<Mutex<Vec<fn(&mut Types)>>> = OnceLock::new();

/// Get the global type store containing all automatically collected types.
///
/// All types with the [`Type`](macro@crate::Type) macro will automatically be registered here unless they have been explicitly disabled with `#[specta(collect = false)]`.
///
/// Note that when enabling the `export` feature, you will not be able to enable the `unsafe_code` lint as [`ctor`] (which is used internally) is marked unsafe.
///
pub fn collect() -> Types {
    let types = TYPES
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(PoisonError::into_inner);

    let mut map = Types::default();
    for export in types.iter() {
        export(&mut map);
    }
    map
}

#[doc(hidden)]
pub mod internal {
    use super::*;

    // Called within ctor functions to register a type.
    pub fn register<T: Type>() {
        TYPES
            .get_or_init(Default::default)
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(|types| {
                // The side-effect of this is registering the type.
                T::definition(types);
            });
    }

    // We expose this for the macros
    #[cfg(feature = "collect")]
    pub use ctor;
}
