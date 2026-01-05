#![doc = include_str!("./docs.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png"
)]

pub mod builder;
#[cfg(feature = "collect")]
#[cfg_attr(docsrs, doc(cfg(feature = "collect")))]
#[doc(hidden)]
pub mod collect;
pub mod datatype;
#[cfg(feature = "function")]
#[cfg_attr(docsrs, doc(cfg(feature = "function")))]
pub mod function;
#[doc(hidden)]
pub mod internal;
mod r#type;
mod type_collection;

// TODO: Can we just move the trait here or `#[doc(inline)]`
pub use r#type::Type;
pub use type_collection::TypeCollection;

#[doc(inline)]
#[cfg(feature = "collect")]
#[cfg_attr(docsrs, doc(cfg(feature = "collect")))]
pub use collect::collect;

#[doc(inline)]
#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use specta_macros::Type;

#[doc(inline)]
#[cfg(all(feature = "derive", feature = "function"))]
#[cfg_attr(docsrs, doc(cfg(all(feature = "derive", feature = "function"))))]
pub use specta_macros::specta;

// TODO: Remove this for major
// This existing is really a mistake but it's depended on by the Tauri alpha's so keeping it for now.
#[doc(hidden)]
pub use datatype::DataType;

// TODO: Remove this for major
// To ensure Tauri doesn't have a breaking change.
#[doc(hidden)]
pub type TypeMap = TypeCollection;
