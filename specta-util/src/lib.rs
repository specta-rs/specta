//! TODO
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png"
)]

/// Provides the global type store and a method to export them to other languages.
#[cfg(feature = "export")]
#[cfg_attr(docsrs, doc(cfg(feature = "export")))]
#[doc(hidden)]
pub mod export;

#[cfg(feature = "export")]
#[cfg_attr(docsrs, doc(cfg(feature = "export")))]
pub use export::export;

#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
mod selection;
mod static_types;

#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
// pub use selection::selection;
pub use static_types::{Any, Unknown};
