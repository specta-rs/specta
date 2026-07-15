//! [Kotlin](https://kotlinlang.org) language exporter for [Specta](specta).
//!
//! This crate exports [`specta::Types`] as idiomatic Kotlin declarations. It supports
//! Kotlin collections, nullable types, generics, data classes, enums, and sealed hierarchies.
//!
//! Generated source targets Kotlin 1.9 or newer (`data object` is used for unit declarations).
//! Kotlinx annotations are opt-in through [`Serialization::Kotlinx`] and require the Kotlin
//! serialization compiler plugin plus `kotlinx-serialization-core`. Shapes for which standard
//! Kotlinx serialization would disagree with Rust/Serde (including tuples and enums) are
//! rejected instead of generating a wire-incompatible codec. Plain declaration export remains
//! available for every shape that can be usefully approximated in Kotlin's nominal type system.
//!
//! # Usage
//!
//! ```rust
//! use specta::{Type, Types};
//! use specta_kotlin::Kotlin;
//!
//! #[derive(Type)]
//! struct User {
//!     name: String,
//! }
//!
//! let types = Types::default().register::<User>();
//! let source = Kotlin::default()
//!     .export(&types, specta_serde::Format)
//!     .unwrap();
//! # let _ = source;
//! ```
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

mod error;
mod kotlin;
mod render;
mod reserved_names;

pub use error::Error;
pub use kotlin::{IndentStyle, Kotlin, Layout, NamingConvention, Serialization, UnknownType};
