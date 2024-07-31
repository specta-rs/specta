// inspired by https://github.com/tauri-apps/tauri/blob/2901145c497299f033ba7120af5f2e7ead16c75a/core/tauri-macros/src/command/handler.rs

use std::str::FromStr;

use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse, FnArg, ItemFn, Pat, Visibility};

use crate::utils::{format_fn_wrapper, parse_attrs};

fn unraw(s: &str) -> &str {
    if s.starts_with("r#") {
        s.split_at(2).1
    } else {
        s.as_ref()
    }
}

pub fn attribute(item: proc_macro::TokenStream) -> syn::Result<proc_macro::TokenStream> {
    let crate_ref = quote!(specta);
    let function = parse::<ItemFn>(item)?;
    let wrapper = format_fn_wrapper(&function.sig.ident);

    // While using wasm_bindgen and Specta is rare, this should make the DX nicer.
    if function.sig.unsafety.is_some()
        && function
            .sig
            .ident
            .to_string()
            .starts_with("__wasm_bindgen_generated")
    {
        return Err(syn::Error::new_spanned(
            function.sig.ident,
            "specta: You must apply the #[specta] macro before the #[wasm_bindgen] macro",
        ));
    }

    let visibility = &function.vis;
    let maybe_macro_export = match &visibility {
        Visibility::Public(_) => {
            quote!(#[macro_export])
        }
        _ => Default::default(),
    };

    let function_name = &function.sig.ident;
    let function_name_str = unraw(&function_name.to_string()).to_string();
    let function_asyncness = match function.sig.asyncness {
        Some(_) => true,
        None => false,
    };

    let mut arg_names = Vec::new();
    for input in function.sig.inputs.iter() {
        let arg = match input {
            FnArg::Receiver(_) => {
                return Err(syn::Error::new_spanned(
                    input,
                    "functions with `#[specta]` cannot take 'self'",
                ))
            }
            FnArg::Typed(arg) => match &*arg.pat {
                Pat::Ident(ident) => ident.ident.to_token_stream(),
                Pat::Macro(m) => m.mac.tokens.to_token_stream(),
                Pat::Struct(s) => s.path.to_token_stream(),
                Pat::Slice(s) => s.attrs[0].to_token_stream(),
                Pat::Tuple(s) => s.elems[0].to_token_stream(),
                _ => {
                    return Err(syn::Error::new_spanned(
                        input,
                        "functions with `#[specta]` must take named arguments",
                    ))
                }
            },
        };

        let mut s = arg.to_string();

        let s = if s.starts_with("r#") {
            s.split_off(2)
        } else {
            s
        };

        arg_names.push(TokenStream::from_str(&s).unwrap());
    }

    let arg_signatures = function.sig.inputs.iter().map(|_| quote!(_));

    let mut attrs = parse_attrs(&function.attrs)?;
    let common = crate::r#type::attr::CommonAttr::from_attrs(&mut attrs)?;

    let deprecated = common.deprecated_as_tokens(&crate_ref);
    let docs = common.doc;

    let no_return_type = match function.sig.output {
        syn::ReturnType::Default => true,
        syn::ReturnType::Type(_, _) => false,
    };

    Ok(quote! {
        #function

        #maybe_macro_export
        #[doc(hidden)]
        macro_rules! #wrapper {
            // We take in `$function` from the invocation so we have `fn_name::<concrete_generics_types>`
            (@export_fn; $function:path) => {{
                fn export(type_map: &mut #crate_ref::TypeMap) -> #crate_ref::datatype::Function {
                    #crate_ref::internal::get_fn_datatype(
                        $function as fn(#(#arg_signatures),*) -> _,
                        #function_asyncness,
                        #function_name_str.into(),
                        type_map,
                        &[#(stringify!(#arg_names).into()),* ],
                        std::borrow::Cow::Borrowed(#docs),
                        #deprecated,
                        #no_return_type,
                    )
                }

                export
            }}
        }

        // allow the macro to be resolved with the same path as the function
        #[allow(unused_imports)]
        #visibility use #wrapper;
    }
    .into())
}
