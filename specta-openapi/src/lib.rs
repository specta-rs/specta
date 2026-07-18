//! [OpenAPI](https://spec.openapis.org/oas/) schema exporter for
//! [Specta](specta).
//!
//! The exporter turns a [`specta::Types`] collection into a valid OpenAPI
//! document whose reusable schemas live in `components.schemas`. Documents
//! target OpenAPI 3.1 by default; select [`OasVersion::V3_0`] to emit for
//! OpenAPI 3.0 consumers.
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
//! # Generator compatibility
//!
//! Some lowerings exist for the toolchains that consume the document rather
//! than for the specification: numeric schemas carry `format` hints, plain
//! string enums compact to the `type: string, enum: [...]` form, and integer
//! bounds beyond the signed 64-bit range are carried in `x-specta-*`
//! extensions, since mainstream generators parse bounds into signed 64-bit
//! integers and silently wrap anything wider.
//!
//! # OpenAPI 3.0 compatibility
//!
//! OpenAPI 3.1's schema dialect is full JSON Schema, so every Specta shape is
//! expressible in it. OpenAPI 3.0 uses an older, restricted dialect: under
//! [`OasVersion::V3_0`] the default [`SchemaMode::Strict`] returns an error
//! for structural schema features which that specification cannot express,
//! and [`SchemaMode::Compatible`] emits a useful approximation instead,
//! retaining the original constraints in `x-specta-*` extensions. This
//! affects nullable references — an `Option<T>` over a named type, which
//! OpenAPI 3.0 cannot mark `nullable` beside a `$ref` — along with null-only
//! types, heterogeneous tuples, constrained map keys, and closed flattened
//! intersections.

#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

mod error;
mod openapi;
mod operation;
mod paths;
mod resolve;
mod transform;

pub use error::Error;
pub use openapi::{OasVersion, OpenApi, OutputFormat, SchemaMode};
pub use operation::{Method, Operation, Param};
