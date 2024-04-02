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
#[cfg(feature = "functions")]
mod internal_fn_datatype;
#[cfg(feature = "functions")]
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
#[cfg(feature = "functions")]
pub fn specta(
    attribute: proc_macro::TokenStream,
    item: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    println!("{:?}", attribute.to_string());

    specta::attribute(item).unwrap_or_else(|err| err.into_compile_error().into())
}

/// This should not be used directly. It's an internal function used by [specta::fn_datatype].
#[doc(hidden)]
#[proc_macro]
#[cfg(feature = "functions")]
pub fn internal_fn_datatype(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    use syn::parse_macro_input;

    internal_fn_datatype::proc_macro(parse_macro_input!(
        input as internal_fn_datatype::FnDatatypeInput
    ))
    .unwrap_or_else(|err| err.into_compile_error())
    .into()
}

// TODO: This would be Tauri Specta
// TODO: Could this apply `specta::specta` to the function too and still work
#[proc_macro_attribute]
pub fn testing(
    _: proc_macro::TokenStream,
    input: proc_macro::TokenStream,
) -> proc_macro::TokenStream {
    let mut func = syn::parse_macro_input!(input as syn::ItemFn);

    func.sig.inputs.iter_mut().for_each(|arg| {
        // if let syn::FnArg::Typed(pat) = arg {
        //     if let syn::Pat::Ident(ident) = &*pat.pat {
        //         if ident.ident == "a" {
        //             ident.ident = syn::Ident::new("b", ident.ident.span());
        //         }
        //     }
        // }
    });

    quote::quote! {
        #func
    }
    .into()
}
