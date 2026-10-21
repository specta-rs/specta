//! [ReScript](https://rescript-lang.org) language exporter for [Specta](specta).
//!
//! Exports Rust types to ReScript type definitions, including built-in support
//! for ReScript's `result<ok, err>` type.
//!
//! # Usage
//!
//! ```bash
//! cargo add specta@2.0.0-rc.27 --features derive
//! cargo add specta-rescript@0.0.2
//! cargo add specta-serde@0.0.14
//! ```
//!
//! ```rust
//! use specta::{Type, Types};
//! use specta_rescript::ReScript;
//!
//! #[derive(Type)]
//! pub struct MyType {
//!     pub field: String,
//! }
//!
//! let types = Types::default().register::<MyType>();
//! let output = ReScript::default().export(&types).unwrap();
//! assert!(output.contains("type myType"));
//! ```
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

mod error;
/// Low-level rendering helpers.
pub mod primitives;
mod rescript;
mod toposort;

pub use error::Error;
pub use rescript::ReScript;
