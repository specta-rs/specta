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
    dt: TokenStream,
}

impl Names {
    fn new(crate_ref: syn::Path) -> Self {
        Self {
            crate_ref,
            ty: parse_quote!(__T),
            types: parse_quote!(__Types),
            dt: quote!(__D),
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
/// `target` is the place expression the attributes are written to (`e.attributes`,
/// `field0.attributes`, ...), so the caller decides which value they belong to and no binding of
/// its own is needed.
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
    target: TokenStream,
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
    serde::lower_runtime_attributes(&names.ty, scope, &target, raw_attrs)
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
                quote!(s.attributes),
                raw_attrs,
                &container_attrs.skip_attrs,
            )?;
            let enum_attrs = build_runtime_attributes(
                &names,
                AttributeScope::Container,
                quote!(en.attributes),
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
                quote!(s.attributes),
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
                quote!(e.attributes),
                raw_attrs,
                &container_attrs.skip_attrs,
            )?;
            None
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
        let resolve = resolve_ty_expr(&names, &container_ty, &used_generic_types, &bounded, true);
        quote!(#dt::inline(types, |types| #resolve))
    } else {
        match data {
            Data::Struct(data) => parse_struct(
                &names,
                &container_attrs,
                data,
                &used_generic_types,
                &bounded,
            ),
            Data::Enum(data) => parse_enum(
                &names,
                &container_attrs,
                data,
                &used_generic_types,
                &bounded,
                enum_attrs_apply.as_ref(),
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
    let build_placeholders = placeholder_definitions(&names, &placeholder_params);

    let build_ty_passthrough_args =
        generic_type_and_const_args(generics, |ty| ty.ident.to_token_stream());

    // `generics_for_ndt` is the declaration's generic list;
    // `instantiation_steps` resolves the type's own arguments in parameter
    // order, one `Some` per parameter that is resolved and a `None` for one that
    // is only stepped over. Unused parameters still consume a substitution slot
    // so the parameters after them stay aligned (issue #609).
    let mut generics_for_ndt: Vec<TokenStream> = Vec::new();
    let mut instantiation_steps: Vec<Option<TokenStream>> = Vec::new();

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
            instantiation_steps.push(None);

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
                let resolve =
                    resolve_ty_expr(&names, default, &used_generic_types, &bounded, false);
                quote!(Some(#resolve))
            }
        };

        generics_for_ndt.push(quote!(#dt::GenericDefinition::new(
            Cow::Borrowed(#i_str),
            #default,
        )));
        instantiation_steps.push(Some(quote!((
            #dt::Generic::new(#i_str.into()),
            types.define_param::<#i>(),
        ))));
    }

    // `init_with_sentinel` only ever borrows the argument list, so it is emitted
    // as an array. A stepped-over parameter is folded into the element that
    // follows it as a statement, which keeps the cursor advancing in declaration
    // order while keeping the list itself an array; a run of them after the last
    // resolved parameter is emitted once the array exists.
    let mut generics_elements: Vec<TokenStream> = Vec::new();
    let mut pending_skips = 0;
    for step in &instantiation_steps {
        let Some(entry) = step else {
            pending_skips += 1;
            continue;
        };

        generics_elements.push(if pending_skips == 0 {
            entry.clone()
        } else {
            let skips = std::iter::repeat_n(quote!(types.skip_param_slot();), pending_skips);
            pending_skips = 0;
            quote!({
                #(#skips)*
                #entry
            })
        });
    }
    let trailing_skips = std::iter::repeat_n(quote!(types.skip_param_slot();), pending_skips)
        .collect::<TokenStream>();

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
    let docs = (!container_attrs.common.doc.is_empty()).then(|| {
        let docs = &container_attrs.common.doc;
        quote! {
            ndt.docs = #docs.into();
        }
    });
    let instantiation_generic_identities = used_generic_types
        .iter()
        .map(|generic| quote!(std::any::type_name::<#generic>()));
    let deprecated = container_attrs
        .common
        .deprecated
        .map(|deprecated| deprecated_as_tokens(dt, deprecated));
    // The same expression is either assigned in the metadata closure or passed
    // straight to `Types::define_named`, never both.
    let deprecated_stmt = deprecated
        .as_ref()
        .map(|tokens| quote!(ndt.deprecated = #tokens;));
    let deprecated_arg = deprecated.clone().unwrap_or_else(|| quote!(None));

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
    let definition =
        if instantiation_steps.is_empty() && !has_const_param && !container_attrs.inline {
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
        } else if instantiation_steps.is_empty() {
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
            // A container can declare parameters it never resolves
            // (`struct S<T>(PhantomData<T>)` with `#[specta(type = ...)]`), in which
            // case only the skips remain and there is nothing to build a list from.
            let generics_list = (!generics_elements.is_empty()).then(|| {
                quote! {
                    let __generics = [#(#generics_elements),*];
                }
            });
            let generics_ref = if generics_elements.is_empty() {
                quote!(&[])
            } else {
                quote!(&__generics)
            };

            quote! {
                {
                    let __base = types.generic_substitution_cursor();
                    #generics_list
                    #trailing_skips
                    let __after = types.generic_substitution_cursor();

                    let __reference = #dt::NamedDataType::init_with_sentinel(
                        #sentinel,
                        #generics_ref,
                        &[#(#instantiation_generic_identities),*],
                        #has_const_param,
                        false,
                        __base,
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
                    );

                    types.set_generic_substitution_cursor(__after);

                    __reference.into()
                }
            }
        };

    let definition = if container_attrs.inline {
        quote!(#dt::inline(types, |types| #definition))
    } else {
        definition
    };

    let imports = names.imports();
    let types = &names.types;

    Ok(quote! {
        #[automatically_derived]
        impl #bounds #crate_ref::Type for #ident #type_args #where_bound {
            // The aliases are imported into the body, so this signature — which
            // is checked against the trait's — keeps the real paths.
            fn definition(types: &mut #crate_ref::Types) -> #crate_ref::datatype::DataType {
                #cow_import
                #imports

                #build_placeholders

                fn build #build_ty_bounds (types: &mut #types #declaration_param) -> #dt::DataType #build_ty_where_bound {
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
