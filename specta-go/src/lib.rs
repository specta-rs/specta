//! [Go](https://go.dev/) language exporter.
//!
//! # Usage
//!
//! ```rust
//! use specta::{Type, TypeCollection};
//! use specta_go::Go;
//!
//! #[derive(Type)]
//! pub struct MyType {
//!     pub field: String,
//! }
//!
//! let mut types = TypeCollection::default();
//! types.register::<MyType>();
//!
//! Go::default()
//!     .export_to("./bindings.go", &types)
//!     .unwrap();
//! ```

mod config;
mod go;
mod primitives;
mod reserved_names;

pub use config::{BigIntExportBehavior, Error, Layout};
pub use go::Go;
