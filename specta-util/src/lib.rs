//! Extended functionality for Specta.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png"
)]

#[cfg(feature = "serde")]
#[cfg_attr(docsrs, doc(cfg(feature = "serde")))]
mod selection;

#[cfg(feature = "unstable_json_macro")]
#[cfg_attr(docsrs, doc(cfg(feature = "unstable_json_macro")))]
pub mod json;
