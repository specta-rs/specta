//! [Rust](https://www.rust-lang.org) language exporter for [Specta](specta).
//!
//! The exporter recreates Specta's language-neutral type graph as Rust source.
//! It is useful for generated API crates, fixtures, and inspecting the shape a
//! [`specta::Format`] produces.
//!
//! Formatters may produce structural wire shapes that Rust cannot express as a
//! type (for example intersections created by `serde(flatten)`). Those shapes
//! return a contextual [`Error`] instead of silently generating a different
//! Rust type. Exporter-specific opaque references can be handled with
//! [`Rust::opaque_type`].
//!
//! # Usage
//!
//! ```rust
//! use specta::{Type, Types};
//! use specta_rust::Rust;
//!
//! #[derive(Type)]
//! struct User {
//!     name: String,
//! }
//!
//! let types = Types::default().register::<User>();
//! let source = Rust::default().export(&types, specta_serde::Format).unwrap();
//! assert!(source.contains("pub struct User"));
//! ```
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

mod error;
mod exporter;

pub use error::Error;
pub use exporter::{Layout, Rust};
