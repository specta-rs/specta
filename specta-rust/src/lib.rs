//! [Rust](https://www.rust-lang.org) language exporter for [Specta](specta).
//!
//! The exporter recreates Specta's language-neutral type graph as Rust source.
//! It is useful for generated API crates, fixtures, and inspecting the shape a
//! [`specta::Format`] produces.
//!
//! Rust is the one target that shares Specta's source language, so [`Identity`]
//! reproduces the source types verbatim — serde container attributes and all.
//! A serialization [`Format`](specta::Format) such as `specta_serde::Format`
//! instead lowers the serde representation into the graph's shape, which is
//! useful for inspecting that shape but can no longer be reproduced as an
//! equivalent Rust type.
//!
//! Formatters may also produce structural wire shapes that Rust cannot express
//! as a type (for example intersections created by `serde(flatten)`). Those
//! shapes return a contextual [`Error`] instead of silently generating a
//! different Rust type. Exporter-specific opaque references can be handled with
//! [`Rust::opaque_type`].
//!
//! # Usage
//!
//! ```rust
//! use specta::{Type, Types};
//! use specta_rust::{Identity, Rust};
//!
//! #[derive(Type)]
//! struct User {
//!     name: String,
//! }
//!
//! let types = Types::default().register::<User>();
//! let source = Rust::default().export(&types, Identity).unwrap();
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
pub use exporter::{Identity, Layout, Rust};
