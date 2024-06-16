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
