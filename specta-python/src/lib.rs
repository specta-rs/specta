//! [Python](https://www.python.org) 3.13 type-hint exporter for [Specta](specta).
//!
//! The exporter uses lazy PEP 695 type aliases and [`typing.TypedDict`](https://docs.python.org/3/library/typing.html#typing.TypedDict)
//! to accurately model serialized dictionary shapes, including non-identifier and optional keys.
//!
//! # Usage
//!
//! ```rust
//! use specta::{Type, Types};
//! use specta_python::Python;
//!
//! #[derive(Type)]
//! struct User {
//!     id: u64,
//!     name: String,
//! }
//!
//! let types = Types::default().register::<User>();
//! let bindings = Python::default()
//!     .export(&types, specta_serde::Format)
//!     .unwrap();
//! assert!(bindings.contains("class User(_specta_typing.TypedDict)"));
//! ```
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

mod error;
mod exporter;
mod opaque;
pub mod primitives;
mod reserved_names;
mod types;

pub use error::{Error, ErrorTraceFrame};
pub use exporter::{Layout, Python};
pub use opaque::define;
pub use types::{Any, Never, Unknown};
