//! Easily export your Rust types to other languages
//!
//! This crate contains the macro which are reexported by the `specta` crate.
//! You shouldn't need to use this crate directly.
//! Checkout [Specta](https://docs.rs/specta).
#![doc(
    html_logo_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png"
)]

#[cfg(feature = "DO_NOT_USE_function")]
mod specta;
mod r#type;
mod utils;

/// Implements [`Type`] for a given struct or enum.
///
/// # Attributes
/// Attributes can be applied to modify Specta's behavior. Specta can natively read `#[serde(...)]` attributes so your generally recommend to [just use them](https://serde.rs/attributes.html).
///
/// Specta also introduces some of it's own attributes:
///  - `#[specta(optional)]` - When paired with an `Option<T>` field, this will result in `{ a?: T | null }` instead of `{ a: T | null }`.
///  - `#[specta(type = ::std::string::String)]` - Will override the type of a item, variant or field to a given type.
///  - `#[specta(collect = false)]` - When using the `collect` feature, this will prevent the specific type from being automatically collected.
///
/// ## Example
///
/// ```ignore
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
#[proc_macro_derive(Type, attributes(specta, serde))]
pub fn derive_type(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    r#type::derive(input).unwrap_or_else(|err| err.into_compile_error().into())
}

/// Prepares a function to have its types extracted using [`functions::fn_datatype`](specta::functions::fn_datatype)
///
/// ## Example
///
/// ```ignore
/// #[specta::specta]
/// fn my_function(arg1: i32, arg2: bool) -> &'static str {
///     "Hello World"
/// }
/// ```
#[proc_macro_attribute]
#[cfg(feature = "DO_NOT_USE_function")]
pub fn specta(
    _: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    specta::attribute(item).unwrap_or_else(|err| err.into_compile_error().into())
}
