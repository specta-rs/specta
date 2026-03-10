use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::{
    parse_quote,
    visit::{self, Visit},
    ConstParam, GenericParam, Generics, LifetimeParam, Type, TypeParam, WhereClause,
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
pub fn used_type_params(generics: &Generics, container_type: Option<&Type>) -> Vec<syn::Ident> {
    let all_generic_type_idents = generics
        .params
        .iter()
        .filter_map(|gp| match gp {
            GenericParam::Type(ty) => Some(ty.ident.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();

    let Some(container_type) = container_type else {
        return all_generic_type_idents;
    };

    if all_generic_type_idents.is_empty() {
        return all_generic_type_idents;
    }

    let known_generics = all_generic_type_idents
        .iter()
        .map(ToString::to_string)
        .collect::<HashSet<_>>();

    let mut visitor = GenericTypeUseVisitor {
        known_generics: &known_generics,
        used_generics: HashSet::new(),
        conservative: false,
    };
    visitor.visit_type(container_type);

    if visitor.conservative {
        return all_generic_type_idents;
    }

    all_generic_type_idents
        .into_iter()
        .filter(|ident| visitor.used_generics.contains(&ident.to_string()))
        .collect()
}

pub fn add_type_to_where_clause(
    ty: &TokenStream,
    generics: &Generics,
    custom_bounds: Option<&[syn::WherePredicate]>,
    used_generic_types: &[syn::Ident],
) -> Option<WhereClause> {
    if let Some(where_clause) = merge_custom_bounds(generics, custom_bounds) {
        return Some(where_clause);
    }

    if used_generic_types.is_empty() {
        return generics.where_clause.clone();
    }

    match &generics.where_clause {
        None => Some(parse_quote! { where #( #used_generic_types : #ty ),* }),
        Some(w) => {
            let bounds = w.predicates.iter();
            Some(parse_quote! { where #(#bounds,)* #( #used_generic_types : #ty ),* })
        }
    }
}

fn merge_custom_bounds(
    generics: &Generics,
    custom_bounds: Option<&[syn::WherePredicate]>,
) -> Option<WhereClause> {
    if let Some(predicates) = custom_bounds {
        if predicates.is_empty() {
            return generics.where_clause.clone();
        }

        return match &generics.where_clause {
            None => Some(parse_quote! { where #(#predicates),* }),
            Some(w) => {
                let existing = w.predicates.iter();
                Some(parse_quote! { where #(#existing,)* #(#predicates),* })
            }
        };
    }

    None
}

struct GenericTypeUseVisitor<'a> {
    known_generics: &'a HashSet<String>,
    used_generics: HashSet<String>,
    conservative: bool,
}

impl Visit<'_> for GenericTypeUseVisitor<'_> {
    fn visit_type_path(&mut self, node: &syn::TypePath) {
        for segment in &node.path.segments {
            let segment_name = segment.ident.to_string();
            if self.known_generics.contains(&segment_name) {
                self.used_generics.insert(segment_name);
            }
        }

        visit::visit_type_path(self, node);
    }

    fn visit_type(&mut self, node: &syn::Type) {
        match node {
            syn::Type::Macro(_) | syn::Type::Verbatim(_) => {
                self.conservative = true;
            }
            _ => visit::visit_type(self, node),
        }
    }
}
