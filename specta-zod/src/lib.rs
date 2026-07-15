//! [Zod](https://zod.dev) language exporter for [Specta](specta).
//!
//! # Usage
//!
//! ```rust,no_run
//! use specta::{Type, Types};
//! use specta_zod::Zod;
//!
//! #[derive(Type)]
//! pub struct User {
//!     pub id: u32,
//!     pub name: String,
//! }
//!
//! let types = Types::default().register::<User>();
//! Zod::default()
//!     .export_to("./schemas.ts", &types, specta_serde::Format)
//!     .unwrap();
//! ```
//!
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

mod error;
mod opaque;
pub mod primitives;
mod references;
mod reserved_names;
mod types;
mod zod;

pub use error::Error;
pub use opaque::define;
pub use references::collect_references;
pub use types::{Any, Never, Unknown};
pub use zod::{FrameworkExporter, Layout, Zod};
