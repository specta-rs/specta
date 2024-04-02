// inspired by https://github.com/tauri-apps/tauri/blob/2901145c497299f033ba7120af5f2e7ead16c75a/core/tauri-macros/src/command/handler.rs

use quote::quote;
use syn::{parse_macro_input, FnArg, ItemFn, Visibility};

use crate::utils::{format_fn_wrapper, parse_attrs};

pub fn attribute(item: proc_macro::TokenStream) -> syn::Result<proc_macro::TokenStream> {
    let crate_ref = quote!(specta);
    let function = parse_macro_input::parse::<ItemFn>(item)?;
    let wrapper = format_fn_wrapper(&function.sig.ident);

    println!("{:?}", function.attrs);

    // TODO
    // let rename_all = function
    //     .attrs
    //     .iter()
    //     .find(|attr| attr.path.is_ident("specta") && attr.tokens.to_string() == "rename_all");

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
    let (maybe_macro_export, pub_the_trait) = match &visibility {
        Visibility::Public(_) => (quote!(#[macro_export]), Default::default()),
        _ => (
            Default::default(),
            quote! {
                // allow the macro to be resolved with the same path as the function
                #[allow(unused_imports)]
                #visibility use #wrapper;
            },
        ),
    };

    let function_name = &function.sig.ident;
    let function_name_str = function_name.to_string();
    let function_asyncness = match function.sig.asyncness {
        Some(_) => true,
        None => false,
    };

    let mut attrs = parse_attrs(&function.attrs)?;
    let common = crate::r#type::attr::CommonAttr::from_attrs(&mut attrs)?;

    let deprecated = common.deprecated_as_tokens(&crate_ref);
    let docs = common.doc;

    let args = function.sig.inputs.iter().map(|input| match input {
        FnArg::Receiver(_) => unreachable!("Commands cannot take 'self'"),
        FnArg::Typed(arg) => {
            let arg_name = &arg.pat;
            let arg_ty = &arg.ty;
            quote! {
                (stringify!(#arg_name).into(), <#arg_ty as #crate_ref::Type>::reference(type_map, &[]).inner)
            }
        }
    });

    let return_type = match &function.sig.output {
        syn::ReturnType::Default => quote!(None),
        syn::ReturnType::Type(_, ty) => {
            // TODO: Move SpectaFunctionResult into `internal`
            quote!(Some(<#ty as #crate_ref::functions::private::SpectaFunctionResult<_>>::to_datatype(type_map)))
        }
    };

    let generic_params = &function.sig.generics.params;
    let where_clause = &function.sig.generics.where_clause;

    Ok(quote! {
        #function

        #maybe_macro_export
        #[doc(hidden)]
        macro_rules! #wrapper {
            (@infer) => {
                fn export<#generic_params>(type_map: &mut #crate_ref::TypeMap) -> #crate_ref::functions::FunctionDataType #where_clause {
                    #crate_ref::functions::FunctionDataType {
                        asyncness: #function_asyncness,
                        name: #function_name_str.into(),
                        args: vec![#(#args),*],
                        result: #return_type,
                        docs: std::borrow::Cow::Borrowed(#docs),
                        deprecated: #deprecated,
                    }
                }
            }
        }

        #pub_the_trait
    }
    .into())
}
