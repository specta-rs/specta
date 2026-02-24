use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    ConstParam, GenericParam, Generics, LifetimeParam, Type, TypeParam, WhereClause, parse_quote,
};

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
    container_type: Option<&Type>,
) -> Option<WhereClause> {
    // If custom bounds are provided, use them instead of automatic inference
    if let Some(predicates) = custom_bounds {
        if predicates.is_empty() {
            // Empty predicates = no automatic bounds, just return existing where clause
            return generics.where_clause.clone();
        }

        // Use custom predicates, merging with existing where clause
        return match &generics.where_clause {
            None => Some(parse_quote! { where #(#predicates),* }),
            Some(w) => {
                let existing = w.predicates.iter();
                Some(parse_quote! { where #(#existing,)* #(#predicates),* })
            }
        };
    }

    let generic_types = generics
        .params
        .iter()
        .filter_map(|gp| match gp {
            GenericParam::Type(ty) => Some(ty.ident.clone()),
            _ => None,
        })
        .filter(|ident| {
            container_type
                .map(|container_type| type_uses_ident(container_type, ident))
                .unwrap_or(true)
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

pub fn container_type_uses_generic(container_type: &Type, ident: &syn::Ident) -> bool {
    type_uses_ident(container_type, ident)
}

fn type_uses_ident(ty: &Type, ident: &syn::Ident) -> bool {
    match ty {
        Type::Array(t) => type_uses_ident(&t.elem, ident),
        Type::BareFn(t) => {
            t.inputs.iter().any(|arg| type_uses_ident(&arg.ty, ident))
                || return_type_uses_ident(&t.output, ident)
        }
        Type::Group(t) => type_uses_ident(&t.elem, ident),
        Type::ImplTrait(t) => t
            .bounds
            .iter()
            .any(|bound| type_param_bound_uses_ident(bound, ident)),
        Type::Infer(_) => false,
        Type::Macro(_) => true,
        Type::Never(_) => false,
        Type::Paren(t) => type_uses_ident(&t.elem, ident),
        Type::Path(t) => {
            t.qself
                .as_ref()
                .is_some_and(|qself| type_uses_ident(&qself.ty, ident))
                || path_uses_ident(&t.path, ident)
        }
        Type::Ptr(t) => type_uses_ident(&t.elem, ident),
        Type::Reference(t) => type_uses_ident(&t.elem, ident),
        Type::Slice(t) => type_uses_ident(&t.elem, ident),
        Type::TraitObject(t) => t
            .bounds
            .iter()
            .any(|bound| type_param_bound_uses_ident(bound, ident)),
        Type::Tuple(t) => t.elems.iter().any(|elem| type_uses_ident(elem, ident)),
        Type::Verbatim(_) => true,
        _ => true,
    }
}

fn type_param_bound_uses_ident(bound: &syn::TypeParamBound, ident: &syn::Ident) -> bool {
    match bound {
        syn::TypeParamBound::Trait(trait_bound) => path_uses_ident(&trait_bound.path, ident),
        syn::TypeParamBound::Lifetime(_) => false,
        _ => true,
    }
}

fn path_uses_ident(path: &syn::Path, ident: &syn::Ident) -> bool {
    path.segments.iter().any(|segment| {
        segment.ident == *ident
            || match &segment.arguments {
                syn::PathArguments::None => false,
                syn::PathArguments::AngleBracketed(args) => args.args.iter().any(|arg| match arg {
                    syn::GenericArgument::Type(ty) => type_uses_ident(ty, ident),
                    syn::GenericArgument::AssocType(binding) => type_uses_ident(&binding.ty, ident),
                    syn::GenericArgument::AssocConst(_) => true,
                    syn::GenericArgument::Constraint(constraint) => constraint
                        .bounds
                        .iter()
                        .any(|bound| type_param_bound_uses_ident(bound, ident)),
                    syn::GenericArgument::Lifetime(_) | syn::GenericArgument::Const(_) => false,
                    _ => true,
                }),
                syn::PathArguments::Parenthesized(args) => {
                    args.inputs
                        .iter()
                        .any(|input| type_uses_ident(input, ident))
                        || return_type_uses_ident(&args.output, ident)
                }
            }
    })
}

fn return_type_uses_ident(output: &syn::ReturnType, ident: &syn::Ident) -> bool {
    match output {
        syn::ReturnType::Default => false,
        syn::ReturnType::Type(_, ty) => type_uses_ident(ty, ident),
    }
}
