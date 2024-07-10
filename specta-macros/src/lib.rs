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

#[proc_macro_derive(Type, attributes(specta, serde))]
pub fn derive_type(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    r#type::derive(input).unwrap_or_else(|err| err.into_compile_error().into())
}

#[proc_macro_attribute]
#[cfg(feature = "DO_NOT_USE_function")]
pub fn specta(
    _: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    specta::attribute(item).unwrap_or_else(|err| err.into_compile_error().into())
}
