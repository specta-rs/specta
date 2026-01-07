//! [JSON Schema](https://json-schema.org) exporter and importer for Specta.
//!
//! This crate provides bidirectional conversion between Specta types and JSON Schema:
//! - Export Specta types to JSON Schema (Draft 7, 2019-09, or 2020-12)
//! - Import JSON Schema definitions as Specta DataTypes
//!
//! # Features
//!
//! - **Bidirectional conversion**: Export to JSON Schema and import from JSON Schema
//! - **Multiple schema versions**: Support for Draft 7 (default), Draft 2019-09, and Draft 2020-12
//! - **Serde integration**: Respect `#[serde(...)]` attributes via `specta-serde`
//! - **Flexible layouts**: Single file with `$defs` or separate files per type
//! - **schemars ecosystem**: Compatible with the schemars crate for interoperability
//!
//! # Usage
//!
//! ## Exporting to JSON Schema
//!
//! ```ignore
//! use specta::{Type, TypeCollection};
//! use specta_jsonschema::{JsonSchema, SchemaVersion};
//!
//! #[derive(Type)]
//! pub struct User {
//!     pub id: u32,
//!     pub name: String,
//!     pub email: Option<String>,
//! }
//!
//! fn main() {
//!     let types = TypeCollection::default()
//!         .register::<User>();
//!
//!     // Export to JSON Schema
//!     let schema = JsonSchema::default()
//!         .schema_version(SchemaVersion::Draft7)
//!         .export(&types)
//!         .unwrap();
//!
//!     println!("{}", schema);
//! }
//! ```
//!
//! ## With Serde Integration
//!
//! ```ignore
//! use specta::{Type, TypeCollection};
//! use specta_jsonschema::JsonSchema;
//!
//! #[derive(Type, serde::Serialize)]
//! #[serde(rename_all = "camelCase")]
//! pub struct User {
//!     pub user_id: u32,
//!     #[serde(rename = "fullName")]
//!     pub name: String,
//! }
//!
//! fn main() {
//!     let types = TypeCollection::default().register::<User>();
//!
//!     // Export with serde transformations
//!     JsonSchema::default()
//!         .with_serde_serialize()
//!         .export_to("./schema.json", &types)
//!         .unwrap();
//! }
//! ```
//!
//! ## Importing from JSON Schema
//!
//! ```ignore
//! use schemars::schema::Schema;
//! use specta_jsonschema::import::from_schema;
//!
//! let schema: Schema = serde_json::from_str(r#"{
//!     "type": "object",
//!     "properties": {
//!         "name": { "type": "string" },
//!         "age": { "type": "integer" }
//!     },
//!     "required": ["name"]
//! }"#).unwrap();
//!
//! let datatype = from_schema(&schema).unwrap();
//! ```
//!
//! ## Multiple Output Layouts
//!
//! ```ignore
//! use specta_jsonschema::{JsonSchema, Layout};
//!
//! // Single file with all types in $defs
//! JsonSchema::default()
//!     .layout(Layout::SingleFile)
//!     .export_to("./schema.json", &types)
//!     .unwrap();
//!
//! // Separate file per type, organized by module
//! JsonSchema::default()
//!     .layout(Layout::Files)
//!     .export_to("./schemas/", &types)
//!     .unwrap();
//! ```

#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]
#![allow(warnings)] // TODO: leaving this until it's implemented to avoid unnecessary warnings.

mod error;
pub mod import;
mod json_schema;
mod layout;
mod primitives;
mod schema_version;

pub use error::Error;
pub use json_schema::JsonSchema;
pub use layout::Layout;
pub use schema_version::SchemaVersion;

// Re-export commonly used types
pub use specta_serde::SerdeMode;

// Legacy function - kept for backward compatibility
#[deprecated(note = "Use import::from_schema instead")]
pub fn to_ast(schema: &schemars::schema::Schema) -> Result<specta::datatype::DataType, Error> {
    import::from_schema(schema)
}
