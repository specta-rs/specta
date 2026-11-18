use attr::*;
use r#enum::parse_enum;
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use r#struct::parse_struct;
use syn::{Data, DeriveInput, GenericParam, parse, parse_quote};

use crate::utils::{AttrExtract, parse_attrs, unraw_raw_ident};

use self::generics::{
    add_type_to_where_clause, build_where_clause, generics_with_ident_and_bounds_no_lifetimes,
    generics_with_ident_and_bounds_only, generics_with_ident_only, placeholder_ident,
    resolve_ty_expr, shares_field_isolation, type_with_inferred_lifetimes, used_type_params,
};

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

/// Lower the Serde attributes of a container, variant or field into the inert `serde:*` runtime
/// attributes.
///
/// This is deliberately unconditional. The parsed attributes are just strings and the derive
/// never interprets them, so gating this behind a Cargo feature only made the recorded metadata
/// depend on graph-wide feature unification: a consumer of `specta` + `derive` + an exporter that
/// never depended on `specta-serde` silently lost every `rename`/`rename_all`/`untagged`
/// attribute. Interpretation stays in `specta-serde`. See
/// https://github.com/specta-rs/specta/issues/590.
pub(super) fn build_runtime_attributes(
    crate_ref: &syn::Path,
    scope: AttributeScope,
    attrs: TokenStream,
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
    let serde_insert = serde::lower_runtime_attributes(crate_ref, scope, raw_attrs)?;

    if serde_insert.is_none() {
        return Ok(None);
    }

    Ok(Some(quote!({
        let attrs = &mut #attrs;
        #serde_insert
    })))
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
    let container_attrs_apply =
        if container_attrs.r#type.is_some() || matches!(data, Data::Union(_)) {
            // `#[specta(type = ...)]` can replace the datatype with any shape, so
            // keep matching both container shapes.
            let struct_attrs = build_runtime_attributes(
                &crate_ref,
                AttributeScope::Container,
                quote!(s.attributes),
                raw_attrs,
                &container_attrs.skip_attrs,
            )?;
            let enum_attrs = build_runtime_attributes(
                &crate_ref,
                AttributeScope::Container,
                quote!(en.attributes),
                raw_attrs,
                &container_attrs.skip_attrs,
            )?;
            (struct_attrs.is_some() || enum_attrs.is_some()).then(|| {
                quote! {
                    match &mut e {
                        datatype::DataType::Struct(s) => { #struct_attrs }
                        datatype::DataType::Enum(en) => { #enum_attrs }
                        _ => unreachable!("specta derive generated non-container datatype"),
                    }
                }
            })
        } else {
            let (shape, target) = if matches!(data, Data::Struct(_)) {
                (quote!(datatype::DataType::Struct(s)), quote!(s.attributes))
            } else {
                (quote!(datatype::DataType::Enum(en)), quote!(en.attributes))
            };
            build_runtime_attributes(
                &crate_ref,
                AttributeScope::Container,
                target,
                raw_attrs,
                &container_attrs.skip_attrs,
            )?
            .map(|attrs| {
                quote! {
                    if let #shape = &mut e {
                        #attrs
                    }
                }
            })
        };

    let used_generic_types = used_type_params(
        generics,
        data,
        container_attrs.r#type.as_ref(),
        &container_attrs.skip_attrs,
    )?;

    let bounded = bounded_params(
        generics,
        container_attrs.bound.as_deref(),
        &used_generic_types,
    );

    let dt_expr = if let Some(container_ty) = &container_attrs.r#type {
        let container_ty = type_with_inferred_lifetimes(container_ty);
        let resolve = resolve_ty_expr(
            &crate_ref,
            &container_ty,
            &used_generic_types,
            &bounded,
            true,
        );
        quote!(datatype::inline(types, |types| #resolve))
    } else {
        match data {
            Data::Struct(data) => parse_struct(
                &crate_ref,
                &container_attrs,
                data,
                &used_generic_types,
                &bounded,
            ),
            Data::Enum(data) => parse_enum(
                &crate_ref,
                &container_attrs,
                data,
                &used_generic_types,
                &bounded,
            ),
            Data::Union(data) => Err(syn::Error::new_spanned(
                data.union_token,
                "specta: Union types are not supported by Specta yet!",
            )),
        }?
    };

    // A block expression, so `build`'s body (and the shared scope below) can
    // use it wherever an expression is expected.
    let dt_expr = if let Some(apply) = container_attrs_apply {
        quote! {{
            let mut e = #dt_expr;
            #apply
            e
        }}
    } else {
        dt_expr
    };

    // A container with no used type parameters resolves every field concretely,
    // so a single empty scope around the whole body isolates all of them from
    // the enclosing declaration scope (issue #609) instead of paying for one
    // scope per field. It covers the container attributes too, which resolve
    // their own `from`/`into` types.
    //
    // Covering the entire body is also what makes the cursor bookkeeping in
    // `definition` dead for such a container: nothing `build` runs can consume a
    // slot of the enclosing scope, so the cursor it observes is the one it
    // leaves behind.
    let dt_expr = if shares_field_isolation(&used_generic_types) {
        // The body is an expression, so the closure needs no block of its own.
        quote!(types.with_generic_substitution(&[], |types| #dt_expr))
    } else {
        dt_expr
    };

    let bounds = generics_with_ident_and_bounds_only(generics);
    let type_args = generics_with_ident_only(generics);
    let has_const_param = generics
        .params
        .iter()
        .any(|param| matches!(param, GenericParam::Const(_)));
    let where_bound = add_type_to_where_clause(
        &crate_ref,
        generics,
        container_attrs.bound.as_deref(),
        &used_generic_types,
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
        &used_generic_types,
    );
    let build_ty_bounds = generics_with_ident_and_bounds_no_lifetimes(&crate_ref, generics);
    let placeholder_params = used_generic_types
        .iter()
        .filter(|ident| !bounded.contains(*ident))
        .cloned()
        .collect::<Vec<_>>();
    let build_placeholders = placeholder_definitions(&crate_ref, &placeholder_params);

    let build_ty_passthrough_args =
        generic_type_and_const_args(generics, |ty| ty.ident.to_token_stream());

    // `generics_for_ndt` is the declaration's generic list; `instantiation_steps`
    // resolves the type's own arguments in parameter order. Unused parameters
    // still step over a substitution slot so the parameters after them stay
    // aligned (issue #609).
    let mut generics_for_ndt: Vec<TokenStream> = Vec::new();
    let mut instantiation_steps: Vec<TokenStream> = Vec::new();

    for param in &generics.params {
        let GenericParam::Type(t) = param else {
            continue;
        };

        let i = &t.ident;
        let i_str = i.to_string();
        let used = used_generic_types.iter().any(|used| used == i);

        let skip_default = parse_attrs(&t.attrs)?
            .extract("specta", "skip_default_generic")
            .map(|attr| attr.parse_bool_or_true())
            .transpose()?
            .unwrap_or(false);

        if !used {
            instantiation_steps.push(quote!(types.skip_param_slot();));

            if skip_default {
                generics_for_ndt.push(quote!(#crate_ref::datatype::GenericDefinition::new(
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
                let resolve =
                    resolve_ty_expr(&crate_ref, default, &used_generic_types, &bounded, false);
                quote!(Some(#resolve))
            }
        };

        generics_for_ndt.push(quote!(#crate_ref::datatype::GenericDefinition::new(
            Cow::Borrowed(#i_str),
            #default,
        )));
        instantiation_steps.push(quote!(
            __specta_instantiation_generics.push((
                #crate_ref::datatype::Generic::new(Cow::Borrowed(#i_str)),
                types.define_param::<#i>(),
            ));
        ));
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
    let generics = (!generics_for_ndt.is_empty()).then(|| {
        if has_generic_default {
            quote! {
                ndt.generics = Cow::Owned(::std::vec::Vec::from([#(#generics_for_ndt),*]));
            }
        } else {
            quote! {
                static GENERICS: &[datatype::GenericDefinition] = &[#(#generics_for_ndt),*];
                ndt.generics = Cow::Borrowed(GENERICS);
            }
        }
    });
    let docs = (!container_attrs.common.doc.is_empty()).then(|| {
        let docs = &container_attrs.common.doc;
        quote! {
            ndt.docs = Cow::Borrowed(#docs);
        }
    });
    let instantiation_generic_identities = used_generic_types
        .iter()
        .map(|generic| quote!(std::any::type_name::<#generic>()));
    let deprecated = container_attrs.common.deprecated.map(|deprecated| {
        let tokens = deprecated_as_tokens(deprecated);
        quote!(ndt.deprecated = #tokens;)
    });

    // `build` only needs the `declaration` flag when a field can actually be
    // resolved through a generic parameter's placeholder.
    let needs_declaration = !used_generic_types.is_empty();
    let build_fn = quote!(build #build_ty_passthrough_args);
    let declaration_call = if needs_declaration {
        quote!(#build_fn(types, true))
    } else {
        quote!(#build_fn(types))
    };
    let build_ty_arg = if needs_declaration {
        quote!(|types| #build_fn(types, false))
    } else {
        build_fn
    };
    let declaration_param = needs_declaration.then(|| quote!(, declaration: bool));

    let ndt_ty = (!container_attrs.inline).then(|| quote!(ndt.ty = Some(#declaration_call);));

    // Container `build` may have escaped no generic parameters at all, in which
    // case the whole substitution bookkeeping in `definition` is dead: nothing
    // is resolved into the enclosing scope and the cursor cannot move.
    let definition = if instantiation_steps.is_empty() {
        quote! {
            datatype::NamedDataType::init_with_sentinel(
                SENTINEL,
                &[],
                &[],
                #has_const_param,
                false,
                types.generic_substitution_cursor(),
                types,
                |types, ndt| {
                    ndt.name = Cow::Borrowed(#name);
                    ndt.module_path = Cow::Borrowed(module_path!());
                    #generics
                    #docs
                    #deprecated

                    #ndt_ty
                },
                #build_ty_arg
            ).into()
        }
    } else {
        quote! {
            {
                let __specta_generic_base = types.generic_substitution_cursor();
                // The element type is inferred from the `push`es below and from
                // `init_with_sentinel`, so no annotation is needed.
                let mut __specta_instantiation_generics = ::std::vec::Vec::new();
                #(#instantiation_steps)*
                let __specta_generic_after = types.generic_substitution_cursor();

                let reference = datatype::NamedDataType::init_with_sentinel(
                    SENTINEL,
                    &__specta_instantiation_generics,
                    &[#(#instantiation_generic_identities),*],
                    #has_const_param,
                    false,
                    __specta_generic_base,
                    types,
                    |types, ndt| {
                        ndt.name = Cow::Borrowed(#name);
                        ndt.module_path = Cow::Borrowed(module_path!());
                        #generics
                        #docs
                        #deprecated

                        #ndt_ty
                    },
                    #build_ty_arg
                );

                types.set_generic_substitution_cursor(__specta_generic_after);

                reference.into()
            }
        }
    };

    let definition = if container_attrs.inline {
        quote!(datatype::inline(types, |types| #definition))
    } else {
        definition
    };

    Ok(quote! {
        #[automatically_derived]
        impl #bounds #crate_ref::Type for #ident #type_args #where_bound {
            fn definition(types: &mut #crate_ref::Types) -> #crate_ref::datatype::DataType {
                use std::borrow::Cow;
                use #crate_ref::datatype;

                static SENTINEL: &str = concat!(module_path!(), "::", stringify!(#raw_ident));

                #build_placeholders

                fn build #build_ty_bounds (types: &mut #crate_ref::Types #declaration_param) -> datatype::DataType #build_ty_where_bound {
                    #dt_expr
                }

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
fn placeholder_definitions(crate_ref: &syn::Path, params: &[syn::Ident]) -> TokenStream {
    let definitions = params.iter().map(|ident| {
        let placeholder = placeholder_ident(ident);
        let name = ident.to_string();

        quote! {
            #[allow(non_camel_case_types)]
            pub struct #placeholder;
            impl #crate_ref::Type for #placeholder {
                fn definition(_: &mut #crate_ref::Types) -> datatype::DataType {
                    datatype::Generic::new(Cow::Borrowed(#name)).into()
                }
            }
        }
    });

    quote!(#(#definitions)*)
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
