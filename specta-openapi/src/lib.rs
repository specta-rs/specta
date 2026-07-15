//! [OpenAPI 3.0](https://spec.openapis.org/oas/v3.0.3) schema exporter for
//! [Specta](specta).
//!
//! The exporter turns a [`specta::Types`] collection into a valid OpenAPI
//! document whose reusable schemas live in `components.schemas`.
//!
//! # Usage
//!
//! Add the core, serde-format, and OpenAPI crates:
//!
//! ```bash
//! cargo add specta --features derive
//! cargo add specta-serde
//! cargo add specta-openapi
//! ```
//!
//! ```rust
//! use specta::{Type, Types};
//! use specta_openapi::OpenApi;
//!
//! #[derive(Type)]
//! struct User {
//!     id: u32,
//!     name: String,
//! }
//!
//! let types = Types::default().register::<User>();
//! let document = OpenApi::default()
//!     .title("Example API")
//!     .version("1.0.0")
//!     .export(&types, specta_serde::Format)
//!     .unwrap();
//! assert!(document.contains("User"));
//! ```
//!
//! # OpenAPI 3.0 compatibility
//!
//! OpenAPI 3.0 uses an older, restricted JSON Schema dialect. The default
//! [`SchemaMode::Strict`] returns an error for structural schema features which
//! the specification cannot express. Opt into [`SchemaMode::Compatible`] to emit a useful
//! approximation and retain the original constraints in `x-specta-*`
//! extensions. This affects nullable references — an `Option<T>` over a named
//! type, which OpenAPI 3.0 cannot mark `nullable` beside a `$ref` — along with
//! exact 64-bit integer bounds, null-only types, heterogeneous tuples,
//! constrained map keys, and closed flattened intersections.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

mod error;
mod openapi;
mod transform;

pub use error::Error;
pub use openapi::{OpenApi, OutputFormat, SchemaMode};
pub use openapiv3::{Components, OpenAPI, ReferenceOr, Schema};
