#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::unwrap_used, clippy::panic)] // TODO: missing_docs
#![cfg_attr(docsrs2, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png"
)]

/// Provides the global type store and a method to export them to other languages.
#[cfg(feature = "export")]
#[cfg_attr(docsrs2, doc(cfg(feature = "export")))]
pub mod export;

#[cfg(feature = "serde")]
#[cfg_attr(docsrs2, doc(cfg(feature = "serde")))]
mod selection;
mod static_types;
mod type_collection;

#[cfg(feature = "serde")]
#[cfg_attr(docsrs2, doc(cfg(feature = "serde")))]
pub use selection::selection;
pub use static_types::{Any, Unknown};
pub use type_collection::TypeCollection;
