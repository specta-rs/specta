#![feature(f128)]
#![feature(f16)]
#![doc = include_str!("./docs.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

#[cfg(feature = "collect")]
#[cfg_attr(docsrs, doc(cfg(feature = "collect")))]
#[doc(hidden)]
pub mod collect;
pub mod datatype;
#[cfg(feature = "function")]
#[cfg_attr(docsrs, doc(cfg(feature = "function")))]
pub mod function;
#[doc(hidden)]
pub mod internal;
mod r#type;
mod type_collection;

// TODO: Can we just move the trait here or `#[doc(inline)]`
pub use r#type::Type;
pub use type_collection::TypeCollection;

#[doc(inline)]
#[cfg(feature = "collect")]
#[cfg_attr(docsrs, doc(cfg(feature = "collect")))]
pub use collect::collect;

#[doc(inline)]
#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use specta_macros::Type;

#[doc(inline)]
#[cfg(all(feature = "derive", feature = "function"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "derive", feature = "function"))))]
pub use specta_macros::specta;

// TODO(v2): Remove this. This must be kept for Specta v1 as Tauri v2 depends on it.
#[doc(hidden)]
#[deprecated(note = "Migrate from `TypeMap` to `TypeCollection`")]
pub type TypeMap = TypeCollection;
