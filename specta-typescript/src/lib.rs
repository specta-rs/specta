//! [TypeScript](https://www.typescriptlang.org) language exporter.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png"
)]

pub(crate) mod utils;
pub(crate) mod constants;
pub mod primitives;
pub mod comments;
mod context;
mod error;
pub mod formatter;
pub mod js_doc; // TODO: Remove in favor of `specta-jsdoc`
mod typescript;
mod legacy;

pub use legacy::*;
pub use context::*;
pub use error::*;
pub use typescript::*;
