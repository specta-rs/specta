//! [Serde](https://serde.rs) support for Specta
//!
//! This crates takes the types with there attribute metadata returned from the macros and validates that the type is a valid Serde type and then applies any transformations needed.
//!
//! # Usage
//!
//! ```
//! let mut types = specta::TypeCollection::default();
//! specta_serde::apply(&mut types).unwrap();
//! // Use your `types` as normal with a language exporter
//! ```
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png"
)]

mod error;
mod apply;

pub use error::Error;
pub use apply::apply;
