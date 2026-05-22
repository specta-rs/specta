use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use std::collections::HashSet;
use syn::{
    ConstParam, Data, GenericParam, Generics, LifetimeParam, Type, TypeParam, WhereClause,
    parse_quote,
    visit::{self, Visit},
    visit_mut::VisitMut,
};

use crate::utils::parse_attrs_with_filter;

use super::{FieldAttr, VariantAttr};

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

pub fn generics_with_ident_only_and_const_ty(generics: &Generics) -> Option<TokenStream> {
    generics
        .params
        .iter()
        .any(|param| matches!(param, GenericParam::Type(_) | GenericParam::Const(_)))
        .then(|| {
            use GenericParam::*;

            generics.params.iter().filter_map(|param| match param {
                Type(TypeParam { ident, .. }) => Some(quote!(#ident)),
                Lifetime(_) => None,
                Const(ConstParam {
                    const_token,
                    ident,
                    colon_token,
                    ty,
                    ..
                }) => Some(quote!(#const_token #ident #colon_token #ty)),
            })
        })
        .map(|gs| quote!(<#(#gs),*>))
}

pub fn build_type_where_clause(
    ty: &TokenStream,
    used_generic_types: &[syn::Ident],
) -> Option<WhereClause> {
    type_predicates(ty, used_generic_types).map(|preds| parse_quote! { where #(#preds),* })
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

pub fn type_with_inferred_lifetimes(ty: &Type) -> Type {
    let mut ty = ty.clone();
    InferredLifetimeVisitor.visit_type_mut(&mut ty);
    ty
}

struct InferredLifetimeVisitor;

impl VisitMut for InferredLifetimeVisitor {
    fn visit_lifetime_mut(&mut self, lifetime: &mut syn::Lifetime) {
        *lifetime = syn::Lifetime::new("'_", lifetime.apostrophe);
    }
}

fn all_type_param_idents(generics: &Generics) -> Vec<syn::Ident> {
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
pub fn used_type_params(
    generics: &Generics,
    data: &Data,
    container_type: Option<&Type>,
    skip_attrs: &[String],
) -> syn::Result<Vec<syn::Ident>> {
    let all_generic_type_idents = all_type_param_idents(generics);

    if all_generic_type_idents.is_empty() {
        return Ok(Vec::new());
    }

    let known_generics = all_generic_type_idents
        .iter()
        .map(ToString::to_string)
        .collect::<HashSet<_>>();

    let mut visitor = GenericTypeUseVisitor {
        known_generics: &known_generics,
        used_generics: HashSet::new(),
        conservative: false,
        unsupported_associated_item: None,
    };

    if let Some(container_type) = container_type {
        visitor.visit_type(container_type);
    } else {
        match data {
            Data::Struct(data) => {
                for field in &data.fields {
                    visit_field_type(&mut visitor, field, skip_attrs)?;
                }
            }
            Data::Enum(data) => {
                for variant in &data.variants {
                    if variant_is_skipped(variant, skip_attrs)? {
                        continue;
                    }

                    for field in &variant.fields {
                        visit_field_type(&mut visitor, field, skip_attrs)?;
                    }
                }
            }
            Data::Union(_) => {}
        }
    }

    if let Some(err) = visitor.unsupported_associated_item {
        return Err(err);
    }

    if visitor.conservative {
        return Ok(all_generic_type_idents);
    }

    Ok(all_generic_type_idents
        .iter()
        .filter(|ident| visitor.used_generics.contains(&ident.to_string()))
        .cloned()
        .collect())
}

fn visit_field_type(
    visitor: &mut GenericTypeUseVisitor<'_>,
    field: &syn::Field,
    skip_attrs: &[String],
) -> syn::Result<()> {
    let mut attrs = parse_attrs_with_filter(&field.attrs, skip_attrs)?;
    let attrs = FieldAttr::from_attrs(&mut attrs)?;

    if !attrs.skip {
        visitor.visit_type(attrs.r#type.as_ref().unwrap_or(&field.ty));
    }

    Ok(())
}

fn variant_is_skipped(variant: &syn::Variant, skip_attrs: &[String]) -> syn::Result<bool> {
    let mut attrs = parse_attrs_with_filter(&variant.attrs, skip_attrs)?;
    Ok(VariantAttr::from_attrs(&mut attrs)?.skip)
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

    let preds = type_predicates(ty, used_generic_types)?;

    match &generics.where_clause {
        None => Some(parse_quote! { where #(#preds),* }),
        Some(w) => {
            let bounds = w.predicates.iter();
            Some(parse_quote! { where #(#bounds,)* #(#preds),* })
        }
    }
}

fn type_predicates(
    ty: &TokenStream,
    used_generic_types: &[syn::Ident],
) -> Option<Vec<syn::WherePredicate>> {
    (!used_generic_types.is_empty()).then(|| {
        used_generic_types
            .iter()
            .map(|ident| parse_quote!(#ident : #ty))
            .collect()
    })
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
    unsupported_associated_item: Option<syn::Error>,
}

impl Visit<'_> for GenericTypeUseVisitor<'_> {
    fn visit_type_path(&mut self, node: &syn::TypePath) {
        self.reject_unsupported_associated_path(&node.path, &node.qself, node);

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

    fn visit_expr_path(&mut self, node: &syn::ExprPath) {
        self.reject_unsupported_associated_path(&node.path, &node.qself, node);

        visit::visit_expr_path(self, node);
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

impl GenericTypeUseVisitor<'_> {
    fn reject_unsupported_associated_path(
        &mut self,
        path: &syn::Path,
        qself: &Option<syn::QSelf>,
        tokens: &impl ToTokens,
    ) {
        let direct_associated_item = path
            .segments
            .first()
            .is_some_and(|first| self.known_generics.contains(&first.ident.to_string()))
            && path.segments.len() > 1;

        let qualified_associated_item = qself
            .as_ref()
            .and_then(|qself| match qself.ty.as_ref() {
                syn::Type::Path(syn::TypePath { qself: None, path }) => path.segments.first(),
                _ => None,
            })
            .is_some_and(|first| self.known_generics.contains(&first.ident.to_string()));

        if direct_associated_item || qualified_associated_item {
            self.unsupported_associated_item.get_or_insert_with(|| {
                syn::Error::new_spanned(
                    tokens,
                    "specta: associated types or constants on generic parameters are not supported",
                )
            });
        }
    }
}
