#![doc = include_str!("./docs.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]
#![cfg_attr(is_nightly, feature(f16))]
#![cfg_attr(is_nightly, feature(f128))]

#[cfg(feature = "collect")]
#[cfg_attr(docsrs, doc(cfg(feature = "collect")))]
#[doc(hidden)]
pub mod collect;
pub mod datatype;
mod format;
mod hash;
mod r#type;
mod types;

#[doc(inline)]
pub use format::{Format, FormatError};
#[doc(inline)]
pub use r#type::Type;
#[doc(inline)]
pub use types::Types;

#[doc(hidden)]
pub use types::GenericSubstitutionSlot;

#[doc(inline)]
#[cfg(feature = "collect")]
#[cfg_attr(docsrs, doc(cfg(feature = "collect")))]
pub use collect::collect;

#[doc(inline)]
#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use specta_macros::Type;

#[doc(hidden)]
#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use specta_macros::parse_type_from_lit;

// TODO(v3): Remove this. Tauri v2's `specta` feature implements this trait for its
// `State`, `AppHandle`, `Window`, `Webview`, and `WebviewWindow` command arguments.
// Function type collection itself has been removed, so only this shim remains.
#[cfg(feature = "function")]
#[cfg_attr(docsrs, doc(cfg(feature = "function")))]
#[doc(hidden)]
pub mod function {
    use crate::{Type, Types, datatype::DataType};

    /// Deprecated compatibility shim for Tauri v2.
    #[deprecated(
        note = "function type collection has been removed; this trait only exists for Tauri v2 compatibility"
    )]
    pub trait FunctionArg {
        /// Returns the argument's datatype, or `None` to omit it.
        fn to_datatype(types: &mut Types) -> Option<DataType>;
    }

    #[allow(deprecated)]
    impl<T: Type> FunctionArg for T {
        fn to_datatype(types: &mut Types) -> Option<DataType> {
            Some(T::definition(types))
        }
    }
}

// TODO(v3): Remove this. This must be kept for Specta v1 as Tauri v2 depends on it.
#[doc(hidden)]
#[deprecated(note = "Migrate from `TypeMap` to `Types`")]
pub type TypeMap = Types;
