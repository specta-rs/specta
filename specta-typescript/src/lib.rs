//! [TypeScript](https://www.typescriptlang.org) language exporter.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png"
)]

mod error;
pub mod js_doc; // TODO
mod legacy;
pub mod primitives;
pub(crate) mod reserved_terms;
mod typescript;

pub use error::*; // TODO
pub use js_doc::*; // TODO
pub use legacy::*;
pub use typescript::{BigIntExportBehavior, Typescript};
