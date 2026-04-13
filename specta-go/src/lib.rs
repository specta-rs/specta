//! [Go](https://go.dev) language exporter for [Specta](specta).
//!
//! <div class="warning">
//! This crate is still in active development and is not yet ready for general purpose use!
//! </div>
//!
//! # Usage
//!
//! ```rust
//! use specta::Types;
//! use specta_go::Go;
//!
//! #[derive(specta::Type)]
//! pub struct MyType {
//!     pub field: String,
//! }
//!
//! let types = Types::default().register::<MyType>();
//!
//! Go::default()
//!     .export_to("./bindings.go", &types)
//!     .unwrap();
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

mod error;
mod go;
mod primitives;
mod reserved_names;

pub use error::Error;
pub use go::{Go, Layout, SerdeMode};
