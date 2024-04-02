//! Easily export your Rust types to other languages
//!
//! This crate contains the macro which are reexported by the `specta` crate.
//! You shouldn't need to use this crate directly.
//! Checkout [Specta](https://docs.rs/specta).
#![doc(
    html_logo_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png"
)]

#[macro_use]
mod utils;
mod data_type_from;
#[cfg(feature = "function")]
mod internal_fn_datatype;
#[cfg(feature = "function")]
mod specta;
mod r#type;

#[proc_macro_derive(Type, attributes(specta, serde))]
pub fn derive_type(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    r#type::derive(input).unwrap_or_else(|err| err.into_compile_error().into())
}

#[proc_macro_derive(DataTypeFrom, attributes(specta))]
pub fn derive_data_type_from(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    data_type_from::derive(input).unwrap_or_else(|err| err.into_compile_error().into())
}

#[proc_macro_attribute]
#[cfg(feature = "function")]
pub fn specta(
    _: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    specta::attribute(item).unwrap_or_else(|err| err.into_compile_error().into())
}

#[proc_macro]
#[doc(hidden)]
#[cfg(feature = "function")]
pub fn internal_fn_datatype(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    use syn::parse_macro_input;

    internal_fn_datatype::proc_macro(parse_macro_input!(
        input as internal_fn_datatype::FnDatatypeInput
    ))
    .unwrap_or_else(|err| err.into_compile_error())
    .into()
}
