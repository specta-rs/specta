//! [TypeScript](https://www.typescriptlang.org) language exporter.
//!
//! # Usage
//!
//! Add `specta` and `specta-typescript` to your project:
//!
//! ```bash
//! cargo add specta@2.0.0-rc.22 --features derive,export
//! cargo add specta-typescript@0.0.9
//! cargo add specta-serde@0.0.9
//! ```
//!
//! Next copy the following into your `main.rs` file:
//!
//! ```rust
//! use specta::{Type, TypeCollection};
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
//! fn main() {
//!     let mut types = TypeCollection::default()
//!         // We don't need to specify `MyOtherType` because it's referenced by `MyType`
//!         .register::<MyType>();
//!
//!     Typescript::default()
//!         .export_to("./bindings.ts", &types)
//!         .unwrap();
//! }
//! ```
//!
//! Now your setup with Specta!
//!
//! If you get tired of listing all your types, checkout [`specta::export`].
//!
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png"
)]

mod error;
mod js_doc;
pub mod legacy;
pub mod primitives;
pub(crate) mod reserved_names;
mod static_types;
mod typescript;

pub use error::Error;
pub use js_doc::JSDoc;
pub use static_types::{Any, Unknown};
pub use typescript::{BigIntExportBehavior, Typescript};
