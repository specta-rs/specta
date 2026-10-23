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

/// Visitor that records whether the node it is handed mentions a lifetime.
struct LifetimeFinder {
    found: bool,
}

impl<'ast> Visit<'ast> for LifetimeFinder {
    fn visit_lifetime(&mut self, _: &'ast syn::Lifetime) {
        self.found = true;
    }
}

/// Whether `visit` finds a lifetime in the node it is handed.
fn mentions_lifetime(visit: impl FnOnce(&mut LifetimeFinder)) -> bool {
    let mut finder = LifetimeFinder { found: false };
    visit(&mut finder);
    finder.found
}

/// Whether a bound mentions any lifetime.
fn bound_uses_lifetime(bound: &syn::TypeParamBound) -> bool {
    mentions_lifetime(|finder| finder.visit_type_param_bound(bound))
}

/// Whether a `where` predicate mentions any lifetime.
pub fn predicate_uses_lifetime(predicate: &syn::WherePredicate) -> bool {
    mentions_lifetime(|finder| finder.visit_where_predicate(predicate))
}

/// Reduce a `where` predicate to the bounds [`build_where_clause`] can keep.
///
/// `build` is a nested fn and so cannot mention the container's lifetime
/// parameters, but a predicate may bundle a lifetime bound with bounds that
/// `build`'s body still needs (`T: 'a + Trait`). Only the lifetime bounds are
/// dropped; `None` comes back when nothing usable remains.
fn strip_lifetime_bounds(predicate: &syn::WherePredicate) -> Option<syn::WherePredicate> {
    let syn::WherePredicate::Type(bounded) = predicate else {
        return (!predicate_uses_lifetime(predicate)).then(|| predicate.clone());
    };

    // A lifetime inside the bounded type itself (`&'a T: Trait`) cannot be
    // rewritten, so the whole predicate goes.
    if mentions_lifetime(|finder| finder.visit_type(&bounded.bounded_ty)) {
        return None;
    }

    let bounds = bounded
        .bounds
        .iter()
        .filter(|bound| !bound_uses_lifetime(bound))
        .cloned()
        .collect::<syn::punctuated::Punctuated<_, syn::Token![+]>>();

    (!bounds.is_empty()).then(|| {
        syn::WherePredicate::Type(syn::PredicateType {
            lifetimes: bounded.lifetimes.clone(),
            bounded_ty: bounded.bounded_ty.clone(),
            colon_token: bounded.colon_token,
            bounds,
        })
    })
}

