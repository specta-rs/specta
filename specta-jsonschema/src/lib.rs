//! [JSON Schema](https://json-schema.org) exporter for [Specta](specta).
//!
//! # Usage
//!
//! ```rust
//! use specta::{Type, Types};
//! use specta_jsonschema::JsonSchema;
//!
//! #[derive(Type)]
//! pub struct User {
//!     pub id: u32,
//!     pub name: String,
//! }
//!
//! let types = Types::default().register::<User>();
//! let schema = JsonSchema::default().export(&types, specta_serde::Format).unwrap();
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

mod error;
mod exporter;
mod jsonschema;
mod render;
mod schema_version;

pub use error::Error;
pub use jsonschema::JsonSchema;
pub use schema_version::SchemaVersion;
