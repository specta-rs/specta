use crate::utils::format_fn_wrapper;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    Path, PathArguments,
};

pub struct FnDatatypeInput {
    function: Path,
}

impl Parse for FnDatatypeInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        Ok(Self {
            function: input.parse()?,
        })
    }
}

pub fn proc_macro(FnDatatypeInput { function }: FnDatatypeInput) -> syn::Result<TokenStream> {
    let mut specta_fn_macro = function.clone();

    let last = specta_fn_macro
        .segments
        .last_mut()
        .expect("Function path is empty!");

    last.ident = format_fn_wrapper(&last.ident.clone());
    last.arguments = PathArguments::None;

    // `type_map` is defined by `specta::fn_datatype!` which invokes this.
    Ok(quote! {
        // This defines `export`
        #specta_fn_macro!(@export_fn; #function);

        // `let` is to workaround: error: macro expansion ignores token `;` and any following
        let result = export(type_map);
    })
}
