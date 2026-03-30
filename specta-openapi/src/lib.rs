//! [OpenAPI](https://www.openapis.org) language exporter for [Specta](specta).
//!
//! This crate exports [`specta::ResolvedTypes`] into [`openapiv3::OpenAPI`] documents.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

mod error;
mod openapi;

pub use error::Error;
pub use openapi::{GenericHandling, OpenAPI, OpenApiVersion};
