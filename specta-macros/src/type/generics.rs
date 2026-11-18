use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use std::collections::HashSet;
use syn::{
    ConstParam, Data, GenericParam, Generics, LifetimeParam, Path, Type, TypeParam, WhereClause,
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

/// `crate_ref::Type`, the bound [`build_where_clause`] adds and
/// [`generics_with_ident_and_bounds_no_lifetimes`] drops.
fn type_bound_path(crate_ref: &Path) -> Path {
    let mut path = crate_ref.clone();
    path.segments.push(syn::PathSegment::from(syn::Ident::new(
        "Type",
        Span::call_site(),
    )));
    path
}

/// Whether `bound` is the `crate_ref::Type` bound that [`build_where_clause`]
/// re-adds as a `where` predicate.
///
/// The whole path is compared, not just its last segment: a user trait that
/// happens to be named `Type` (`other::Type`) is a real bound `build` needs.
fn is_type_bound(bound: &syn::TypeParamBound, crate_ref: &Path) -> bool {
    let syn::TypeParamBound::Trait(bound) = bound else {
        return false;
    };

    // Anything that would contribute more than a plain path (`for<'a>`, `?` or
    // a parenthesized bound) is not the bound being looked for.
    if !matches!(bound.modifier, syn::TraitBoundModifier::None)
        || bound.lifetimes.is_some()
        || bound.paren_token.is_some()
        || bound.path.leading_colon.is_some() != crate_ref.leading_colon.is_some()
        || bound.path.segments.len() != crate_ref.segments.len() + 1
    {
        return false;
    }

    let mut segments = bound.path.segments.iter().rev();
    let Some(last) = segments.next() else {
        return false;
    };

    last.ident == "Type"
        && last.arguments.is_none()
        && segments
            .rev()
            .zip(&crate_ref.segments)
            .all(|(prefix, ours)| {
                prefix.ident == ours.ident && prefix.arguments.is_none() && ours.arguments.is_none()
            })
}

/// The `build` fn parameter list.
///
/// `build` is a nested fn, so it cannot name the container's lifetime
/// parameters; lifetime bounds and `crate_ref::Type` bounds are dropped. The
/// former are never needed because field types are rewritten to inferred `'_`
/// lifetimes, and the latter are re-added to the `where` clause by
/// [`build_where_clause`].
pub fn generics_with_ident_and_bounds_no_lifetimes(
    crate_ref: &Path,
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

/// A `where` clause from the given predicates.
fn where_clause(predicates: impl IntoIterator<Item = syn::WherePredicate>) -> WhereClause {
    WhereClause {
        where_token: Default::default(),
        predicates: predicates.into_iter().collect(),
    }
}

/// The `#ident: #bound` predicate.
fn param_predicate(ident: &syn::Ident, bound: syn::TypeParamBound) -> syn::WherePredicate {
    syn::WherePredicate::Type(syn::PredicateType {
        lifetimes: None,
        bounded_ty: Type::Path(syn::TypePath {
            qself: None,
            path: Path::from(ident.clone()),
        }),
        colon_token: Default::default(),
        bounds: std::iter::once(bound).collect(),
    })
}

/// The `Type` bound of `crate_ref`.
fn type_bound(crate_ref: &Path) -> syn::TypeParamBound {
    syn::TypeParamBound::Trait(syn::TraitBound {
        paren_token: None,
        modifier: syn::TraitBoundModifier::None,
        lifetimes: None,
        path: type_bound_path(crate_ref),
    })
}

/// The `where` clause for the `build` fn.
///
/// It carries the container's declared bounds (so `build`'s body type-checks
/// wherever it mentions a bounded field type) plus the automatic `Type`
/// predicates. Lifetime bounds are dropped: `build` cannot name the container's
/// lifetime parameters, and field types use inferred `'_` lifetimes so they are
/// never required.
pub fn build_where_clause(
    crate_ref: &Path,
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

    let bound = type_bound(crate_ref);
    for ident in used_generic_types {
        let predicate = param_predicate(ident, bound.clone());
        // Parameters are unique, so the automatic predicates can only collide
        // with one of the user predicates above.
        if !seen.is_empty() && !seen.insert(quote!(#predicate).to_string()) {
            continue;
        }
        predicates.push(predicate);
    }

    (!predicates.is_empty()).then(|| where_clause(predicates))
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

    let mut visitor = TypeScan::new(&all_generic_type_idents);

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

    // An opaque type mentions every parameter: the derive can't see what it
    // expands to, so `mentioned` conservatively returns all of them.
    Ok(visitor.mentioned())
}

/// The placeholder type used to stand in for a generic parameter while the
/// canonical declaration of a type is being built.
pub fn placeholder_ident(ident: &syn::Ident) -> syn::Ident {
    quote::format_ident!("__SPECTA_GENERIC_PLACEHOLDER_{}", ident)
}

/// Rewrites single-segment paths that name one of `params` to their
/// placeholder type, so the declaration is built with `Generic` placeholders
/// instead of the concrete arguments. Also reports whether any path named one
/// of `params`.
pub fn rewrite_generics_with_placeholders(ty: &Type, params: &[syn::Ident]) -> (Type, bool) {
    struct Rewrite<'a> {
        params: &'a [syn::Ident],
        rewritten: bool,
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
                    self.rewritten = true;
                }
            }
            syn::visit_mut::visit_type_path_mut(self, node);
        }
    }

    let mut ty = ty.clone();
    let mut visitor = Rewrite {
        params,
        rewritten: false,
    };
    visitor.visit_type_mut(&mut ty);
    (ty, visitor.rewritten)
}

