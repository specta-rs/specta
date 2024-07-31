#![doc = include_str!("./docs.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png"
)]

pub mod datatype;
#[cfg(feature = "function")]
#[cfg_attr(docsrs, doc(cfg(feature = "function")))]
pub mod function;
#[doc(hidden)]
pub mod internal;
mod language;
mod specta_id;
mod r#type;
mod type_map;

// TODO: Can we just move the trait here or `#[doc(inline)]`
pub use r#type::{Flatten, Generics, NamedType, Type};
// #[doc(inline)]
pub use specta_id::{ImplLocation, SpectaID};
pub use type_map::TypeMap;

pub use language::Language;

#[doc(inline)]
#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use specta_macros::Type;

#[doc(inline)]
#[cfg(feature = "function")]
#[cfg_attr(docsrs, doc(cfg(feature = "function")))]
pub use specta_macros::specta;

// This existing is really a mistake but it's depended on by the Tauri alpha's so keeping it for now.
#[doc(hidden)]
pub use datatype::DataType;
