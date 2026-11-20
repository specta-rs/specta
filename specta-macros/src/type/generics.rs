use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use std::borrow::Cow;
use std::collections::HashSet;
use syn::{
    ConstParam, Data, GenericParam, Generics, LifetimeParam, Path, Type, TypeParam, WhereClause,
    visit::{self, Visit},
    visit_mut::VisitMut,
};

use crate::utils::invisible;

use super::{
    Names,
    r#struct::{ContainerItems, FieldInfo},
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
/// [`generics_with_ident_only_no_lifetimes`] drops.
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
/// parameters; lifetime parameters are dropped and every bound is carried by the
/// `where` clause from [`build_where_clause`]. Declaring them in both places
/// would trip `clippy::multiple_bound_locations` in every crate that derives a
/// bounded generic type, and the copied bound tokens carry the user's spans, so
/// the warning would point at their struct.
pub fn generics_with_ident_only_no_lifetimes(generics: &Generics) -> Option<TokenStream> {
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

/// A `where` clause from the given predicates.
fn where_clause(predicates: impl IntoIterator<Item = syn::WherePredicate>) -> WhereClause {
    WhereClause {
        where_token: Default::default(),
        predicates: predicates.into_iter().collect(),
    }
}

/// The `#ident: #bound` predicate.
fn param_predicate(ident: &syn::Ident, bound: syn::TypeParamBound) -> syn::WherePredicate {
    param_predicate_with(ident, std::iter::once(bound))
}

/// The `#ident: #bounds` predicate.
fn param_predicate_with(
    ident: &syn::Ident,
    bounds: impl IntoIterator<Item = syn::TypeParamBound>,
) -> syn::WherePredicate {
    syn::WherePredicate::Type(syn::PredicateType {
        lifetimes: None,
        bounded_ty: Type::Path(syn::TypePath {
            qself: None,
            path: Path::from(ident.clone()),
        }),
        colon_token: Default::default(),
        bounds: bounds.into_iter().collect(),
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
/// predicates. Bounds are collected here rather than on the parameter list so
/// each bound has exactly one location, which keeps the generated code clean
/// under `clippy::multiple_bound_locations`.
///
/// Lifetime bounds are dropped: `build` cannot name the container's lifetime
/// parameters, and field types use inferred `'_' lifetimes so they are never
/// required.
pub fn build_where_clause(
    crate_ref: &Path,
    generics: &Generics,
    custom_bounds: Option<&[syn::WherePredicate]>,
    used_generic_types: &[&syn::Ident],
) -> Option<WhereClause> {
    let mut predicates: Vec<syn::WherePredicate> = Vec::new();

    // A parameter's inline bounds belong here, not on `build`'s parameter list:
    // a bound declared in both places trips `clippy::multiple_bound_locations`
    // in the deriving crate, where the warning points at the user's struct. The
    // `Type` bound is the exception, because [`add_type_to_where_clause`]
    // re-adds it from the automatic predicates.
    for param in &generics.params {
        let GenericParam::Type(param) = param else {
            continue;
        };

        let bounds = param
            .bounds
            .iter()
            .filter(|bound| !bound_uses_lifetime(bound) && !is_type_bound(bound, crate_ref))
            .cloned()
            .collect::<Vec<_>>();
        if !bounds.is_empty() {
            predicates.push(param_predicate_with(&param.ident, bounds));
        }
    }

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

    // `#[specta(bound = "...")]` replaces the inferred `Type` bounds, and an
    // empty string explicitly asks for none.
    if custom_bounds.is_none() {
        // Building the bound clones the `specta` path, so it is only built for a
        // container that actually has a used parameter to predicate on.
        let mut bound = None;
        for ident in used_generic_types {
            let predicate = param_predicate(
                ident,
                bound.get_or_insert_with(|| type_bound(crate_ref)).clone(),
            );
            // Parameters are unique, so the automatic predicates can only collide
            // with one of the user predicates above.
            if !seen.is_empty() && !seen.insert(quote!(#predicate).to_string()) {
                continue;
            }
            predicates.push(predicate);
        }
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

/// The type with each lifetime replaced by an inferred `'_`.
///
/// The AST is only cloned when it names a lifetime at all: walking it is much
/// cheaper than cloning every node only to find nothing to rewrite.
pub fn type_with_inferred_lifetimes(ty: &Type) -> Cow<'_, Type> {
    if !mentions_lifetime(|finder| finder.visit_type(ty)) {
        return Cow::Borrowed(ty);
    }

    let mut ty = ty.clone();
    InferredLifetimeVisitor.visit_type_mut(&mut ty);
    Cow::Owned(ty)
}

struct InferredLifetimeVisitor;

impl VisitMut for InferredLifetimeVisitor {
    fn visit_lifetime_mut(&mut self, lifetime: &mut syn::Lifetime) {
        *lifetime = syn::Lifetime::new("'_", lifetime.apostrophe);
    }
}

pub fn all_type_param_idents(generics: &Generics) -> Vec<&syn::Ident> {
    generics
        .params
        .iter()
        .filter_map(|gp| match gp {
            GenericParam::Type(ty) => Some(&ty.ident),
            _ => None,
        })
        .collect()
}

/// The type parameters of the container that its fields mention, plus whether
/// any field type is opaque (a `macro!`/`Verbatim` node whose expansion the
/// derive cannot see).
pub struct UsedTypeParams<'a> {
    /// Borrowed from the container's `Generics`, which outlives every use.
    pub idents: Vec<&'a syn::Ident>,
    /// An opaque field mentions every parameter and can never be rewritten to a
    /// placeholder.
    pub opaque: bool,
}

// Code adopted from ts-rs. Thanks to it's original author!
pub fn used_type_params<'a>(
    generics: &'a Generics,
    data: &Data,
    container_type: Option<&Type>,
    items: &ContainerItems,
) -> syn::Result<UsedTypeParams<'a>> {
    let all_generic_type_idents = all_type_param_idents(generics);

    if all_generic_type_idents.is_empty() {
        return Ok(UsedTypeParams {
            idents: Vec::new(),
            opaque: false,
        });
    }

    let mut visitor = TypeScan::new(&all_generic_type_idents);

    if let Some(container_type) = container_type {
        visitor.visit_type(container_type);
    } else {
        match (data, items) {
            (Data::Struct(data), ContainerItems::Struct(items)) => {
                for (field, item) in data.fields.iter().zip(items) {
                    visit_field_type(&mut visitor, field, item);
                }
            }
            (Data::Enum(data), ContainerItems::Enum(items)) => {
                for (variant, item) in data.variants.iter().zip(items) {
                    if item.attrs.skip {
                        continue;
                    }

                    if let Some(ty) = &item.attrs.r#type {
                        visitor.visit_type(ty);
                    } else {
                        for (field, item) in variant.fields.iter().zip(&item.fields) {
                            visit_field_type(&mut visitor, field, item);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(err) = visitor.unsupported_associated_item {
        return Err(err);
    }

    // An opaque type mentions every parameter: the derive can't see what it
    // expands to, so `mentioned` conservatively returns all of them.
    Ok(UsedTypeParams {
        idents: visitor.mentioned(),
        opaque: visitor.opaque,
    })
}

/// The placeholder type used to stand in for the generic parameter at `index`
/// while the canonical declaration of a type is being built.
///
/// `index` is the parameter's position in the container's unbounded parameter
/// list, which both the placeholder declarations and the rewrite read.
pub fn placeholder_ident(index: usize) -> syn::Ident {
    quote::format_ident!("__P{}", index)
}

/// Rewrites single-segment paths that name one of `params` to their
/// placeholder type, so the declaration is built with `Generic` placeholders
/// instead of the concrete arguments. Also reports whether any path named one
/// of `params`.
pub fn rewrite_generics_with_placeholders(ty: &Type, params: &[&syn::Ident]) -> (Type, bool) {
    struct Rewrite<'a> {
        params: &'a [&'a syn::Ident],
        rewritten: bool,
    }

    impl VisitMut for Rewrite<'_> {
        fn visit_type_path_mut(&mut self, node: &mut syn::TypePath) {
            if node.qself.is_none()
                && node.path.leading_colon.is_none()
                && node.path.segments.len() == 1
            {
                let ident = &node.path.segments[0].ident;
                if let Some(index) = self.params.iter().position(|param| *param == ident) {
                    node.path.segments[0].ident = placeholder_ident(index);
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
pub fn shares_field_isolation(used_generic_types: &[&syn::Ident]) -> bool {
    used_generic_types.is_empty()
}

/// One walk over a field type, reporting which of the enclosing generic
/// parameters it mentions and whether it contains a node the derive can't see
/// into.
///
/// [`resolve_ty_expr`] needs both answers, plus what
/// [`rewrite_generics_with_placeholders`] would find, so they share a single
/// visit instead of walking the same type three times.
/// `'c` is the lifetime of the candidate slice, `'i` the lifetime of the
/// idents in it. They are separate so a caller can build the slice locally
/// (`used_type_params` does) while the reported idents still borrow from the
/// container's `Generics`.
pub struct TypeScan<'c, 'i> {
    candidates: &'c [&'i syn::Ident],
    /// Which of `candidates` were mentioned, parallel to it.
    mentioned: Mentioned,
    /// The type contains a `macro!`/`Verbatim` node. Whatever it expands to is
    /// invisible to the derive, so every candidate must be treated as mentioned
    /// and none of them can be rewritten.
    opaque: bool,
    /// A parameter's associated type or constant was used, which Specta can't
    /// express. Only [`used_type_params`] reports it; the callers that merely
    /// ask which parameters a type mentions ignore it.
    unsupported_associated_item: Option<syn::Error>,
}

/// Which of a scan's candidates a type mentioned.
///
/// A container almost always has one to three type parameters, and a scan runs
/// once per field type (and once per type argument while building substitution
/// slots), so this is the derive's hottest small allocation. A bitmask keeps it
/// inline and allocation-free; containers with more than 64 parameters — which
/// no real type has, but the language allows — fall back to a vector so the
/// representation cannot silently truncate.
enum Mentioned {
    Mask(u64),
    Wide(Vec<bool>),
}

impl Mentioned {
    fn new(len: usize) -> Self {
        if len <= u64::BITS as usize {
            Self::Mask(0)
        } else {
            Self::Wide(vec![false; len])
        }
    }

    fn set(&mut self, index: usize) {
        match self {
            Self::Mask(mask) => *mask |= 1 << index,
            Self::Wide(bits) => bits[index] = true,
        }
    }

    fn get(&self, index: usize) -> bool {
        match self {
            Self::Mask(mask) => mask & (1 << index) != 0,
            Self::Wide(bits) => bits[index],
        }
    }

    fn any(&self) -> bool {
        match self {
            Self::Mask(mask) => *mask != 0,
            Self::Wide(bits) => bits.iter().any(|mentioned| *mentioned),
        }
    }
}

impl<'c, 'i> TypeScan<'c, 'i> {
    fn new(candidates: &'c [&'i syn::Ident]) -> Self {
        Self {
            candidates,
            mentioned: Mentioned::new(candidates.len()),
            opaque: false,
            unsupported_associated_item: None,
        }
    }

    pub fn of(ty: &Type, candidates: &'c [&'i syn::Ident]) -> Self {
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
        !self.opaque && !self.mentioned.any()
    }

    /// The mentioned parameters in declaration order, or every candidate when
    /// the type is opaque.
    pub fn mentioned(&self) -> Vec<&'i syn::Ident> {
        self.candidates
            .iter()
            .enumerate()
            .filter(|(index, _)| self.opaque || self.mentioned.get(*index))
            .map(|(_, ident)| *ident)
            .collect()
    }

    /// The index of `ident` in [`Self::candidates`], if it names one.
    fn candidate_index(&self, ident: &syn::Ident) -> Option<usize> {
        self.candidates
            .iter()
            .position(|candidate| *candidate == ident)
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

impl Visit<'_> for TypeScan<'_, '_> {
    fn visit_type_path(&mut self, node: &syn::TypePath) {
        self.reject_unsupported_associated_path(&node.path, &node.qself, node);

        if node.qself.is_none()
            && node.path.leading_colon.is_none()
            && node.path.segments.len() == 1
            && let Some(index) = self.candidate_index(&node.path.segments[0].ident)
        {
            self.mentioned.set(index);
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

/// Which of a container's two datatype builds a field expression belongs to.
///
/// A generic container builds its datatype twice: once as the canonical
/// declaration (parameters replaced by `Generic` placeholders) and once per
/// concrete instantiation (parameters resolved as written). The derive emits the
/// helpers each half needs, so only the concrete build is monomorphised per
/// instantiation.
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum BuildKind {
    /// The canonical declaration: parameters become placeholders.
    Declaration,
    /// A concrete instantiation: each parameter is resolved as written.
    Concrete,
}

/// The expression [`resolve_ty_expr`] built, plus whether it names one of the
/// container's type parameters.
pub struct ResolvedExpression {
    pub tokens: TokenStream,
    /// Whether `tokens` mentions a generic parameter. A declaration expression
    /// only does when it resolves through a generic-substitution scope, which
    /// means the function holding it must be generic too.
    pub uses_params: bool,
}

/// Builds the expression that resolves `ty` to a [`DataType`].
///
/// For the canonical declaration generic parameters are replaced by placeholder
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
    names: &Names,
    ty: &Type,
    used_generic_types: &[&syn::Ident],
    bounded_params: &[&syn::Ident],
    build: BuildKind,
) -> ResolvedExpression {
    crate::phase!("field:resolve");

    // One walk answers every question this function asks of the type.
    let scan = TypeScan::of(ty, used_generic_types);

    // Nothing to rewrite and no parameter to substitute: both builds resolve
    // the same way. A container with no used type parameters already isolated
    // its entire body with one shared empty scope (see
    // [`shares_field_isolation`]).
    let ty_trait = &names.ty;
    let crate_ref = &names.crate_ref;
    if scan.mentions_nothing() {
        let tokens = if shares_field_isolation(used_generic_types) {
            quote!(<#ty as #ty_trait>::definition(types))
        } else {
            // A field that mentions no enclosing parameter is resolved at its
            // own use site. It must not see the enclosing field's substitution
            // scope: its own concrete element type could otherwise match an
            // enclosing slot and bake that parameter into a nested declaration
            // (issue #609).
            //
            // The function item is monomorphised either way; wrapping it in a
            // closure would add a second body and its `call_once` shim.
            quote!(types.with_generic_substitution_fn(
                &[],
                <#ty as #ty_trait>::definition,
            ))
        };
        return ResolvedExpression {
            tokens: invisible(tokens),
            uses_params: false,
        };
    }

    // The concrete half inherits whatever substitution scope is active. That
    // scope belongs to an enclosing *declaration* whose slots describe this
    // field's own arguments, which is what an inlined generic type needs to
    // rewrite its expansion back to `Generic` (issue #613 regressed this: an
    // empty scope baked the first instantiation's concrete arguments into a
    // generic inline declaration).
    if build == BuildKind::Concrete {
        return ResolvedExpression {
            tokens: invisible(quote!(<#ty as #ty_trait>::definition(types))),
            uses_params: false,
        };
    }

    // Everything below only exists for the canonical declaration: the concrete
    // build resolves the type as written. A generic container builds its fields
    // twice, so this used to clone every field type into a rewritten copy and
    // collect three vectors that the concrete half never read.
    let mentioned = scan.mentioned();
    // Placeholder indices are positions in the container's unbounded parameter
    // list, which is also what `placeholder_definitions` numbers, so it is
    // rebuilt here rather than taken from the parameters this type mentions.
    let unbounded = used_generic_types
        .iter()
        .filter(|ident| !bounded_params.contains(*ident))
        .copied()
        .collect::<Vec<_>>();
    let bounded = mentioned
        .iter()
        .filter(|ident| bounded_params.contains(*ident))
        .copied()
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

    let declaration = quote!(types.define_root::<#declaration_ty>());

    if substituted.is_empty() {
        // No substitution scope: the declaration resolves with one call, and
        // the build that holds it names no parameter.
        return ResolvedExpression {
            tokens: invisible(declaration),
            uses_params: false,
        };
    }

    // One slot per type argument of the declaration type, in source order.
    // `Types::define_param` consumes them positionally as the nested type
    // resolves its own parameters, so two parameters instantiated with the
    // same concrete type still map to distinct `Generic` names. See #609.
    //
    // The placeholder rewrite only renames idents, so an opaque node in `ty`
    // is still opaque in `declaration_ty`.
    //
    // `define_root` is passed as a function item: it is monomorphised for the
    // declaration type either way, and a closure around it would add a second
    // body plus its `call_once` shim per field.
    let slots = generic_substitution_slots(names, declaration_ty, &substituted, scan.opaque);
    let declaration = quote!(types.with_generic_substitution_fn(
        &[#(#slots),*],
        #crate_ref::Types::define_root::<#declaration_ty>,
    ));

    // The slots name the parameters they substitute (`type_name::<T>()`), so
    // the function holding this expression has to be generic.
    ResolvedExpression {
        tokens: invisible(declaration),
        uses_params: true,
    }
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
    names: &Names,
    ty: &Type,
    substituted: &[&syn::Ident],
    opaque: bool,
) -> Vec<TokenStream> {
    if opaque {
        return substituted
            .iter()
            .map(|ident| generic_slot(names, ident))
            .collect();
    }

    slots_for_type(names, ty, substituted)
}

/// The slots for one type's own type-argument list.
fn slots_for_type(names: &Names, ty: &Type, substituted: &[&syn::Ident]) -> Vec<TokenStream> {
    if let Some(ident) = bare_param(ty, substituted) {
        return vec![generic_slot(names, ident)];
    }

    type_arguments(ty)
        .into_iter()
        .map(|argument| slot_for_argument(names, argument, substituted))
        .collect()
}

/// The slot for a single type argument.
fn slot_for_argument(names: &Names, argument: &Type, substituted: &[&syn::Ident]) -> TokenStream {
    let crate_ref = &names.crate_ref;

    if let Some(ident) = bare_param(argument, substituted) {
        return generic_slot(names, ident);
    }

    if TypeScan::of(argument, substituted).mentions_nothing() {
        return quote!(#crate_ref::GenericSubstitutionSlot::Skip);
    }

    let slots = slots_for_type(names, argument, substituted);
    quote!(#crate_ref::GenericSubstitutionSlot::Nested([#(#slots),*].into()))
}

fn generic_slot(names: &Names, ident: &syn::Ident) -> TokenStream {
    let crate_ref = &names.crate_ref;
    let dt = &names.dt;
    let name = ident.to_string();
    quote!(#crate_ref::GenericSubstitutionSlot::Generic {
        name: ::std::any::type_name::<#ident>(),
        generic: #dt::Generic::new(#name.into()),
    })
}

/// The ident if `ty` is a bare single-segment path naming one of `substituted`.
fn bare_param<'a>(ty: &Type, substituted: &[&'a syn::Ident]) -> Option<&'a syn::Ident> {
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
        .find(|ident| ***ident == segment.ident)
        .copied()
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

fn visit_field_type(visitor: &mut TypeScan<'_, '_>, field: &syn::Field, info: &FieldInfo) {
    if !info.attrs.skip {
        visitor.visit_type(info.attrs.r#type.as_ref().unwrap_or(&field.ty));
    }
}

pub fn add_type_to_where_clause(
    crate_ref: &Path,
    generics: &Generics,
    custom_bounds: Option<&[syn::WherePredicate]>,
    used_generic_types: &[&syn::Ident],
) -> Option<WhereClause> {
    // A custom bound replaces the automatically inferred `Type` predicates,
    // including the empty bound which explicitly requests no inference.
    if custom_bounds.is_some() {
        return merge_custom_bounds(generics, custom_bounds);
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

    /// The scans take `&[&Ident]`, so the tests pass a borrowed view of
    /// `params(..)`.
    fn refs(idents: &[syn::Ident]) -> Vec<&syn::Ident> {
        idents.iter().collect()
    }

    fn mentioned(scan: &TypeScan<'_, '_>) -> Vec<String> {
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
        let candidates = refs(&candidates);
        let scan = TypeScan::of(&parse("HashMap<String, U>"), &candidates);

        assert!(!scan.opaque);
        assert!(!scan.mentions_nothing());
        assert_eq!(mentioned(&scan), ["U"]);
    }

    #[test]
    fn scan_finds_nothing_when_no_parameter_is_mentioned() {
        let candidates = params(&["T", "U"]);

        assert!(TypeScan::of(&parse("Vec<String>"), &refs(&candidates)).mentions_nothing());
        // `TU` is not a mention of `T`, only a path that starts with it.
        assert!(TypeScan::of(&parse("(TU, u32)"), &refs(&candidates)).mentions_nothing());
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
        let candidates = refs(&candidates);

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
        assert!(!TypeScan::of(&ty, &refs(&candidates)).opaque);
        let (rewritten, rewritten_flag) =
            rewrite_generics_with_placeholders(&ty, &refs(&candidates));
        assert!(rewritten_flag);
        assert_eq!(rewritten.to_token_stream().to_string(), "Option < __P0 >");

        let ty = parse("type_from_macro!(T)");
        assert!(TypeScan::of(&ty, &refs(&candidates)).opaque);
        let (_, rewritten_flag) = rewrite_generics_with_placeholders(&ty, &refs(&candidates));
        assert!(!rewritten_flag);
    }
}