/// Whether `build`'s body already runs inside a single empty substitution scope
/// shared by all of its fields, so a field needs no scope of its own.
///
/// A container with no used type parameters resolves every field concretely, so
/// `build` wraps its whole body in one `Types::with_generic_substitution(&[], ..)`
/// instead of paying for a scope per field. A generic container keeps the
/// per-field scope for the fields that mention no parameter, because its other
/// fields must still see the enclosing declaration's scope (issue #613).
pub fn shares_field_isolation(used_generic_types: &[syn::Ident]) -> bool {
    used_generic_types.is_empty()
}

/// One walk over a field type, reporting which of the enclosing generic
/// parameters it mentions and whether it contains a node the derive can't see
/// into.
///
/// [`resolve_ty_expr`] needs both answers, plus what
/// [`rewrite_generics_with_placeholders`] would find, so they share a single
/// visit instead of walking the same type three times.
struct TypeScan<'a> {
    candidates: &'a [syn::Ident],
    /// Which of `candidates` were mentioned, parallel to it.
    mentioned: Vec<bool>,
    /// The type contains a `macro!`/`Verbatim` node. Whatever it expands to is
    /// invisible to the derive, so every candidate must be treated as mentioned
    /// and none of them can be rewritten.
    opaque: bool,
    /// A parameter's associated type or constant was used, which Specta can't
    /// express. Only [`used_type_params`] reports it; the callers that merely
    /// ask which parameters a type mentions ignore it.
    unsupported_associated_item: Option<syn::Error>,
}

impl<'a> TypeScan<'a> {
    fn new(candidates: &'a [syn::Ident]) -> Self {
        Self {
            candidates,
            mentioned: vec![false; candidates.len()],
            opaque: false,
            unsupported_associated_item: None,
        }
    }

    fn of(ty: &Type, candidates: &'a [syn::Ident]) -> Self {
        let mut scan = Self::new(candidates);
        // Without candidates there is nothing to record, and no field of a
        // container without used parameters needs the answer.
        if !candidates.is_empty() {
            scan.visit_type(ty);
        }
        scan
    }

    /// Whether the type mentions no enclosing parameter at all.
    fn mentions_nothing(&self) -> bool {
        !self.opaque && !self.mentioned.iter().any(|mentioned| *mentioned)
    }

    /// The mentioned parameters in declaration order, or every candidate when
    /// the type is opaque.
    fn mentioned(&self) -> Vec<syn::Ident> {
        self.candidates
            .iter()
            .zip(&self.mentioned)
            .filter(|(_, mentioned)| self.opaque || **mentioned)
            .map(|(ident, _)| ident.clone())
            .collect()
    }

    /// The index of `ident` in [`Self::candidates`], if it names one.
    fn candidate_index(&self, ident: &syn::Ident) -> Option<usize> {
        self.candidates
            .iter()
            .position(|candidate| candidate == ident)
    }

