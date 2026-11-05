//! [Java](https://dev.java) language exporter for [Specta](specta).
//!
//! The exporter targets Java 17 and renders Rust structs as records, fieldless
//! enums as Java enums, and data-carrying enums as sealed interfaces.
//! [`JavaVersion::Java11`] renders the same models without records or sealed
//! types: structs become final classes with a canonical constructor, accessors,
//! `equals`, `hashCode`, and `toString`, and data-carrying enums become abstract
//! classes with a private constructor and one nested final subclass per variant.
//! It generates data models rather than serializer-specific annotations or codecs;
//! configure Jackson, Gson, or another serializer separately when wire-name mapping
//! is required.
//!
//! # Usage
//!
//! ```rust
//! use specta::{Type, Types};
//! use specta_java::Java;
//!
//! #[derive(Type)]
//! struct User {
//!     name: String,
//! }
//!
//! let types = Types::default().register::<User>();
//! let java = Java::default().export(&types, specta_serde::Format)?;
//! assert!(java.contains("record User"));
//! # Ok::<(), specta_java::Error>(())
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

mod error;
mod java;
mod render;
mod reserved_names;

pub use error::{Error, ErrorTraceFrame};
pub use java::{Java, JavaVersion, Layout, OptionalStyle};
