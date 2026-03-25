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
#[cfg(feature = "function")]
#[cfg_attr(docsrs, doc(cfg(feature = "function")))]
pub mod function;
#[doc(hidden)]
pub mod internal;
mod r#type;
mod types;

// TODO: Can we just move the trait here or `#[doc(inline)]`
pub use r#type::Type;
pub use types::{ResolvedTypes, Types};

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

#[doc(inline)]
#[cfg(all(feature = "derive", feature = "function"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "derive", feature = "function"))))]
pub use specta_macros::specta;

// TODO(v2): Remove this. This must be kept for Specta v1 as Tauri v2 depends on it.
#[doc(hidden)]
#[deprecated(note = "Migrate from `TypeMap` to `Types`")]
pub type TypeMap = Types;
