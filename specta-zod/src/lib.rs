//! [Zod](https://zod.dev) language exporter for [Specta](specta).
//!
//! <div class="warning">
//! This crate is still in active development and is not yet ready for general purpose use!
//! </div>
//!
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/specta-rs/specta/raw/main/.github/logo-128.png"
)]

mod compat;
mod error;
mod opaque;
pub mod primitives;
mod references;
mod reserved_names;
mod types;
mod zod;

pub use error::Error;
pub use opaque::define;
pub use types::{Any, Never, Unknown};
pub use zod::{BigIntExportBehavior, FrameworkExporter, Layout, Zod};
