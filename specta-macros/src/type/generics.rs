use proc_macro2::TokenStream;
use quote::quote;
use std::collections::{HashSet, hash_map::Entry};
use syn::{
    ConstParam, Data, GenericParam, Generics, LifetimeParam, Type, TypeParam, TypePath,
    WhereClause, parse_quote,
    visit::{self, Visit},
};

#[derive(Default)]
pub struct UsedTypeParams {
    pub direct: Vec<syn::Ident>,
    pub associated: Vec<TypePath>,
    pub conservative: bool,
}

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

pub fn all_type_param_idents(generics: &Generics) -> Vec<syn::Ident> {
    generics
        .params
        .iter()
        .filter_map(|gp| match gp {
            GenericParam::Type(ty) => Some(ty.ident.clone()),
            _ => None,
        })
        .collect()
}

// Code adopted from ts-rs. Thanks to it's original author!
// Additional associated-type handling inspired by Serde's derive bound collection.
pub fn used_type_params(
    generics: &Generics,
    data: &Data,
    container_type: Option<&Type>,
) -> UsedTypeParams {
    let all_generic_type_idents = all_type_param_idents(generics);

    if all_generic_type_idents.is_empty() {
        return UsedTypeParams::default();
    }

    let known_generics = all_generic_type_idents
        .iter()
        .map(ToString::to_string)
        .collect::<HashSet<_>>();

    let mut visitor = GenericTypeUseVisitor {
        known_generics: &known_generics,
        used_generics: HashSet::new(),
        associated_type_usage: Vec::new(),
        conservative: false,
    };

    if let Some(container_type) = container_type {
        visitor.visit_type(container_type);
    } else {
        match data {
            Data::Struct(data) => {
                for field in &data.fields {
                    visitor.visit_type(&field.ty);
                }
            }
            Data::Enum(data) => {
                for variant in &data.variants {
                    for field in &variant.fields {
                        visitor.visit_type(&field.ty);
                    }
                }
            }
            Data::Union(_) => {}
        }
    }

    if visitor.conservative {
        return UsedTypeParams {
            direct: all_generic_type_idents,
            associated: Vec::new(),
            conservative: true,
        };
    }

    let direct = all_generic_type_idents
        .iter()
        .filter(|ident| visitor.used_generics.contains(&ident.to_string()))
        .cloned()
        .collect();

    let mut associated = Vec::new();
    let mut seen = std::collections::HashMap::<String, ()>::new();
    for path in visitor.associated_type_usage {
        let key = quote!(#path).to_string();
        match seen.entry(key) {
            Entry::Vacant(v) => {
                v.insert(());
                associated.push(path);
            }
            Entry::Occupied(_) => {}
        }
    }

    UsedTypeParams {
        direct,
        associated,
        conservative: false,
    }
}

pub fn has_associated_type_usage(used_generic_types: &UsedTypeParams) -> bool {
    !used_generic_types.associated.is_empty()
}

pub fn used_direct_type_params<'a>(
    used_generic_types: &'a UsedTypeParams,
    all_generic_type_idents: &'a [syn::Ident],
) -> &'a [syn::Ident] {
    if used_generic_types.conservative {
        all_generic_type_idents
    } else {
        &used_generic_types.direct
    }
}

pub fn used_associated_type_paths(used_generic_types: &UsedTypeParams) -> &[TypePath] {
    if used_generic_types.conservative {
        &[]
    } else {
        &used_generic_types.associated
    }
}

pub fn add_type_to_where_clause(
    ty: &TokenStream,
    generics: &Generics,
    custom_bounds: Option<&[syn::WherePredicate]>,
    used_generic_types: &[syn::Ident],
    associated_type_usage: &[TypePath],
) -> Option<WhereClause> {
    if let Some(where_clause) = merge_custom_bounds(generics, custom_bounds) {
        return Some(where_clause);
    }

    if used_generic_types.is_empty() && associated_type_usage.is_empty() {
        return generics.where_clause.clone();
    }

    let generic_preds = used_generic_types
        .iter()
        .map(|ident| parse_quote!(#ident : #ty));
    let associated_preds = associated_type_usage
        .iter()
        .map(|path| parse_quote!(#path : #ty));
    let preds = generic_preds
        .chain(associated_preds)
        .collect::<Vec<syn::WherePredicate>>();

    match &generics.where_clause {
        None => Some(parse_quote! { where #(#preds),* }),
        Some(w) => {
            let bounds = w.predicates.iter();
            Some(parse_quote! { where #(#bounds,)* #(#preds),* })
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
    associated_type_usage: Vec<TypePath>,
    conservative: bool,
}

impl Visit<'_> for GenericTypeUseVisitor<'_> {
    fn visit_type_path(&mut self, node: &syn::TypePath) {
        if let Some(first) = node.path.segments.first()
            && self.known_generics.contains(&first.ident.to_string())
            && node.path.segments.len() > 1
        {
            self.associated_type_usage.push(node.clone());
        }

        if let Some(qself) = &node.qself
            && let syn::Type::Path(syn::TypePath { qself: None, path }) = qself.ty.as_ref()
            && let Some(first) = path.segments.first()
            && self.known_generics.contains(&first.ident.to_string())
        {
            self.associated_type_usage.push(node.clone());
        }

        if node.qself.is_none()
            && node.path.leading_colon.is_none()
            && node.path.segments.len() == 1
            && let Some(segment) = node.path.segments.first()
        {
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
