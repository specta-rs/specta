#![doc = include_str!("./docs.md")]
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::unwrap_used, clippy::panic)] // TODO: missing_docs
#![allow(clippy::module_inception)]
#![cfg_attr(docsrs2, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png"
)]

#[doc(hidden)]
pub mod internal;

/// Types related to working with [`DataType`](crate::DataType). Exposed for advanced users.
pub mod datatype;
/// Support for exporting Rust functions.
#[cfg(feature = "function")]
#[cfg_attr(docsrs2, doc(cfg(feature = "function")))]
pub mod function;
/// Contains [`Type`] and everything related to it, including implementations and helper macros
pub mod r#type;

#[doc(hidden)] // TODO: Should we actually do this? I think not
pub use datatype::*;
pub use r#type::*;

/// Implements [`Type`] for a given struct or enum.
///
/// ## Example
///
/// ```rust
/// use specta::Type;
///
/// // Use it on structs
/// #[derive(Type)]
/// pub struct MyCustomStruct {
///     pub name: String,
/// }
///
/// #[derive(Type)]
/// pub struct MyCustomStruct2(String, i32, bool);
///
/// // Use it on enums
/// #[derive(Type)]
/// pub enum MyCustomType {
///     VariantOne,
///     VariantTwo(String, i32),
///     VariantThree { name: String, age: i32 },
/// }
/// ```
pub use specta_macros::Type;

/// Prepares a function to have its types extracted using [`fn_datatype`]
///
/// ## Example
///
/// ```rust
/// #[specta::specta]
/// fn my_function(arg1: i32, arg2: bool) -> &'static str {
///     "Hello World"
/// }
/// ```
#[cfg(feature = "function")]
#[cfg_attr(docsrs2, doc(cfg(feature = "function")))]
pub use specta_macros::specta;

#[cfg(doctest)]
doc_comment::doctest!("../../README.md");
