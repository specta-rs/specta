//! Easily export your Rust types to other languages
//!
//! Specta provides a system for type introspection and a set of language exporter which allows you to export your Rust types to other languages!
//! Currently we only support exporting to [TypeScript](https://www.typescriptlang.org).
//!
//! ## Example
//! ```rust
//! use specta::{*, ts::*};
//!
//! #[derive(Type)]
//! pub struct MyCustomType {
//!    pub my_field: String,
//! }
//!
//! fn main() {
//!     assert_eq!(
//!         ts::export::<MyCustomType>(&ExportConfiguration::default()).unwrap(),
//!         "export type MyCustomType = { my_field: string }".to_string()
//!     );
//! }
//! ```
//!
//! ## Why not ts-rs?
//!
//! [ts-rs](https://github.com/Aleph-Alpha/ts-rs) is a great library,
//! but it has a few limitations which became a problem when I was building [rspc](https://github.com/oscartbeaumont/rspc).
//! Namely it deals with types individually which means it is not possible to export a type and all of the other types it depends on.
//!
//! ## Why not [Typeshare](https://github.com/1Password/typeshare)?
//! [Typeshare](https://github.com/1Password/typeshare) is also great, but its approach is fundamentally different.
//! While Specta uses traits and runtime information, Typeshare statically analyzes your Rust
//! files.
//! This results in a loss of information and lack of compatability with types from other crates.
//!
//! ## Supported Libraries
//!
//! If you are using [Prisma Client Rust](https://prisma.brendonovich.dev) you can enable the `rspc` feature on it to allow for Specta support on types coming directly from your database. This includes support for the types created via a selection.
//!
//! ## Feature flags
#![doc = document_features::document_features!()]
#![forbid(unsafe_code)]
#![warn(clippy::all, clippy::unwrap_used, clippy::panic, missing_docs)]

/// Types related to working with [`crate::DataType`] directly.
/// You'll probably never need this.
pub mod datatype;
/// Provides the global type store and a method to export them to other languages.
#[cfg(feature = "export")]
pub mod export;
/// Support for exporting Rust functions.
#[cfg(feature = "functions")]
pub mod functions;
mod lang;
/// Contains [`Type`] and everything related to it, including implementations and helper macros
pub mod r#type;

pub use datatype::*;
pub use lang::*;
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

#[doc(hidden)]
/// This macro is exposed from rspc as a wrapper around [Type] with a correct import path.
/// This is exposed from here so rspc doesn't need a macro package for 4 lines of code.
pub use specta_macros::RSPCType;

/// Generates a From implementation for [`DataType`] of the given type.
/// This differs from [`Type`] in that you can use other [`DataType`] values
/// at runtime inside the targeted type, providing an easy way to construct types at
/// runtime from other types which are known statically via [`Type`].
///
/// Along with inner data types such as [`ObjectType`] and [`EnumType`], some builtin types
/// implement `From for DataType`:
/// - [`Vec`] will become [`DataType::Enum`]
/// - [`Option`] will become the value it contains or [`LiteralType::None`] if it is [`None`]
/// - [`String`] and [`&str`] will become [`LiteralType::String`]
///
/// This is an advanced feature and should only be of use to library authors.
///
/// ## Example
///
/// ```rust
/// use specta::{DataTypeFrom, ts};
///
/// #[derive(DataTypeFrom)]
/// pub struct MyEnum(pub Vec<String>);
///
/// let e = MyEnum(vec![
///     "A".to_string(),
///     "B".to_string(),
/// ]);
///
/// // TODO: come up with something for DataTypeFrom
/// // assert_eq!(
/// //  ts::export_datatype(&ts::ExportConfiguration::default(), &e.into()).unwrap(),
/// //     "export type MyEnum = \"A\" | \"B\""
/// // );
/// ```
///
pub use specta_macros::DataTypeFrom;

/// Prepares a function to have its types extracted using [`fn_datatype`]
///
/// ```rust
/// #[specta::specta]
/// fn my_function(arg1: i32, arg2: bool) -> &'static str {
///     "Hello World"
/// }
/// ```
pub use specta_macros::specta;

#[doc(hidden)]
pub mod internal {
    #[cfg(feature = "export")]
    pub use ctor;
    pub use specta_macros::fn_datatype;
}

#[cfg(doctest)]
doc_comment::doctest!("../README.md");
