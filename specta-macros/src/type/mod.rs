use attr::*;
use r#enum::parse_enum;
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use r#struct::parse_struct;
use syn::{Data, DeriveInput, GenericParam, parse, parse_quote};

use crate::utils::{AttrExtract, parse_attrs, unraw_raw_ident};

use self::generics::{
    BuildKind, add_type_to_where_clause, build_where_clause,
    generics_with_ident_and_bounds_no_lifetimes, generics_with_ident_and_bounds_only,
    generics_with_ident_only, placeholder_ident, resolve_ty_expr, shares_field_isolation,
    type_with_inferred_lifetimes, used_type_params,
};
use self::serde::AttributeTarget;

pub(crate) mod attr;
mod r#enum;
mod field;
mod generics;
mod serde;
mod r#struct;

#[derive(Copy, Clone)]
pub(super) enum AttributeScope {
    Container,
    Variant,
    Field,
}

/// The paths the generated code uses.
///
/// Almost all of a derive's output lands in the body of `fn definition`, which
/// refers to `Type`, `Types` and the `datatype` module several thousand times
/// across a crate. That body therefore imports them under three short aliases
/// and uses those everywhere, instead of repeating `<crate>::` and the module
/// path at every reference.
///
/// The aliases live in `fn definition`'s body only. The `impl` header (and its
/// `where` clause), `build`'s predicates and the collected registration keep
/// [`Names::crate_ref`], so a compiler message a user sees never names one of
/// them.
pub(super) struct Names {
    /// The real path of the `specta` crate.
    pub(super) crate_ref: syn::Path,
    /// The aliased `Type` trait.
    ty: syn::Path,
    /// The aliased `Types` collection.
    types: syn::Path,
    /// The aliased `datatype` module, used as `#dt::Field`.
    dt: syn::Ident,
}

impl Names {
    fn new(crate_ref: syn::Path) -> Self {
        Self {
            crate_ref,
            ty: parse_quote!(__T),
            types: parse_quote!(__Types),
            dt: format_ident!("__D"),
        }
    }