    fn reject_unsupported_associated_path(
        &mut self,
        path: &syn::Path,
        qself: &Option<syn::QSelf>,
        tokens: &impl ToTokens,
    ) {
        let direct_associated_item = path
            .segments
            .first()
            .is_some_and(|first| self.candidate_index(&first.ident).is_some())
            && path.segments.len() > 1;

        let qualified_associated_item = qself
            .as_ref()
            .and_then(|qself| match qself.ty.as_ref() {
                syn::Type::Path(syn::TypePath { qself: None, path }) => path.segments.first(),
                _ => None,
            })
            .is_some_and(|first| self.candidate_index(&first.ident).is_some());

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

impl Visit<'_> for TypeScan<'_> {
    fn visit_type_path(&mut self, node: &syn::TypePath) {
        self.reject_unsupported_associated_path(&node.path, &node.qself, node);

        if node.qself.is_none()
            && node.path.leading_colon.is_none()
            && node.path.segments.len() == 1
            && let Some(index) = self.candidate_index(&node.path.segments[0].ident)
        {
            self.mentioned[index] = true;
        }

        visit::visit_type_path(self, node);
    }

    fn visit_expr_path(&mut self, node: &syn::ExprPath) {
        self.reject_unsupported_associated_path(&node.path, &node.qself, node);

        visit::visit_expr_path(self, node);
    }

    fn visit_type(&mut self, node: &syn::Type) {
        match node {
            syn::Type::Macro(_) | syn::Type::Verbatim(_) => self.opaque = true,
            _ => visit::visit_type(self, node),
        }
    }
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
    crate_ref: &Path,
    ty: &Type,
    used_generic_types: &[syn::Ident],
    bounded_params: &[syn::Ident],
    conditional: bool,
) -> TokenStream {
    // One walk answers every question this function asks of the type.
    let scan = TypeScan::of(ty, used_generic_types);

    // Nothing to rewrite and no parameter to substitute: resolve concretely.
    // A container with no used type parameters already isolated its entire
    // `build` body with one shared empty scope (see [`shares_field_isolation`]).
    if scan.mentions_nothing() {
        return if shares_field_isolation(used_generic_types) {
            quote!(<#ty as #crate_ref::Type>::definition(types))
        } else {
            // A field that mentions no enclosing parameter is resolved at its
            // own use site. It must not see the enclosing field's substitution
            // scope: its own concrete element type could otherwise match an
            // enclosing slot and bake that parameter into a nested declaration
            // (issue #609).
            quote!(types.with_generic_substitution(
                &[],
                |types| <#ty as #crate_ref::Type>::definition(types),
            ))
        };
    }

    let mentioned = scan.mentioned();
    let unbounded = mentioned
        .iter()
        .filter(|ident| !bounded_params.contains(*ident))
        .cloned()
        .collect::<Vec<_>>();
    let bounded = mentioned
        .iter()
        .filter(|ident| bounded_params.contains(*ident))
        .cloned()
        .collect::<Vec<_>>();

    // Only clone and rewrite when a parameter can actually be replaced: an
    // opaque type mentions every parameter but can't be rewritten.
    let rewritten_ty =
        (!unbounded.is_empty()).then(|| rewrite_generics_with_placeholders(ty, &unbounded));
    let (declaration_ty, rewritten) = match &rewritten_ty {
        Some((ty, rewritten)) => (ty, *rewritten),
        None => (ty, false),
    };

    // When the placeholder rewrite could not reach a parameter — an opaque
    // `macro!`-typed field, `Type::Verbatim`, ... — substitute every parameter
    // the type might mention by name instead, or the first instantiation's
    // concrete argument would be baked into the shared declaration.
    let substituted = if rewritten { bounded } else { mentioned };

    let mut declaration = quote!(types.define_root::<#declaration_ty>());

    if !substituted.is_empty() {
        // One slot per type argument of the declaration type, in source order.
        // `Types::define_param` consumes them positionally as the nested type
        // resolves its own parameters, so two parameters instantiated with the
        // same concrete type still map to distinct `Generic` names. See #609.
        //
        // The placeholder rewrite only renames idents, so an opaque node in `ty`
        // is still opaque in `declaration_ty`.
        let slots =
            generic_substitution_slots(crate_ref, declaration_ty, &substituted, scan.opaque);
        declaration = quote!(types.with_generic_substitution(
            &[#(#slots),*],
            |types| #declaration,
        ));
    }

    if !conditional {
        return declaration;
    }

    // The concrete half of a conditional resolution instead inherits whatever
    // scope is active. That scope belongs to an enclosing *declaration* whose
    // slots describe this field's own arguments, which is what an inlined
    // generic type needs to rewrite its expansion back to `Generic` (issue
    // #613 regressed this: an empty scope baked the first instantiation's
    // concrete arguments into a generic inline declaration).
    quote!(if declaration { #declaration } else { <#ty as #crate_ref::Type>::definition(types) })
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
    crate_ref: &Path,
    ty: &Type,
    substituted: &[syn::Ident],
    opaque: bool,
) -> Vec<TokenStream> {
    if opaque {
        return substituted
            .iter()
            .map(|ident| generic_slot(crate_ref, ident))
            .collect();
    }

    slots_for_type(crate_ref, ty, substituted)
}

/// The slots for one type's own type-argument list.
fn slots_for_type(crate_ref: &Path, ty: &Type, substituted: &[syn::Ident]) -> Vec<TokenStream> {
    if let Some(ident) = bare_param(ty, substituted) {
        return vec![generic_slot(crate_ref, &ident)];
    }

    type_arguments(ty)
        .into_iter()
        .map(|argument| slot_for_argument(crate_ref, argument, substituted))
        .collect()
}

/// The slot for a single type argument.
fn slot_for_argument(crate_ref: &Path, argument: &Type, substituted: &[syn::Ident]) -> TokenStream {
    if let Some(ident) = bare_param(argument, substituted) {
        return generic_slot(crate_ref, &ident);
    }

    if TypeScan::of(argument, substituted).mentions_nothing() {
        return quote!(#crate_ref::GenericSubstitutionSlot::Skip);
    }

    let slots = slots_for_type(crate_ref, argument, substituted);
    quote!(#crate_ref::GenericSubstitutionSlot::Nested(::std::vec::Vec::from([#(#slots),*])))
}

fn generic_slot(crate_ref: &Path, ident: &syn::Ident) -> TokenStream {
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

fn visit_field_type(
    visitor: &mut TypeScan<'_>,
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
    crate_ref: &Path,
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

    let bound = type_bound(crate_ref);
    let preds = used_generic_types
        .iter()
        .map(|ident| param_predicate(ident, bound.clone()));

    match &generics.where_clause {
        None => Some(where_clause(preds.collect::<Vec<_>>())),
        Some(w) => Some(where_clause(w.predicates.iter().cloned().chain(preds))),
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

        return Some(match &generics.where_clause {
            None => where_clause(predicates.iter().cloned()),
            Some(w) => where_clause(w.predicates.iter().chain(predicates).cloned()),
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(names: &[&str]) -> Vec<syn::Ident> {
        names
            .iter()
            .map(|name| syn::Ident::new(name, Span::call_site()))
            .collect()
    }

    fn mentioned(scan: &TypeScan<'_>) -> Vec<String> {
        scan.mentioned()
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
    }

    fn parse(src: &str) -> Type {
        syn::parse_str(src).expect("test type should parse")
    }

    #[test]
    fn scan_reports_the_parameters_a_type_mentions() {
        let candidates = params(&["T", "U"]);
        let scan = TypeScan::of(&parse("HashMap<String, U>"), &candidates);

        assert!(!scan.opaque);
        assert!(!scan.mentions_nothing());
        assert_eq!(mentioned(&scan), ["U"]);
    }

    #[test]
    fn scan_finds_nothing_when_no_parameter_is_mentioned() {
        let candidates = params(&["T", "U"]);

        assert!(TypeScan::of(&parse("Vec<String>"), &candidates).mentions_nothing());
        // `TU` is not a mention of `T`, only a path that starts with it.
        assert!(TypeScan::of(&parse("(TU, u32)"), &candidates).mentions_nothing());
    }

    #[test]
    fn scan_without_candidates_mentions_nothing() {
        // Fields of a container with no used parameters skip the walk entirely.
        assert!(TypeScan::of(&parse("Option<T>"), &[]).mentions_nothing());
    }

    /// A `macro!`/`Verbatim` type hides its expansion, so every candidate has to
    /// be treated as mentioned. `resolve_ty_expr` then falls back to the
    /// substitution slots instead of the placeholder rewrite, or the first
    /// instantiation's concrete argument would be baked into the declaration
    /// (see `tests/tests/bound_nested.rs::macro_typed_field_is_not_baked`).
    #[test]
    fn scan_treats_opaque_types_as_mentioning_every_parameter() {
        let candidates = params(&["T", "U"]);

        for ty in [
            parse("type_from_macro!(U)"),
            Type::Verbatim(quote!(some::verbatim::T)),
        ] {
            let scan = TypeScan::of(&ty, &candidates);
            assert!(scan.opaque);
            assert!(!scan.mentions_nothing());
            assert_eq!(mentioned(&scan), ["T", "U"]);
        }
    }

    /// The scan and the rewrite must agree: `rewritten` is only true when a
    /// placeholder was actually substituted, which an opaque type prevents even
    /// though it mentions every parameter.
    #[test]
    fn scan_opacity_matches_the_placeholder_rewrite() {
        let candidates = params(&["T"]);

        let ty = parse("Option<T>");
        assert!(!TypeScan::of(&ty, &candidates).opaque);
        let (rewritten, rewritten_flag) = rewrite_generics_with_placeholders(&ty, &candidates);
        assert!(rewritten_flag);
        assert_eq!(
            rewritten.to_token_stream().to_string(),
            "Option < __SPECTA_GENERIC_PLACEHOLDER_T >"
        );

        let ty = parse("type_from_macro!(T)");
        assert!(TypeScan::of(&ty, &candidates).opaque);
        let (_, rewritten_flag) = rewrite_generics_with_placeholders(&ty, &candidates);
        assert!(!rewritten_flag);
    }
}
