//! Extended functionality for [Specta](specta).

#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
mod selection;

mod array;
mod remapper;

#[doc(hidden)]
#[cfg(feature = "serde")]
pub mod __private {
    pub use serde;
    pub use specta;
}

pub use array::FixedArray;
pub use remapper::Remapper;
