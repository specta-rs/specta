// inspired by https://github.com/tauri-apps/tauri/blob/2901145c497299f033ba7120af5f2e7ead16c75a/core/tauri-macros/src/command/handler.rs

use quote::quote;
use syn::{parse_macro_input, FnArg, ItemFn, Pat, Visibility};

use crate::utils::format_fn_wrapper;

pub fn attribute(item: proc_macro::TokenStream) -> syn::Result<proc_macro::TokenStream> {
    let function = parse_macro_input::parse::<ItemFn>(item)?;
    let wrapper = format_fn_wrapper(&function.sig.ident);

    let visibility = &function.vis;
    let maybe_macro_export = match &visibility {
        Visibility::Public(_) => quote!(#[macro_export]),
        _ => Default::default(),
    };

    let function_name = &function.sig.ident;
    let function_asyncness = match function.sig.asyncness {
        Some(_) => true,
        None => false,
    };

    let arg_names = function.sig.inputs.iter().map(|input| match input {
        FnArg::Receiver(_) => unreachable!("Commands cannot take 'self'"),
        FnArg::Typed(arg) => {
            match &*arg.pat {
                Pat::Ident(ident) => &ident.ident,
                // TODO: Adding support for these
                // Pat::Macro(m) => &m.mac.path.segments[0].ident,
                // Pat::Struct(s) => {
                //     s.
                // }
                // Pat::Slice()
                // Pat::Tuple(t) => {},
                // Pat::TupleStruct(t) => &t.path.segments[0].ident,
                _ => unreachable!("Commands must take named arguments"),
            }
        }
    });

    let arg_signatures = function.sig.inputs.iter().map(|_| quote!(_));

    let docs = function
        .attrs
        .iter()
        .filter(|attr| attr.path.is_ident("doc"))
        .filter_map(|attr| match attr.parse_meta() {
            Ok(syn::Meta::NameValue(v)) => Some(v.lit),
            _ => None,
        });

    Ok(quote! {
        #function

        #maybe_macro_export
        #[doc(hidden)]
        macro_rules! #wrapper {
            (@asyncness) => { #function_asyncness };
            (@name) => { stringify!(#function_name) };
            (@arg_names) => { &[#(stringify!(#arg_names)),* ] };
            (@signature) => { fn(#(#arg_signatures),*) -> _ };
            (@docs) => { vec![#(#docs),*] };
        }

        // allow the macro to be resolved with the same path as the function
        #[allow(unused_imports)]
        #visibility use #wrapper;
    }
    .into())
}
