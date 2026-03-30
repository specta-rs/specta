use attr::*;
use r#enum::parse_enum;
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use r#struct::parse_struct;
use syn::{Data, DeriveInput, GenericParam, parse};

use crate::utils::{parse_attrs, unraw_raw_ident};

use self::generics::{
    add_type_to_where_clause, all_type_param_idents, generics_with_ident_and_bounds_only,
    generics_with_ident_only, has_associated_type_usage, used_associated_type_paths,
    used_direct_type_params, used_type_params,
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
    raw_attrs: &[syn::Attribute],
    skip_attrs: &[String],
) -> syn::Result<TokenStream> {
    let metas = raw_attrs
        .iter()
        .filter(|attr| {
            let path = attr.path().to_token_stream().to_string();
            !skip_attrs.contains(&path) && path != "specta"
        })
        .map(|attr| attr.meta.to_token_stream())
        .collect::<Vec<_>>();

    #[cfg(feature = "serde")]
    let serde_insert = serde::lower_runtime_attributes(crate_ref, scope, raw_attrs)?;
    #[cfg(not(feature = "serde"))]
    let serde_insert: Option<TokenStream> = {
        let _ = crate_ref;
        let _ = scope;
        None
    };

    if metas.is_empty() && serde_insert.is_none() {
        return Ok(quote!(datatype::Attributes::default()));
    }

    Ok(quote!({
        let mut attrs = datatype::Attributes::default();
        #serde_insert
        attrs
    }))
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

    // Check for unknown specta attributes after all parsing is done
    // Since extract() removes consumed attributes, any remaining ones are unknown
    if let Some(attr) = attrs.iter().find(|attr| attr.source == "specta") {
        // Check if it's an invalid formatted attribute (like #[specta] or #[specta = "..."])
        match &attr.value {
            None
            | Some(crate::utils::AttributeValue::Lit(_))
            | Some(crate::utils::AttributeValue::Path(_)) => {
                return Err(syn::Error::new(
                    attr.key.span(),
                    "specta: invalid formatted attribute",
                ));
            }
            Some(crate::utils::AttributeValue::Expr(_)) => {
                return Err(syn::Error::new(
                    attr.key.span(),
                    "specta: invalid formatted attribute",
                ));
            }
            Some(crate::utils::AttributeValue::Attribute {
                attr: inner_attrs, ..
            }) => {
                // If there are nested attributes remaining, report the first one
                if let Some(inner_attr) = inner_attrs.first() {
                    if let Some(message) =
                        migration_hint(Scope::Container, &inner_attr.key.to_string())
                    {
                        return Err(syn::Error::new(inner_attr.key.span(), message));
                    }

                    return Err(syn::Error::new(
                        inner_attr.key.span(),
                        format!(
                            "specta: Found unsupported container attribute '{}'",
                            inner_attr.key
                        ),
                    ));
                }
                // If the nested list is empty, it's an invalid format
                return Err(syn::Error::new(
                    attr.key.span(),
                    "specta: invalid formatted attribute",
                ));
            }
        }
    }

    let container_runtime_attrs = build_runtime_attributes(
        &crate_ref,
        AttributeScope::Container,
        raw_attrs,
        &container_attrs.skip_attrs,
    )?;

    let dt_expr = if let Some(container_ty) = &container_attrs.r#type {
        quote!(<#container_ty as #crate_ref::Type>::definition(types))
    } else {
        let dt_expr = match data {
            Data::Struct(data) => parse_struct(&crate_ref, &container_attrs, data),
            Data::Enum(data) => parse_enum(&crate_ref, &container_attrs, data),
            Data::Union(data) => Err(syn::Error::new_spanned(
                data.union_token,
                "specta: Union types are not supported by Specta yet!",
            )),
        }?;

        quote!(
            {
                let mut e = #dt_expr;
                match &mut e {
                    datatype::DataType::Struct(s) => *s.attributes_mut() = #container_runtime_attrs,
                    datatype::DataType::Enum(en) => *en.attributes_mut() = #container_runtime_attrs,
                    _ => unreachable!("specta derive generated non-container datatype"),
                }
                e
            }
        )
    };

    let bounds = generics_with_ident_and_bounds_only(generics);
    let type_args = generics_with_ident_only(generics);
    let used_generic_types = used_type_params(generics, data, container_attrs.r#type.as_ref());
    let all_generic_type_idents = all_type_param_idents(generics);
    let used_direct_generics =
        used_direct_type_params(&used_generic_types, &all_generic_type_idents);
    let used_associated_paths = used_associated_type_paths(&used_generic_types);
    let where_bound = add_type_to_where_clause(
        &quote!(#crate_ref::Type),
        generics,
        container_attrs.bound.as_deref(),
        used_direct_generics,
        used_associated_paths,
    );

    let (generic_placeholders, shadow_generics): (Vec<_>, Vec<_>) = generics
        .params
        .iter()
        .filter_map(|param| match param {
            GenericParam::Lifetime(_) | GenericParam::Const(_) => None,
            GenericParam::Type(t) => {
                let ident = &t.ident;
                let placeholder_ident = format_ident!("PLACEHOLDER_{ident}");
                Some((
                    quote!(
                        pub struct #placeholder_ident;
                        impl #crate_ref::Type for #placeholder_ident {
                            fn definition(_: &mut #crate_ref::Types) -> datatype::DataType {
                                datatype::GenericReference::new::<Self>().into()
                            }
                        }
                    ),
                    quote!(type #ident = #placeholder_ident;),
                ))
            }
        })
        .unzip();

    let (generics_for_ndt, generics_for_ref): (Vec<_>, Vec<_>) = generics
        .params
        .iter()
        .filter_map(|param| match param {
            GenericParam::Lifetime(_) | GenericParam::Const(_) => None,
            GenericParam::Type(t) => {
                let i = &t.ident;
                let placeholder_ident = format_ident!("PLACEHOLDER_{}", t.ident);
                if !used_direct_generics.iter().any(|used| used == i) {
                    return None;
                }
                let i_str = i.to_string();
                Some((
                    quote!((
                        #crate_ref::datatype::GenericReference::new::<#placeholder_ident>(),
                        Cow::Borrowed(#i_str),
                    )),
                    quote!((
                        #crate_ref::datatype::GenericReference::new::<#placeholder_ident>(),
                        <#i as #crate_ref::Type>::definition(types),
                    )),
                ))
            }
        })
        .unzip();

    let collect = (cfg!(feature = "DO_NOT_USE_collect") && container_attrs.collect.unwrap_or(true))
        .then(|| {
            let export_fn_name = format_ident!("__push_specta_type_{}", raw_ident);

            let generic_params = generics
                .params
                .iter()
                .filter(|param| matches!(param, syn::GenericParam::Type(_)))
                .map(|_| quote! { () });

            quote! {
                #[allow(unsafe_code, non_snake_case)]
                #[#crate_ref::collect::internal::ctor::ctor(anonymous, crate_path = #crate_ref::collect::internal::ctor)]
                unsafe fn #export_fn_name() {
                    #crate_ref::collect::internal::register::<#ident<#(#generic_params),*>>();
                }
            }
        });

    let comments = &container_attrs.common.doc;
    let inline = container_attrs.inline;
    let deprecated = container_attrs.common.deprecated_as_tokens();

    let shadow_generic_aliases = if has_associated_type_usage(&used_generic_types) {
        quote!()
    } else {
        quote!(#(#shadow_generics)*)
    };

    Ok(quote! {
        #[allow(non_camel_case_types)]
        const _: () = {
            use std::borrow::Cow;
            use #crate_ref::datatype;

            #[automatically_derived]
            impl #bounds #crate_ref::Type for #ident #type_args #where_bound {
                fn definition(types: &mut #crate_ref::Types) -> datatype::DataType {
                    #(#generic_placeholders)*

                    static SENTINEL: &str = concat!(module_path!(), "::", stringify!(#raw_ident));
                    static GENERICS: &[(datatype::GenericReference, Cow<'static, str>)] = &[#(#generics_for_ndt),*];
                    datatype::DataType::Reference(
                        datatype::NamedDataType::init_with_sentinel(
                            GENERICS,
                            vec![#(#generics_for_ref),*],
                            #inline,
                            types,
                            SENTINEL,
                            |types, ndt| {
                                ndt.set_name(Cow::Borrowed(#name));
                                ndt.set_docs(Cow::Borrowed(#comments));
                                ndt.set_deprecated(#deprecated);
                                ndt.set_module_path(Cow::Borrowed(module_path!()));
                                ndt.set_ty({
                                    #shadow_generic_aliases

                                    #dt_expr
                                });
                            }
                        )
                    )
                }
            }

            #collect
        };

    }
    .into())
}
