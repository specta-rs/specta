//! Shared semantic mapping analysis and JS/TS runtime generation.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

mod v1;
pub mod v2;

pub use v1::*;
