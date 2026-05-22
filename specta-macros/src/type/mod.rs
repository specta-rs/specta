use attr::*;
use r#enum::parse_enum;
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use r#struct::parse_struct;
use syn::{Data, DeriveInput, GenericParam, parse};

use crate::utils::{AttrExtract, parse_attrs, unraw_raw_ident};

use self::generics::{
    add_type_to_where_clause, build_type_where_clause, generics_with_ident_and_bounds_only,
    generics_with_ident_only, generics_with_ident_only_and_const_ty, type_with_inferred_lifetimes,
    used_type_params,
};

pub(crate) mod attr;
mod r#enum;
mod field;
mod generics;
#[cfg(feature = "serde")]
mod serde;
mod r#struct;

#[derive(Copy, Clone)]
pub(super) enum AttributeScope {
    Container,
    Variant,
    Field,
}

pub(super) fn build_runtime_attributes(
    crate_ref: &TokenStream,
    scope: AttributeScope,
    attrs: TokenStream,
    raw_attrs: &[syn::Attribute],
    skip_attrs: &[String],
) -> syn::Result<Option<TokenStream>> {
    #[cfg(feature = "serde")]
    let serde_insert = serde::lower_runtime_attributes(
        crate_ref,
        scope,
        &raw_attrs
            .iter()
            .filter(|attr| {
                !attr
                    .path()
                    .segments
                    .last()
                    .is_some_and(|segment| skip_attrs.contains(&segment.ident.to_string()))
            })
            .cloned()
            .collect::<Vec<_>>(),
    )?;
    #[cfg(not(feature = "serde"))]
    let serde_insert: Option<TokenStream> = {
        let _ = crate_ref;
        let _ = scope;
        let _ = raw_attrs;
        let _ = skip_attrs;
        None
    };

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

    let container_attrs = ContainerAttr::from_attrs(&mut attrs)?;
    let crate_ref = container_attrs.crate_name.clone().unwrap_or(quote!(specta));

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

    let name = unraw_raw_ident(&format_ident!("{}", raw_ident.to_string())).to_token_stream();

    reject_unknown_specta_attrs(&attrs, Scope::Container)?;

    let container_runtime_attrs = build_runtime_attributes(
        &crate_ref,
        AttributeScope::Container,
        quote!(s.attributes),
        raw_attrs,
        &container_attrs.skip_attrs,
    )?;
    let enum_runtime_attrs = build_runtime_attributes(
        &crate_ref,
        AttributeScope::Container,
        quote!(en.attributes),
        raw_attrs,
        &container_attrs.skip_attrs,
    )?;
    let container_runtime_attrs =
        if container_runtime_attrs.is_some() || enum_runtime_attrs.is_some() {
            quote! {
                match &mut e {
                    datatype::DataType::Struct(s) => { #container_runtime_attrs }
                    datatype::DataType::Enum(en) => { #enum_runtime_attrs }
                    _ => unreachable!("specta derive generated non-container datatype"),
                }
            }
        } else {
            quote!()
        };

    let dt_expr = if let Some(container_ty) = &container_attrs.r#type {
        let container_ty = type_with_inferred_lifetimes(container_ty);
        quote!(datatype::inline(types, |types| <#container_ty as #crate_ref::Type>::definition(types)))
    } else {
        let dt_expr = match data {
            Data::Struct(data) => parse_struct(&crate_ref, &container_attrs, data),
            Data::Enum(data) => parse_enum(&crate_ref, &container_attrs, data),
            Data::Union(data) => Err(syn::Error::new_spanned(
                data.union_token,
                "specta: Union types are not supported by Specta yet!",
            )),
        }?;

        quote! {
                let mut e = #dt_expr;
                #container_runtime_attrs
                e
        }
    };

    let bounds = generics_with_ident_and_bounds_only(generics);
    let type_args = generics_with_ident_only(generics);
    let has_const_param = generics
        .params
        .iter()
        .any(|param| matches!(param, GenericParam::Const(_)));
    let used_generic_types = used_type_params(
        generics,
        data,
        container_attrs.r#type.as_ref(),
        &container_attrs.skip_attrs,
    )?;
    let where_bound = add_type_to_where_clause(
        &quote!(#crate_ref::Type),
        generics,
        container_attrs.bound.as_deref(),
        &used_generic_types,
    );
    let build_ty_where_bound =
        build_type_where_clause(&quote!(#crate_ref::Type), &used_generic_types);
    let build_ty_bounds = generics_with_ident_only_and_const_ty(generics);

    let build_ty_placeholder_args = generic_type_and_const_args(generics, |ty| {
        format_ident!("PLACEHOLDER_{}", ty.ident).to_token_stream()
    });
    let build_ty_passthrough_args =
        generic_type_and_const_args(generics, |ty| ty.ident.to_token_stream());

    let (generic_placeholders, shadow_generics): (Vec<_>, Vec<_>) = generics
        .params
        .iter()
        .filter_map(|param| match param {
            GenericParam::Lifetime(_) | GenericParam::Const(_) => None,
            GenericParam::Type(t) => {
                let ident = &t.ident;
                let ident_str = ident.to_string();
                let placeholder_ident = format_ident!("PLACEHOLDER_{ident}");
                Some((
                    quote!(
                        pub struct #placeholder_ident;
                        impl #crate_ref::Type for #placeholder_ident {
                            fn definition(_: &mut #crate_ref::Types) -> datatype::DataType {
                                datatype::Generic::new(Cow::Borrowed(#ident_str)).into()
                            }
                        }
                    ),
                    quote!(type #ident = #placeholder_ident;),
                ))
            }
        })
        .unzip();

    let (generics_for_ndt, instantiation_generics) = generics
        .params
        .iter()
        .map(|param| match param {
            GenericParam::Lifetime(_) | GenericParam::Const(_) => Ok(None),
            GenericParam::Type(t) => {
                let i = &t.ident;

                let skip_default = parse_attrs(&t.attrs)?
                    .extract("specta", "skip_default_generic")
                    .map(|attr| attr.parse_bool_or_true())
                    .transpose()?
                    .unwrap_or(false);

                if !used_generic_types.iter().any(|used| used == i) && !skip_default {
                    return Ok(None);
                }

                let i_str = i.to_string();
                let default = match (&t.default, skip_default) {
                    (_, true) | (None, false) => quote!(None),
                    (Some(default), false) => {
                        quote!(Some(<#default as #crate_ref::Type>::definition(types)))
                    }
                };
                let reference = used_generic_types.iter().any(|used| used == i).then(|| {
                    quote!((
                        #crate_ref::datatype::Generic::new(Cow::Borrowed(#i_str)),
                        <#i as #crate_ref::Type>::definition(types),
                    ))
                });
                Ok(Some((
                    quote!(#crate_ref::datatype::GenericDefinition::new(
                        Cow::Borrowed(#i_str),
                        #default,
                    )),
                    reference,
                )))
            }
        })
        .collect::<syn::Result<Vec<_>>>()?
        .into_iter()
        .flatten()
        .collect::<(Vec<_>, Vec<_>)>();

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
                ndt.generics = Cow::Owned(vec![#(#generics_for_ndt),*]);
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
    let deprecated = container_attrs.common.deprecated.map(|deprecated| {
        let tokens = deprecated_as_tokens(deprecated);
        quote!(ndt.deprecated = #tokens;)
    });

    let ndt_ty = (!container_attrs.inline).then(|| {
        quote! {
            #(#generic_placeholders)*
            #(#shadow_generics)*

            ndt.ty = Some(build #build_ty_placeholder_args (types));
        }
    });
    let definition = quote! {
        datatype::DataType::Reference(
            datatype::NamedDataType::init_with_sentinel(
                SENTINEL,
                &[#(#instantiation_generics),*],
                #has_const_param,
                false,
                types,
                |types, ndt| {
                    ndt.name = Cow::Borrowed(#name);
                    ndt.module_path = Cow::Borrowed(module_path!());
                    #generics
                    #docs
                    #deprecated;

                    #ndt_ty
                },
                build #build_ty_passthrough_args
            )
        )
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

                fn build #build_ty_bounds (types: &mut #crate_ref::Types) -> datatype::DataType #build_ty_where_bound {
                    #dt_expr
                }

                #definition
            }
        }

        #collect

    }
    .into())
}
