use std::sync::{Mutex, OnceLock, PoisonError};

use crate::{Type, Types};

// Global type store for collecting custom types to export.
//
// We intentionally store functions over a `Types` directly to ensure any internal panics aren't done in CTOR.
#[allow(clippy::type_complexity)]
static TYPES: OnceLock<Mutex<Vec<fn(&mut Types)>>> = OnceLock::new();

/// Get the global type store containing all automatically collected types.
///
/// All non-generic types with the [`Type`](macro@crate::Type) macro will automatically be registered here unless they have been explicitly disabled with `#[specta(collect = false)]`.
///
/// Generic declarations are not collected automatically: there is no single
/// concrete instantiation to register, and choosing one would invent bounds the
/// Rust type may not satisfy. Register each instantiation you export with
/// [`Types::register`](crate::Types::register) instead.
///
/// Note that when enabling the `export` feature, you will not be able to enable the `unsafe_code` lint as [`small_ctor`] (which is used internally) is marked unsafe.
///
/// # Example
///
/// ```no_run
/// use specta::Type;
/// use specta_typescript::Typescript;
///
/// #[derive(Type)]
/// pub struct User {
///     id: String,
///     name: String,
/// }
///
/// #[derive(Type)]
/// pub struct Post {
///     id: String,
///     author: User,
///     comments: Vec<Comment>,
/// }
///
/// #[derive(Type)]
/// pub struct Comment {
///     body: String,
/// }
///
/// // This type can still derive `Type`, but it is left out of `specta::collect()`.
/// #[derive(Type)]
/// #[specta(collect = false)]
/// pub struct InternalMetrics {
///     latency_ms: u64,
/// }
///
/// fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let output = Typescript::default().export(&specta::collect(), specta_serde::Format)?;
///
///     println!("{output}");
///
///     Ok(())
/// }
/// ```
pub fn collect() -> Types {
    // Snapshot the registrations and release the lock before running any
    // `Type::definition`. A definition may legitimately register another type,
    // and holding the lock across arbitrary user code would deadlock if it
    // called back into `collect`.
    let exports = TYPES
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone();

    let mut map = Types::default();
    for export in exports {
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
                types.register_mut::<T>();
            });
    }

    // We expose this for the macros
    #[cfg(feature = "collect")]
    pub use small_ctor;
}
