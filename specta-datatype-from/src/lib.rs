//! Dynamically construct types with Specta
//!
//! Generates an implementation to help converting a type into into [`DataType`].
//!
//! **This is an advanced feature and should probably be limited to usage in libraries built on top of Specta.**
//!
//! This differs from [`Type`] in that you can use other [`DataType`] values
//! at runtime inside the targeted type, providing an easy way to construct types at
//! runtime from other types which are known statically via [`Type`].
//!
//! Along with inner data types such as [`StructType`] and [`EnumType`], some builtin types
//! can easily be convert to a [`DataType`]:
//! - [`Vec`] will become [`DataType::Enum`]
//! - [`Option`] will become the value it contains or [`LiteralType::None`] if it is [`None`]
//! - [`String`] and [`&str`] will become [`LiteralType::String`]
//!
//! # Example
//! ```ignore
//! use specta::{datatype::LiteralType, ts, DataType, DataTypeFrom, StructType, TupleType};
//!
//! #[derive(Clone, DataTypeFrom)]
//! pub struct MyEnum(pub Vec<DataType>);
//!
//! #[derive(Clone, DataTypeFrom)]
//! pub struct MyObject {
//!     a: Vec<DataType>,
//! }
//!
//! //
//! // Enum
//! //
//!
//! let val: DataType = MyEnum(vec![
//!     LiteralType::String("A".to_string()).into(),
//!     LiteralType::String("B".to_string()).into(),
//! ]).into();
//!
//! let anon = ts::datatype(&Default::default(), &val, &Default::default()).unwrap();
//! assert_eq!(anon, "\"A\" | \"B\"");
//!
//! let named = val.to_named("MyEnum");
//! let named_export = ts::export_named_datatype(&Default::default(), &named, &Default::default()).unwrap();
//! assert_eq!(named_export, "export type MyEnum = \"A\" | \"B\"");
//!
//! //
//! // Object
//! //
//!
//! let val: StructType = MyObject {
//!     a: vec![
//!         LiteralType::String("A".to_string()).into(),
//!         LiteralType::String("B".to_string()).into(),
//!     ],
//! }
//! .into();
//!
//! let anon = val.clone().to_anonymous();
//! let anon = ts::datatype(&Default::default(), &anon, &Default::default()).unwrap();
//! assert_eq!(anon, "{ a: \"A\" | \"B\" }");
//!
//! let named = val.to_named("MyObject");
//! let named_export = ts::export_named_datatype(&Default::default(), &named, &Default::default()).unwrap();
//! assert_eq!(named_export, "export type MyObject = { a: \"A\" | \"B\" }");
//! ```
#![doc(
    html_logo_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png"
)]

mod data_type_from;
mod utils;

#[proc_macro_derive(DataTypeFrom, attributes(specta))]
pub fn derive_data_type_from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    data_type_from::derive(input).unwrap_or_else(|err| err.into_compile_error().into())
}
