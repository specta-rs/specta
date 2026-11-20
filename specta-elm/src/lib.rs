//! [Elm](https://elm-lang.org) language exporter.
//!
//! `specta-elm` was originally written by [vivi](https://codeberg.org/itsvivi)
//! and published as a standalone crate. It now lives in the Specta workspace and
//! is maintained alongside the other exporters.
//!
//! # Usage
//!
//! Add `specta` and `specta-elm` to your project:
//!
//! ```bash
//! cargo add specta@2.0.0-rc.28 --features derive
//! cargo add specta-elm
//! ```
//!
//! Then export your types into an existing Elm project:
//!
//! ```no_run
//! use specta::{Type, Types};
//! use specta_elm::Elm;
//!
//! #[derive(Type)]
//! struct User {
//!     id: u32,
//!     name: String,
//!     role: Role,
//! }
//!
//! #[derive(Type)]
//! enum Role {
//!     Guest,
//!     Admin { level: u8 },
//! }
//!
//! let types = Types::default().register::<User>();
//!
//! Elm::init(types, "./frontend")
//!     .unwrap()
//!     .export(specta_elm::exporter::SingleFile("Types"))
//!     .unwrap();
//! ```
//!
//! `specta-elm` writes into the source directories declared by the Elm
//! project's `elm.json`, and relies on the Elm compiler to type check the
//! result.
//!
//! # Unsupported types
//!
//! Elm has no representation for generics, recursive types, intersection types
//! (such as Serde's `flatten`), tuples longer than three elements, or integers
//! which don't fit in Elm's `Int`. Exporting one returns an [`Error`] rather
//! than writing code which won't compile.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

mod elm;
mod error;
mod export;
mod module;
mod types;

pub use elm::{Elm, Project};
pub use error::Error;
pub use export::{Exporter, IntoExporter};

/// Output targets for an export.
pub mod exporter {
    pub use crate::export::IntoExporter as Output;
    pub use crate::export::MsgFileOutput as MsgFile;
    pub use crate::export::SingleFileOutput as SingleFile;
}
