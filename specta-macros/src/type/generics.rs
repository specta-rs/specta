use proc_macro2::TokenStream;
use quote::quote;
use syn::{ConstParam, GenericParam, Generics, LifetimeParam, TypeParam, WhereClause, parse_quote};

pub fn generics_with_ident_and_bounds_only(generics: &Generics) -> Option<TokenStream> {
    (!generics.params.is_empty())
        .then(|| {
            use GenericParam::*;
            generics.params.iter().map(|param| match param {
                Type(TypeParam {
                    ident,
                    colon_token,
                    bounds,
                    ..
                }) => quote!(#ident #colon_token #bounds),
                Lifetime(LifetimeParam {
                    lifetime,
                    colon_token,
                    bounds,
                    ..
                }) => quote!(#lifetime #colon_token #bounds),
                Const(ConstParam {
                    const_token,
                    ident,
                    colon_token,
                    ty,
                    ..
                }) => quote!(#const_token #ident #colon_token #ty),
            })
        })
        .map(|gs| quote!(<#(#gs),*>))
}

pub fn generics_with_ident_only(generics: &Generics) -> Option<TokenStream> {
    (!generics.params.is_empty())
        .then(|| {
            use GenericParam::*;

            generics.params.iter().map(|param| match param {
                Type(TypeParam { ident, .. }) | Const(ConstParam { ident, .. }) => quote!(#ident),
                Lifetime(LifetimeParam { lifetime, .. }) => quote!(#lifetime),
            })
        })
        .map(|gs| quote!(<#(#gs),*>))
}

// Code adopted from ts-rs. Thanks to it's original author!
pub fn add_type_to_where_clause(
    ty: &TokenStream,
    generics: &Generics,
    custom_bounds: Option<&[syn::WherePredicate]>,
) -> Option<WhereClause> {
    // If custom bounds are provided, use them instead of automatic inference
    if let Some(predicates) = custom_bounds {
        if predicates.is_empty() {
            // Empty predicates = no automatic bounds, just return existing where clause
            return generics.where_clause.clone();
        }

        // Use custom predicates, merging with existing where clause
        match &generics.where_clause {
            None => Some(parse_quote! { where #(#predicates),* }),
            Some(w) => {
                let existing = w.predicates.iter();
                Some(parse_quote! { where #(#existing,)* #(#predicates),* })
            }
        }
    } else {
        // Original automatic bound logic
        let generic_types = generics
            .params
            .iter()
            .filter_map(|gp| match gp {
                GenericParam::Type(ty) => Some(ty.ident.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();
        if generic_types.is_empty() {
            return generics.where_clause.clone();
        }
        match &generics.where_clause {
            None => Some(parse_quote! { where #( #generic_types : #ty ),* }),
            Some(w) => {
                let bounds = w.predicates.iter();
                Some(parse_quote! { where #(#bounds,)* #( #generic_types : #ty ),* })
            }
        }
    }
}