/// Whether `bound` is the `crate_ref::Type` bound that [`build_where_clause`]
/// re-adds as a `where` predicate.
///
/// The whole path is compared, not just its last segment: a user trait that
/// happens to be named `Type` (`other::Type`) is a real bound `build` needs.
fn is_type_bound(bound: &syn::TypeParamBound, crate_ref: &TokenStream) -> bool {
    let syn::TypeParamBound::Trait(bound) = bound else {
        return false;
    };

    quote!(#bound).to_string() == quote!(#crate_ref::Type).to_string()
}

/// The `build` fn parameter list.
///
/// `build` is a nested fn, so it cannot name the container's lifetime
/// parameters; lifetime bounds and `crate_ref::Type` bounds are dropped. The
/// former are never needed because field types are rewritten to inferred `'_`
/// lifetimes, and the latter are re-added to the `where` clause by
/// [`build_where_clause`].
pub fn generics_with_ident_and_bounds_no_lifetimes(
    crate_ref: &TokenStream,
    generics: &Generics,
) -> Option<TokenStream> {
    generics
        .params
        .iter()
        .any(|param| matches!(param, GenericParam::Type(_) | GenericParam::Const(_)))
        .then(|| {
            use GenericParam::*;

            generics.params.iter().filter_map(|param| match param {
                Type(TypeParam { ident, bounds, .. }) => {
                    let bounds = bounds
                        .iter()
                        .filter(|bound| {
                            !bound_uses_lifetime(bound) && !is_type_bound(bound, crate_ref)
                        })
                        .collect::<Vec<_>>();
                    if bounds.is_empty() {
                        Some(quote!(#ident))
                    } else {
                        Some(quote!(#ident: #(#bounds)+*))
                    }
                }
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

/// The `where` clause for the `build` fn.
///
/// It carries the container's declared bounds (so `build`'s body type-checks
/// wherever it mentions a bounded field type) plus the automatic `Type`
/// predicates. Lifetime bounds are dropped: `build` cannot name the container's
/// lifetime parameters, and field types use inferred `'_` lifetimes so they are
/// never required.
pub fn build_where_clause(
    crate_ref: &TokenStream,
    generics: &Generics,
    custom_bounds: Option<&[syn::WherePredicate]>,
    used_generic_types: &[syn::Ident],
) -> Option<WhereClause> {
    let mut predicates: Vec<syn::WherePredicate> = Vec::new();

    if let Some(where_clause) = &generics.where_clause {
        predicates.extend(
            where_clause
                .predicates
                .iter()
                .filter_map(strip_lifetime_bounds),
        );
    }
    if let Some(custom_bounds) = custom_bounds {
        predicates.extend(custom_bounds.iter().filter_map(strip_lifetime_bounds));
    }

    let mut seen = HashSet::new();
    predicates.retain(|predicate| seen.insert(quote!(#predicate).to_string()));

    for ident in used_generic_types {
        let predicate: syn::WherePredicate = parse_quote!(#ident: #crate_ref::Type);
        if seen.insert(quote!(#predicate).to_string()) {
            predicates.push(predicate);
        }
    }

    (!predicates.is_empty()).then(|| parse_quote!(where #(#predicates),*))
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
    // The concrete (use-site) resolution must not see the enclosing field's
    // substitution scope: a field that mentions no enclosing parameter would
    // otherwise let its own concrete element type match an enclosing slot and
    // bake that parameter into a nested declaration (issue #609).
    let concrete = quote!(types.with_generic_substitution(
        &[],
        |types| <#ty as #crate_ref::Type>::definition(types),
    ));

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
    let rewritten =
        declaration_ty.to_token_stream().to_string() != ty.to_token_stream().to_string();

    // When the placeholder rewrite could not reach a parameter — an opaque
    // `macro!`-typed field, `Type::Verbatim`, ... — substitute every parameter
    // the type might mention by name instead, or the first instantiation's
    // concrete argument would be baked into the shared declaration.
    let substituted = if rewritten {
        bounded.clone()
    } else {
        mentioned.clone()
    };

    // Nothing to rewrite and no parameter to substitute: resolve concretely.
    if !rewritten && mentioned.is_empty() {
        return concrete;
    }

    let mut declaration = quote!(types.define_root::<#declaration_ty>());

    if !substituted.is_empty() {
        // One slot per type argument of the declaration type, in source order.
        // `Types::define_param` consumes them positionally as the nested type
        // resolves its own parameters, so two parameters instantiated with the
        // same concrete type still map to distinct `Generic` names. See #609.
        let slots = generic_substitution_slots(crate_ref, &declaration_ty, &substituted);
        declaration = quote!(types.with_generic_substitution(
            &[#(#slots),*],
            |types| #declaration,
        ));
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

/// Builds the positional substitution slots for the type arguments of `ty`.
///
/// [`Types::define_param`] consumes these as the nested type resolves its own
/// parameters, so two parameters instantiated with the same concrete type still
/// map to distinct `Generic` names; see issue #609.
///
/// When `ty` cannot be analysed (a `macro!`/`Verbatim` type, whose expansion
/// the derive cannot see) the slots collapse to a flat, name-keyed list of
/// `substituted`. That is best-effort — an opaque field mentioning two
/// parameters of the same concrete type is still ambiguous — but it preserves
/// the previous behaviour for opaque fields rather than baking the concrete
/// argument in.
pub fn generic_substitution_slots(
    crate_ref: &TokenStream,
    ty: &Type,
    substituted: &[syn::Ident],
) -> Vec<TokenStream> {
    if mentions_opaque(ty) {
        return substituted
            .iter()
            .map(|ident| generic_slot(crate_ref, ident))
            .collect();
    }

    slots_for_type(crate_ref, ty, substituted)
}

/// The slots for one type's own type-argument list.
fn slots_for_type(
    crate_ref: &TokenStream,
    ty: &Type,
    substituted: &[syn::Ident],
) -> Vec<TokenStream> {
    if let Some(ident) = bare_param(ty, substituted) {
        return vec![generic_slot(crate_ref, &ident)];
    }

    type_arguments(ty)
        .into_iter()
        .map(|argument| slot_for_argument(crate_ref, argument, substituted))
        .collect()
}

/// The slot for a single type argument.
fn slot_for_argument(
    crate_ref: &TokenStream,
    argument: &Type,
    substituted: &[syn::Ident],
) -> TokenStream {
    if let Some(ident) = bare_param(argument, substituted) {
        return generic_slot(crate_ref, &ident);
    }

    if type_params_used_in(argument, substituted).is_empty() {
        return quote!(#crate_ref::GenericSubstitutionSlot::Skip);
    }

    let slots = slots_for_type(crate_ref, argument, substituted);
    quote!(#crate_ref::GenericSubstitutionSlot::Nested(vec![#(#slots),*]))
}

fn generic_slot(crate_ref: &TokenStream, ident: &syn::Ident) -> TokenStream {
    let name = ident.to_string();
    quote!(#crate_ref::GenericSubstitutionSlot::Generic {
        name: ::std::any::type_name::<#ident>(),
        generic: #crate_ref::datatype::Generic::new(::std::borrow::Cow::Borrowed(#name)),
    })
}

/// The ident if `ty` is a bare single-segment path naming one of `substituted`.
fn bare_param(ty: &Type, substituted: &[syn::Ident]) -> Option<syn::Ident> {
    let Type::Path(path) = ty else {
        return None;
    };

    if path.qself.is_some() || path.path.leading_colon.is_some() || path.path.segments.len() != 1 {
        return None;
    }

    let segment = path.path.segments.first()?;
    if !segment.arguments.is_empty() {
        return None;
    }

    substituted
        .iter()
        .find(|ident| **ident == segment.ident)
        .cloned()
}

/// The type arguments of a type constructor, in source order.
fn type_arguments(ty: &Type) -> Vec<&Type> {
    match ty {
        Type::Path(path) if path.qself.is_none() => path
            .path
            .segments
            .last()
            .map(|segment| match &segment.arguments {
                syn::PathArguments::AngleBracketed(arguments) => arguments
                    .args
                    .iter()
                    .filter_map(|argument| match argument {
                        syn::GenericArgument::Type(ty) => Some(ty),
                        _ => None,
                    })
                    .collect(),
                _ => Vec::new(),
            })
            .unwrap_or_default(),
        Type::Reference(reference) => vec![&reference.elem],
        Type::Tuple(tuple) => tuple.elems.iter().collect(),
        Type::Slice(slice) => vec![&slice.elem],
        Type::Array(array) => vec![&array.elem],
        Type::Ptr(pointer) => vec![&pointer.elem],
        Type::Paren(paren) => vec![&paren.elem],
        Type::Group(group) => vec![&group.elem],
        _ => Vec::new(),
    }
}

/// Whether `ty` contains a `macro!`/`Verbatim` node the derive cannot see into.
fn mentions_opaque(ty: &Type) -> bool {
    struct OpaqueFinder(bool);

    impl Visit<'_> for OpaqueFinder {
        fn visit_type(&mut self, node: &syn::Type) {
            match node {
                syn::Type::Macro(_) | syn::Type::Verbatim(_) => self.0 = true,
                _ => visit::visit_type(self, node),
            }
        }
    }

    let mut finder = OpaqueFinder(false);
    finder.visit_type(ty);
    finder.0
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
