//! [TypeScript](https://www.typescriptlang.org) language exporter.
//!
//! # Usage
//!
//! Add `specta` and `specta-typescript` to your project:
//!
//! ```bash
//! cargo add specta@2.0.0-rc.23 --features derive,export
//! cargo add specta-typescript@0.0.10
//! cargo add specta-serde@0.0.10
//! ```
//!
//! Next copy the following into your `main.rs` file:
//!
//! ```rust
//! use specta::{ResolvedTypes, Type, Types};
//! use specta_typescript::Typescript;
//!
//! #[derive(Type)]
//! pub struct MyType {
//!     pub field: MyOtherType,
//! }
//!
//!
//! #[derive(Type)]
//! pub struct MyOtherType {
//!     pub other_field: String,
//! }
//!
//! let mut types = Types::default()
//!     // We don't need to specify `MyOtherType` because it's referenced by `MyType`
//!     .register::<MyType>();
//! let resolved_types = ResolvedTypes::from_resolved_types(types);
//!
//! Typescript::default()
//!     .export_to("./bindings.ts", &resolved_types)
//!     .unwrap();
//! ```
//!
//! Now your setup with Specta!
//!
//! If you get tired of listing all your types manually? Checkout `specta::collect`!
//!
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

mod branded;
mod error;
mod exporter;
mod jsdoc;
mod legacy; // TODO: Remove this
mod opaque;
pub mod primitives;
mod references;
pub(crate) mod reserved_names;
mod types;
mod typescript;

pub use branded::Branded;
pub use error::Error;
pub use exporter::{BrandedTypeExporter, Exporter, FrameworkExporter, Layout};
pub use jsdoc::JSDoc;
pub use opaque::define;
pub use references::collect_references;
pub use types::{Any, Never, Unknown};
pub use typescript::Typescript;