    /// The aliases `fn definition`'s body imports.
    fn imports(&self) -> TokenStream {
        let crate_ref = &self.crate_ref;
        quote!(use #crate_ref::{Type as __T, Types as __Types, datatype as __D};)
    }
}

/// Lower the Serde attributes of a container, variant or field into the inert `serde:*` runtime
/// attributes.
///
/// `target` is where the attributes are written: a place expression such as `e.attributes` for a
/// container or variant that is mutated in place, or [`AttributeTarget::Chain`] for a field, whose
/// whole value is one expression.
///
/// This is deliberately unconditional. The parsed attributes are just strings and the derive
/// never interprets them, so gating this behind a Cargo feature only made the recorded metadata
/// depend on graph-wide feature unification: a consumer of `specta` + `derive` + an exporter that
/// never depended on `specta-serde` silently lost every `rename`/`rename_all`/`untagged`
/// attribute. Interpretation stays in `specta-serde`. See
/// https://github.com/specta-rs/specta/issues/590.
pub(super) fn build_runtime_attributes(
    names: &Names,
    scope: AttributeScope,
    target: AttributeTarget,
    raw_attrs: &[syn::Attribute],
    skip_attrs: &[String],
) -> syn::Result<Option<TokenStream>> {
    let raw_attrs = raw_attrs.iter().filter(|attr| {
        !attr
            .path()
            .segments
            .last()
            .is_some_and(|segment| skip_attrs.iter().any(|skip| segment.ident == skip.as_str()))
    });
    serde::lower_runtime_attributes(&names.ty, scope, target, raw_attrs)
}

fn generic_type_and_const_args(
    generics: &syn::Generics,
    type_arg: impl Fn(&syn::TypeParam) -> TokenStream,
) -> Option<TokenStream> {
    generics
        .params
        .iter()
        .any(|param| matches!(param, GenericParam::Type(_) | GenericParam::Const(_)))
        .then(|| {
            let args = generics.params.iter().filter_map(|param| match param {
                GenericParam::Lifetime(_) => None,
                GenericParam::Const(param) => Some(param.ident.to_token_stream()),
                GenericParam::Type(param) => Some(type_arg(param)),
            });

            quote!(::<#(#args),*>)
        })
}

pub fn derive(input: proc_macro::TokenStream) -> syn::Result<proc_macro::TokenStream> {
    let DeriveInput {
        ident: raw_ident,
        generics,
        data,
        attrs,
        ..
    } = &parse::<DeriveInput>(input)?;

    // We pass all the attributes at the start and when decoding them pop them off the list.
    // This means at the end we can check for any that weren't consumed and throw an error.
    let raw_attrs = attrs; // Preserve raw attrs before parse_attrs shadows the variable
    let mut attrs = parse_attrs(attrs)?;

    let container_attrs = ContainerAttr::from_attrs(&mut attrs, matches!(data, Data::Struct(_)))?;
    let crate_ref = container_attrs
        .crate_name
        .clone()
        .unwrap_or_else(|| parse_quote!(specta));
    let names = Names::new(crate_ref.clone());
    let dt = &names.dt;
    let types = &names.types;
    let ty_trait = &names.ty;
    let container_generics = generics;

    if container_attrs.r#type.is_some() && container_attrs.transparent {
        return Err(syn::Error::new(
            raw_ident.span(),
            "specta: `#[specta(type = ...)]` cannot be combined with `#[specta(transparent)]`",
        ));
    }

    let ident = container_attrs
        .remote
        .clone()
        .unwrap_or_else(|| raw_ident.to_token_stream());

    let name = unraw_raw_ident(raw_ident).to_token_stream();

    reject_unknown_specta_attrs(&attrs, Scope::Container)?;

    // Where the container-level runtime attributes are written. Without a
    // container type override the shape of the built datatype is known from the
    // input, so only the arm that can be reached is emitted.
    //
    // An enum binds its `Enum` value in the body `parse_enum` emits, so it can
    // write the attributes onto that value directly instead of converting to a
    // `DataType` and destructuring it again. Everything else builds the
    // datatype first and writes through it.
    let mut enum_attrs_apply = None;
    let container_attrs_apply =
        if container_attrs.r#type.is_some() || matches!(data, Data::Union(_)) {
            // `#[specta(type = ...)]` can replace the datatype with any shape, so
            // keep matching both container shapes.
            let struct_attrs = build_runtime_attributes(
                &names,
                AttributeScope::Container,
                AttributeTarget::Statements(quote!(s.attributes)),
                raw_attrs,
                &container_attrs.skip_attrs,
            )?;
            let enum_attrs = build_runtime_attributes(
                &names,
                AttributeScope::Container,
                AttributeTarget::Statements(quote!(en.attributes)),
                raw_attrs,
                &container_attrs.skip_attrs,
            )?;
            (struct_attrs.is_some() || enum_attrs.is_some()).then(|| {
                quote! {
                    match &mut e {
                        #dt::DataType::Struct(s) => { #struct_attrs }
                        #dt::DataType::Enum(en) => { #enum_attrs }
                        _ => unreachable!("specta derive generated non-container datatype"),
                    }
                }
            })
        } else if matches!(data, Data::Struct(_)) {
            build_runtime_attributes(
                &names,
                AttributeScope::Container,
                AttributeTarget::Statements(quote!(s.attributes)),
                raw_attrs,
                &container_attrs.skip_attrs,
            )?
            .map(|attrs| {
                quote! {
                    if let #dt::DataType::Struct(s) = &mut e {
                        #attrs
                    }
                }
            })
        } else {
            enum_attrs_apply = build_runtime_attributes(
                &names,
                AttributeScope::Container,
                AttributeTarget::Statements(quote!(e.attributes)),
                raw_attrs,
                &container_attrs.skip_attrs,
            )?;
            None
        };

    let used_type_params = used_type_params(
        generics,
        data,
        container_attrs.r#type.as_ref(),
        &container_attrs.skip_attrs,
    )?;
    let used_generic_types = &used_type_params.idents;

    let bounded = bounded_params(
        generics,
        container_attrs.bound.as_deref(),
        used_generic_types,
    );
    let has_const_param = generics
        .params
        .iter()
        .any(|param| matches!(param, GenericParam::Const(_)));

    // A container with no type parameters at all, no const parameter and no
    // container-wide inline request collapses to one `Types::define_named` call.
    // That call enters the empty substitution scope a non-generic container
    // needs by itself, so `build`'s body is not wrapped in a second copy of
    // itself (`Types::define_named`'s `build_ty`). Every other shape wraps.
    let uses_define_named = !generics
        .params
        .iter()
        .any(|param| matches!(param, GenericParam::Type(_)))
        && !has_const_param
        && !container_attrs.inline;

    // A container with used type parameters builds its datatype twice: the
    // canonical declaration (parameters replaced by placeholders) and a concrete
    // instantiation. Each half is emitted into its own nested function so only
    // the concrete one is monomorphised per instantiation.
    let needs_declaration = !used_generic_types.is_empty();

    // When the placeholders can stand in for every parameter, the declaration is
    // exactly the concrete build resolved at the placeholder types: a
    // placeholder's own definition *is* its `Generic`. That saves emitting the
    // declaration body a second time.
    //
    // This needs a parameter that no declared bound can make a field type
    // require (see `resolve_ty_expr`), no const parameter (`[T; N]` names it),
    // no opaque field type (it can't be rewritten to a placeholder) and no
    // container attribute resolving a user type (it would resolve it at the
    // placeholder instead of the concrete argument).
    let declaration_from_placeholders = needs_declaration
        && !used_type_params.opaque
        && container_attrs.bound.is_none()
        && generics.where_clause.is_none()
        && !has_const_param
        && !container_attrs_apply
            .as_ref()
            .is_some_and(|apply| names_type_param(apply, generics))
        && !enum_attrs_apply
            .as_ref()
            .is_some_and(|apply| names_type_param(apply, generics))
        && generics
            .params
            .iter()
            .all(|param| !matches!(param, GenericParam::Type(param) if !param.bounds.is_empty()));

    // The body of one of the two builds, before the container-wide wrappers.
    let build_body = |build: BuildKind| -> syn::Result<(TokenStream, bool)> {
        if let Some(container_ty) = &container_attrs.r#type {
            let container_ty = type_with_inferred_lifetimes(container_ty);
            let resolved =
                resolve_ty_expr(&names, &container_ty, used_generic_types, &bounded, build);
            let resolve = resolved.tokens;
            Ok((
                quote!(#dt::inline(types, |types| #resolve)),
                resolved.uses_params,
            ))
        } else {
            match data {
                Data::Struct(data) => parse_struct(
                    &names,
                    &container_attrs,
                    data,
                    used_generic_types,
                    &bounded,
                    build,
                ),
                Data::Enum(data) => parse_enum(
                    &names,
                    &container_attrs,
                    data,
                    used_generic_types,
                    &bounded,
                    enum_attrs_apply.as_ref(),
                    build,
                ),
                Data::Union(data) => Err(syn::Error::new_spanned(
                    data.union_token,
                    "specta: Union types are not supported by Specta yet!",
                )),
            }
        }
    };

    // The wrappers both builds share: the container runtime attributes, which
    // may replace a `#[specta(type = ...)]` shape, and — for a container with no
    // used type parameters — one empty substitution scope around the whole body.
    //
    // That single scope resolves every field concretely, so all of them are
    // isolated from the enclosing declaration scope (issue #609) without paying
    // for one scope per field. It covers the container attributes too, which
    // resolve their own `from`/`into` types. Covering the entire body is also
    // what makes the cursor bookkeeping in `definition` dead for such a
    // container: nothing `build` runs can consume a slot of the enclosing scope,
    // so the cursor it observes is the one it leaves behind.
    let wrap_body = |dt_expr: TokenStream| {
        let dt_expr = if let Some(apply) = &container_attrs_apply {
            quote! {{
                let mut e = #dt_expr;
                #apply
                e
            }}
        } else {
            dt_expr
        };

        if shares_field_isolation(used_generic_types) && !uses_define_named {
            // The body is an expression, so the closure needs no block of its own.
            quote!(types.with_generic_substitution_fn(&[], |types| #dt_expr))
        } else {
            dt_expr
        }
    };

    let concrete_dt_expr = wrap_body(build_body(BuildKind::Concrete)?.0);
    let (declaration_dt_expr, declaration_uses_params) =
        if needs_declaration && !declaration_from_placeholders {
            let (dt_expr, uses_params) = build_body(BuildKind::Declaration)?;
            // The container's runtime attributes resolve their own `from`/`into`
            // types, which may name a parameter. They are applied to both builds.
            let uses_params = uses_params
                || container_attrs_apply
                    .as_ref()
                    .is_some_and(|apply| names_type_param(apply, generics))
                || enum_attrs_apply
                    .as_ref()
                    .is_some_and(|apply| names_type_param(apply, generics));
            (Some(wrap_body(dt_expr)), uses_params)
        } else {
            (None, false)
        };

    let bounds = generics_with_ident_and_bounds_only(generics);
    let type_args = generics_with_ident_only(generics);
    let where_bound = add_type_to_where_clause(
        &crate_ref,
        generics,
        container_attrs.bound.as_deref(),
        used_generic_types,
    );
    // `build` resolves field types with the *concrete* generic arguments (so any
    // bounds a field's own type declaration requires are satisfied) and the
    // declaration then rewrites those arguments back to `Generic` placeholders.
    // It therefore needs the declared inline bounds on the parameter list plus
    // the container's `where` clause, the `#[specta(bound = "...")]` override
    // and the automatic `Type` predicates.
    let build_ty_where_bound = build_where_clause(
        &crate_ref,
        generics,
        container_attrs.bound.as_deref(),
        used_generic_types,
    );
    let build_ty_bounds = generics_with_ident_and_bounds_no_lifetimes(&crate_ref, generics);
    let placeholder_params = used_generic_types
        .iter()
        .filter(|ident| !bounded.contains(*ident))
        .cloned()
        .collect::<Vec<_>>();
    let build_placeholders = placeholder_definitions(&names, &placeholder_params);

    let build_ty_passthrough_args =
        generic_type_and_const_args(generics, |ty| ty.ident.to_token_stream());

    // `generics_for_ndt` is the declaration's generic list;
    // `instantiation_params` is the per-parameter table
    // `Types::define_instantiation_params` resolves. A parameter the
    // implementation does not resolve is stepped over in place so the parameters
    // after it stay aligned (issue #609).
    let mut generics_for_ndt: Vec<TokenStream> = Vec::new();
    let mut instantiation_params: Vec<TokenStream> = Vec::new();

    for param in &generics.params {
        let GenericParam::Type(t) = param else {
            continue;
        };

        let i = &t.ident;
        let i_str = i.to_string();
        let used = used_generic_types.iter().any(|used| used == i);

        // Most parameters carry no attributes at all, so skip building the
        // parsed attribute list for them.
        let skip_default = if t.attrs.is_empty() {
            false
        } else {
            parse_attrs(&t.attrs)?
                .extract("specta", "skip_default_generic")
                .map(|attr| attr.parse_bool_or_true())
                .transpose()?
                .unwrap_or(false)
        };

        if !used {
            instantiation_params.push(quote!(#crate_ref::InstantiationParam::Skipped));

            if skip_default {
                generics_for_ndt.push(quote!(#dt::GenericDefinition::new(
                    Cow::Borrowed(#i_str),
                    None,
                )));
            }
            continue;
        }

        let default = match (&t.default, skip_default) {
            (_, true) | (None, false) => quote!(None),
            (Some(default), false) => {
                // A default may reference another parameter (`U = T`), so
                // resolve it with the same placeholder rewriting.
                let resolved = resolve_ty_expr(
                    &names,
                    default,
                    used_generic_types,
                    &bounded,
                    BuildKind::Declaration,
                );
                let resolve = resolved.tokens;
                quote!(Some(#resolve))
            }
        };

        generics_for_ndt.push(quote!(#dt::GenericDefinition::new(
            Cow::Borrowed(#i_str),
            #default,
        )));
        instantiation_params.push(quote!(#crate_ref::InstantiationParam::Resolved {
            name: #i_str,
            definition: <#i as #ty_trait>::definition,
        }));
    }

    if cfg!(feature = "DO_NOT_USE_collect")
        && has_const_param
        && !container_attrs.inline
        && container_attrs.collect.unwrap_or(true)
    {
        return Err(syn::Error::new(
            raw_ident.span(),
            "specta: automatic collection does not support const generics; use `#[specta(collect = false)]`",
        ));
    }

    let collect = (cfg!(feature = "DO_NOT_USE_collect")
        && !container_attrs.inline
        && container_attrs.collect.unwrap_or(true))
    .then(|| {
        let export_fn_name = format_ident!("__push_specta_type_{}", raw_ident);

        let generic_params = generics
            .params
            .iter()
            .filter(|param| matches!(param, syn::GenericParam::Type(_)))
            .map(|_| quote! { () });

        quote! {
            #[doc(hidden)]
            #[allow(unsafe_code, non_snake_case)]
            #[#crate_ref::collect::internal::small_ctor::ctor]
            unsafe fn #export_fn_name() {
                #crate_ref::collect::internal::register::<#ident<#(#generic_params),*>>();
            }
        }
    });

    let has_generic_default = generics
        .params
        .iter()
        .any(|param| matches!(param, GenericParam::Type(ty) if ty.default.is_some()));
    // `Cow` is named only by the generic-definition items, which use the const
    // `Cow::Borrowed` constructor because they are also built in a `static`.
    let cow_import = (!generics_for_ndt.is_empty()).then(|| {
        quote!(
            use std::borrow::Cow;
        )
    });
    let generics = (!generics_for_ndt.is_empty()).then(|| {
        if has_generic_default {
            quote! {
                ndt.generics = ::std::vec::Vec::from([#(#generics_for_ndt),*]).into();
            }
        } else {
            quote! {
                static GENERICS: &[#dt::GenericDefinition] = &[#(#generics_for_ndt),*];
                ndt.generics = GENERICS.into();
            }
        }
    });
    // The generic instantiation path, which passes these declarations through a
    // `Types` entry point instead of assigning them in a closure.
    //
    // A parameter default can name another parameter (`U = T`), so its
    // declarations are resolved at runtime and cannot live in the `static` spec
    // the other instantiations use.
    let uses_instantiation = !instantiation_params.is_empty();
    let generics_arg = (uses_instantiation && has_generic_default).then(
        || quote!(::std::borrow::Cow::Owned(::std::vec::Vec::from([#(#generics_for_ndt),*]))),
    );
    let docs = (!container_attrs.common.doc.is_empty()).then(|| {
        let docs = &container_attrs.common.doc;
        quote! {
            ndt.docs = #docs.into();
        }
    });
    let docs_arg = &container_attrs.common.doc;
    let instantiation_generic_identities = used_generic_types
        .iter()
        .map(|generic| quote!(std::any::type_name::<#generic>()));
    let deprecated = container_attrs
        .common
        .deprecated
        .clone()
        .map(|deprecated| deprecated_as_tokens(dt, deprecated));
    // The same value in a `static`, which the promoted-constant instantiation
    // path points at: an owned `Option<Deprecated>` argument is a temporary
    // that every monomorphised `definition` would need a drop flag and cleanup
    // path for. `Into::into` is not callable in a const context, hence the
    // separate const form.
    let deprecated_const = container_attrs
        .common
        .deprecated
        .clone()
        .map(|deprecated| deprecated_as_const_tokens(dt, deprecated));
    // The same expression is either assigned in the metadata closure or passed
    // straight to `Types::define_named`, never both.
    let deprecated_stmt = deprecated
        .as_ref()
        .map(|tokens| quote!(ndt.deprecated = #tokens;));
    let deprecated_arg = deprecated.clone().unwrap_or_else(|| quote!(None));
    let (deprecated_static, deprecated_borrowed_arg) = match &deprecated_const {
        Some(tokens) => (
            quote!(static __SPECTA_DEPRECATED: #dt::Deprecated = #tokens;),
            quote!(Some(&__SPECTA_DEPRECATED)),
        ),
        None => (TokenStream::new(), quote!(None)),
    };

    // The nested functions that build the datatype. A container with used type
    // parameters gets one per half.
    //
    // `__declaration` only ever mentions placeholder types, so parameterising it
    // over them gives it exactly one monomorphisation per container — and none
    // at all for a container a crate never instantiates, which is what keeps a
    // crate that only *declares* generic types from paying for their declaration
    // bodies. When it resolves a bounded parameter through the substitution
    // scope (or names a const parameter) it carries the container's own generics
    // instead, and is instantiated per concrete argument.
    let (declaration_generics, declaration_args) = if declaration_uses_params || has_const_param {
        (build_ty_bounds.clone(), build_ty_passthrough_args.clone())
    } else {
        let placeholders = placeholder_params
            .iter()
            .enumerate()
            .map(|(index, _)| placeholder_ident(index))
            .collect::<Vec<_>>();
        (
            Some(quote!(<#(#placeholders: #ty_trait),*>)),
            Some(quote!(::<#(#placeholders),*>)),
        )
    };

    let build_fns = if needs_declaration {
        let declaration_fn = declaration_dt_expr.map(|declaration_dt_expr| {
            let declaration_where = if declaration_uses_params || has_const_param {
                build_ty_where_bound.clone()
            } else {
                None
            };

            quote! {
                fn __declaration #declaration_generics (types: &mut #types) -> #dt::DataType #declaration_where {
                    #declaration_dt_expr
                }
            }
        });

        quote! {
            #declaration_fn

            fn __concrete #build_ty_bounds (types: &mut #types) -> #dt::DataType #build_ty_where_bound {
                #concrete_dt_expr
            }
        }
    } else {
        quote! {
            fn build #build_ty_bounds (types: &mut #types) -> #dt::DataType #build_ty_where_bound {
                #concrete_dt_expr
            }
        }
    };

    // Both builds are `fn` items, so they coerce to the `fn` pointers
    // `Types::define_instantiation_params` takes without a closure being
    // monomorphised per instantiation.
    let build_fn = quote!(build #build_ty_passthrough_args);
    let (build_ty_arg, build_reference_arg) = if needs_declaration {
        let concrete = quote!(__concrete #build_ty_passthrough_args);
        if declaration_from_placeholders {
            // The declaration *is* the concrete build at the placeholders. A
            // parameter the fields never mention resolves to nothing, so it can
            // take `()` and keep the instantiation shared.
            let args = container_generics
                .params
                .iter()
                .filter_map(|param| match param {
                    GenericParam::Type(param) => Some(
                        placeholder_params
                            .iter()
                            .position(|used| *used == param.ident)
                            .map_or_else(
                                || quote!(()),
                                |index| placeholder_ident(index).to_token_stream(),
                            ),
                    ),
                    _ => None,
                });
            (concrete, quote!(__concrete::<#(#args),*>))
        } else {
            (concrete, quote!(__declaration #declaration_args))
        }
    } else {
        (build_fn.clone(), build_fn.clone())
    };

    let ndt_ty = (!container_attrs.inline).then(|| quote!(ndt.ty = Some(#build_fn(types));));

    // The sentinel is only ever read as an argument, and `init_with_sentinel`
    // takes a `&'static str`, so the `concat!` is passed straight through
    // instead of being materialised into a `static` item and then read back.
    let sentinel = quote!(concat!(module_path!(), "::", stringify!(#raw_ident)));

    // A container that resolved no generic parameter leaves the substitution
    // bookkeeping in `definition` dead: nothing is resolved into the enclosing
    // scope and the cursor cannot move. Without a const parameter and without
    // inlining, that case also fixes every `init_with_sentinel` argument, so it
    // collapses to one `define_named` call; the branches below cover the two
    // remaining shapes.
    let definition = if uses_define_named {
        let docs = &container_attrs.common.doc;
        quote! {
            types.define_named(
                #sentinel,
                #name,
                module_path!(),
                #docs,
                #deprecated_arg,
                #build_ty_arg,
            )
        }
    } else if instantiation_params.is_empty() {
        quote! {
            #dt::NamedDataType::init_with_sentinel(
                #sentinel,
                &[],
                &[],
                #has_const_param,
                false,
                types.generic_substitution_cursor(),
                types,
                |types, ndt| {
                    ndt.name = #name.into();
                    ndt.module_path = module_path!().into();
                    #generics
                    #docs
                    #deprecated_stmt

                    #ndt_ty
                },
                #build_ty_arg
            ).into()
        }
    } else {
        let inline = container_attrs.inline;
        let params = quote!(&[#(#instantiation_params),*]);
        let identities = quote!(&[#(#instantiation_generic_identities),*]);

        if has_generic_default {
            // A generic default is resolved through `types`, so the
            // `GenericDefinition` list is built at runtime and cannot be a
            // `&'static` slice. Evaluate it into a binding first: passing it as
            // an argument would borrow `types` while the call already holds it
            // mutably. Deprecation metadata is owned here for the same reason.
            let generics_value = generics_arg.clone().expect("an instantiation");
            quote! {
                {
                    let __declaration_generics = #generics_value;

                    types.define_instantiation_params_owned(
                        #sentinel,
                        #name,
                        module_path!(),
                        __declaration_generics,
                        #docs_arg,
                        #deprecated_arg,
                        #params,
                        #identities,
                        #has_const_param,
                        #inline,
                        false,
                        #build_ty_arg,
                        #build_reference_arg,
                    )
                }
            }
        } else {
            // Everything that does not depend on the concrete arguments is a
            // promoted constant or a shared `static`, and the metadata builder,
            // the inline wrapping and the parameter resolution are compiled once
            // in `specta`. The body is left with `type_name` plus one call: no
            // `Cow` or `Option<Deprecated>` temporary, so no drop flag, drop
            // glue or cleanup path either.
            let generics = if generics_for_ndt.is_empty() {
                quote!(&[] as &[#dt::GenericDefinition])
            } else {
                // A static, not a promoted constant: `GenericDefinition` needs
                // drop, so `&[..]` inline would not be promoted.
                quote!({
                    static __SPECTA_GENERICS: &[#dt::GenericDefinition] = &[#(#generics_for_ndt),*];
                    __SPECTA_GENERICS
                })
            };

            quote! {
                {
                    #deprecated_static

                    types.define_instantiation_params(
                        #sentinel,
                        #name,
                        module_path!(),
                        #generics,
                        #docs_arg,
                        #deprecated_borrowed_arg,
                        #params,
                        #identities,
                        #has_const_param,
                        #inline,
                        false,
                        #build_ty_arg,
                        #build_reference_arg,
                    )
                }
            }
        }
    };

    let definition = if container_attrs.inline && !uses_instantiation {
        quote!(#dt::inline(types, |types| #definition))
    } else {
        definition
    };

    let imports = names.imports();

    Ok(quote! {
        #[automatically_derived]
        impl #bounds #crate_ref::Type for #ident #type_args #where_bound {
            // The aliases are imported into the body, so this signature — which
            // is checked against the trait's — keeps the real paths.
            fn definition(types: &mut #crate_ref::Types) -> #crate_ref::datatype::DataType {
                #cow_import
                #imports

                #build_placeholders

                #build_fns

                #definition
            }
        }

        #collect
    }
    .into())
}

/// Declares a placeholder type for each of `params` that resolves to a
/// `Generic` node. The canonical declaration build substitutes these for the
/// corresponding parameters.
fn placeholder_definitions(names: &Names, params: &[syn::Ident]) -> TokenStream {
    let ty = &names.ty;
    let types = &names.types;
    let dt = &names.dt;

    let definitions = params.iter().enumerate().map(|(index, ident)| {
        let placeholder = placeholder_ident(index);
        let name = ident.to_string();

        quote! {
            struct #placeholder;
            impl #ty for #placeholder {
                fn definition(_: &mut #types) -> #dt::DataType {
                    #dt::Generic::new(#name.into()).into()
                }
            }
        }
    });

    quote!(#(#definitions)*)
}

/// Whether `tokens` name any type parameter of `generics`.
///
/// Field types are rewritten to placeholder types before they reach the
/// declaration build, so the only expression left that can still name a
/// parameter is the container's serde `from`/`into`/`try_from` conversion, which
/// resolves a user type verbatim. The answer only has to be conservative:
/// reporting a parameter makes the declaration build generic, which still
/// compiles.
fn names_type_param(tokens: &TokenStream, generics: &syn::Generics) -> bool {
    fn walk(tokens: TokenStream, params: &[&GenericParam]) -> bool {
        tokens.into_iter().any(|tree| match tree {
            proc_macro2::TokenTree::Ident(ident) => params.iter().any(|param| match param {
                GenericParam::Type(param) => param.ident == ident,
                GenericParam::Const(param) => param.ident == ident,
                GenericParam::Lifetime(_) => false,
            }),
            proc_macro2::TokenTree::Group(group) => walk(group.stream(), params),
            _ => false,
        })
    }

    walk(tokens.clone(), &generics.params.iter().collect::<Vec<_>>())
}

/// Whether any of the `predicates` bounds `ident`.
fn predicates_bound_param<'a>(
    predicates: impl Iterator<Item = &'a syn::WherePredicate>,
    ident: &syn::Ident,
) -> bool {
    predicates.into_iter().any(|predicate| {
        matches!(
            predicate,
            syn::WherePredicate::Type(predicate)
                if !predicate.bounds.is_empty()
                    && matches!(
                        &predicate.bounded_ty,
                        syn::Type::Path(bounded)
                            if bounded.qself.is_none()
                                && bounded.path.segments.len() == 1
                                && &bounded.path.segments[0].ident == ident
                    )
        )
    })
}

/// Generic parameters that declare at least one bound, inline, in the
/// container's `where` clause, or via `#[specta(bound = "...")]`.
fn bounded_params(
    generics: &syn::Generics,
    custom_bounds: Option<&[syn::WherePredicate]>,
    used_generic_types: &[syn::Ident],
) -> Vec<syn::Ident> {
    used_generic_types
        .iter()
        .filter(|ident| {
            let inline = matches!(
                generics
                    .params
                    .iter()
                    .find(|param| matches!(param, GenericParam::Type(param) if &param.ident == *ident)),
                Some(GenericParam::Type(param)) if !param.bounds.is_empty()
            );

            let where_clause = generics
                .where_clause
                .as_ref()
                .is_some_and(|where_clause| {
                    predicates_bound_param(where_clause.predicates.iter(), ident)
                });

            let custom = custom_bounds
                .is_some_and(|predicates| predicates_bound_param(predicates.iter(), ident));

            inline || where_clause || custom
        })
        .cloned()
        .collect()
}
