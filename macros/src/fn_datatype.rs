use crate::utils::format_fn_wrapper;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    Ident, Path, PathArguments, Token,
};

pub struct FnDatatypeInput {
    type_map: Ident,
    function: Path,
}

impl Parse for FnDatatypeInput {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let type_map: Ident = input.parse()?;
        input.parse::<Token![,]>()?;
        let function: Path = input.parse()?;

        Ok(Self { type_map, function })
    }
}

pub fn proc_macro(
    FnDatatypeInput { type_map, function }: FnDatatypeInput,
) -> syn::Result<TokenStream> {
    let mut specta_fn_macro = function.clone();

    let last = specta_fn_macro
        .segments
        .last_mut()
        .expect("Function path is empty!");

    last.ident = format_fn_wrapper(&last.ident.clone());
    last.arguments = PathArguments::None;

    Ok(quote! {
        specta::internal::get_fn_datatype(
            #function as #specta_fn_macro!(@signature),
            #specta_fn_macro!(@asyncness),
            #specta_fn_macro!(@name),
            #type_map,
            #specta_fn_macro!(@arg_names),
            #specta_fn_macro!(@docs),
            #specta_fn_macro!(@deprecated),
            #specta_fn_macro!(@no_return_type),
        )
    })
}
