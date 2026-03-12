//! [ReScript](https://rescript-lang.org) language exporter for [Specta](specta).
//!
//! Exports Rust types to ReScript type definitions, including built-in support
//! for ReScript's `result<ok, err>` type.
//!
//! # Usage
//!
//! ```bash
//! cargo add specta@2.0.0-rc.23 --features derive
//! cargo add specta-rescript@0.0.1
//! cargo add specta-serde@0.0.10
//! ```
//!
//! ```rust
//! use specta::{Type, TypeCollection};
//! use specta_rescript::ReScript;
//!
//! #[derive(Type)]
//! pub struct MyType {
//!     pub field: String,
//! }
//!
//! let types = TypeCollection::default().register::<MyType>();
//! ReScript::default().export_to("./Types.res", &types).unwrap();
//! ```
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

mod error;
mod primitives;
mod rescript;

pub use error::Error;
pub use rescript::ReScript;
pub use specta_serde::SerdeMode;
