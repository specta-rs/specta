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

/// The `build` fn parameter list: type parameters keep their declared inline
/// bounds because the body may mention field types whose own declarations
/// require them. Lifetimes are still omitted since field types use inferred `'_`
/// lifetimes.
pub fn generics_with_ident_and_bounds_no_lifetimes(generics: &Generics) -> Option<TokenStream> {
    generics
        .params
        .iter()
        .any(|param| matches!(param, GenericParam::Type(_) | GenericParam::Const(_)))
        .then(|| {
            use GenericParam::*;

            generics.params.iter().filter_map(|param| match param {
                Type(TypeParam {
                    ident,
                    colon_token,
                    bounds,
                    ..
                }) => Some(quote!(#ident #colon_token #bounds)),
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
                    let mut attrs = parse_attrs_with_filter(&variant.attrs, skip_attrs)?;
                    let attrs = VariantAttr::from_attrs(&mut attrs)?;

                    if attrs.skip {
                        continue;
                    }

                    if let Some(ty) = &attrs.r#type {
                        visitor.visit_type(ty);
                    } else {
                        for field in &variant.fields {
                            visit_field_type(&mut visitor, field, skip_attrs)?;
                        }
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

/// The placeholder type used to stand in for a generic parameter while the
/// canonical declaration of a type is being built.
pub fn placeholder_ident(ident: &syn::Ident) -> syn::Ident {
    quote::format_ident!("__SPECTA_GENERIC_PLACEHOLDER_{}", ident)
}

/// Rewrites single-segment paths that name one of `params` to their
/// placeholder type, so the declaration is built with `Generic` placeholders
/// instead of the concrete arguments.
pub fn rewrite_generics_with_placeholders(ty: &Type, params: &[syn::Ident]) -> Type {
    struct Rewrite<'a> {
        params: &'a [syn::Ident],
    }

    impl VisitMut for Rewrite<'_> {
        fn visit_type_path_mut(&mut self, node: &mut syn::TypePath) {
            if node.qself.is_none()
                && node.path.leading_colon.is_none()
                && node.path.segments.len() == 1
            {
                let ident = node.path.segments[0].ident.clone();
                if self.params.contains(&ident) {
                    node.path.segments[0].ident = placeholder_ident(&ident);
                }
            }
            syn::visit_mut::visit_type_path_mut(self, node);
        }
    }

    let mut ty = ty.clone();
    Rewrite { params }.visit_type_mut(&mut ty);
    ty
}

/// Builds the expression that resolves `ty` to a [`DataType`].
///
/// For the canonical declaration (`conditional == true`, selected by the
/// generated `declaration` flag) generic parameters are replaced by placeholder
/// types that resolve to `Generic`, so the declaration stays generic without
/// baking the concrete arguments in.
///
/// Parameters that declare trait bounds can't be replaced by a placeholder (the
/// placeholder cannot satisfy an arbitrary user trait) — the field's own type
/// declaration may require the bound — so those are resolved through the
/// scoped generic substitution on [`specta::Types`] instead. Parameters with no
/// declared bounds cannot be required by any field type (the container would not
/// compile otherwise), so the precise placeholder path is used for them.
pub fn resolve_ty_expr(
    crate_ref: &TokenStream,
    ty: &Type,
    used_generic_types: &[syn::Ident],
    bounded_params: &HashSet<String>,
    conditional: bool,
) -> TokenStream {
    let concrete = quote!(<#ty as #crate_ref::Type>::definition(types));

    let mentioned = type_params_used_in(ty, used_generic_types);
    let unbounded = mentioned
        .iter()
        .filter(|ident| !bounded_params.contains(&ident.to_string()))
        .cloned()
        .collect::<Vec<_>>();
    let bounded = mentioned
        .iter()
        .filter(|ident| bounded_params.contains(&ident.to_string()))
        .cloned()
        .collect::<Vec<_>>();

    let declaration_ty = rewrite_generics_with_placeholders(ty, &unbounded);
    let mut declaration = quote!(types.define::<#declaration_ty>());

    if !bounded.is_empty() {
        let substitutions = bounded.iter().map(|ident| {
            let name = ident.to_string();
            quote!((
                ::std::any::type_name::<#ident>(),
                #crate_ref::datatype::Generic::new(::std::borrow::Cow::Borrowed(#name)),
            ))
        });
        declaration = quote!(types.with_generic_substitution(
            &[#(#substitutions),*],
            |types| #declaration,
        ));
    }

    if declaration_ty.to_token_stream().to_string() == ty.to_token_stream().to_string()
        && bounded.is_empty()
    {
        return concrete;
    }

    if !conditional {
        return declaration;
    }

    quote!(if declaration { #declaration } else { #concrete })
}

/// Returns the subset of `candidates` that appear in `ty`, conservatively
/// returning every candidate when the type can't be analysed (e.g. a macro).
///
/// Used to scope declaration-time generic substitution to the parameters a
/// given field actually mentions.
pub fn type_params_used_in(ty: &Type, candidates: &[syn::Ident]) -> Vec<syn::Ident> {
    if candidates.is_empty() {
        return Vec::new();
    }

    let known_generics = candidates
        .iter()
        .map(ToString::to_string)
        .collect::<HashSet<_>>();

    let mut visitor = GenericTypeUseVisitor {
        known_generics: &known_generics,
        used_generics: HashSet::new(),
        conservative: false,
        unsupported_associated_item: None,
    };
    visitor.visit_type(ty);

    if visitor.conservative {
        return candidates.to_vec();
    }

    candidates
        .iter()
        .filter(|ident| visitor.used_generics.contains(&ident.to_string()))
        .cloned()
        .collect()
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
