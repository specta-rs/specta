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

    let specta_fn_macro = specta_fn_macro
        .segments
        .last_mut()
        .expect("Function path is empty!");

    specta_fn_macro.ident = format_fn_wrapper(&specta_fn_macro.ident.clone());
    let arguments = std::mem::replace(&mut specta_fn_macro.arguments, PathArguments::None);

    Ok(quote! {
        let mut type_map: &mut specta::TypeMap = type_map; // This is set by [specta::fn_datatype!]

        // This macro is created by putting the `specta` macro on a function
        #specta_fn_macro!(@infer);

        // `export` is defined by the macro call above
        let result: specta::functions::FunctionDataType = export #arguments(type_map);
    })
}
