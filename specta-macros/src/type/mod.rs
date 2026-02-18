use attr::*;
use r#enum::parse_enum;
use quote::{ToTokens, format_ident, quote};
use r#struct::parse_struct;
use syn::{Data, DeriveInput, GenericParam, parse};

use crate::utils::{parse_attrs, unraw_raw_ident};

use self::lower_attr::lower_attribute;

use self::generics::{
    add_type_to_where_clause, generics_with_ident_and_bounds_only, generics_with_ident_only,
};

pub(crate) mod attr;
mod r#enum;
mod field;
mod generics;
mod lower_attr;
mod r#struct;

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

    // Lower the container attributes to RuntimeAttribute tokens
    // We use the raw attrs from DeriveInput, not the parsed ones
    // This follows the same pattern as variant attribute lowering in enum.rs:58-71
    let lowered_attrs = raw_attrs
        .iter()
        .filter(|attr| {
            let path = attr.path().to_token_stream().to_string();
            !container_attrs.skip_attrs.contains(&path) && path != "specta"
        })
        .filter_map(|attr| lower_attribute(attr).transpose())
        .map(|result| result.map(|attr| attr.to_tokens()))
        .collect::<Result<Vec<_>, _>>()?;

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

    let (dt_type, dt_impl) = match data {
        Data::Struct(data) => parse_struct(&crate_ref, &container_attrs, data),
        Data::Enum(data) => parse_enum(&crate_ref, &container_attrs, data),
        Data::Union(data) => Err(syn::Error::new_spanned(
            data.union_token,
            "specta: Union types are not supported by Specta yet!",
        )),
    }?;

    let bounds = generics_with_ident_and_bounds_only(generics);
    let type_args = generics_with_ident_only(generics);
    let where_bound = add_type_to_where_clause(
        &quote!(#crate_ref::Type),
        generics,
        container_attrs.bound.as_deref(),
    );

    let shadow_generics = {
        let g = generics.params.iter().map(|param| match param {
            // Pulled from outside
            GenericParam::Lifetime(_) | GenericParam::Const(_) => quote!(),
            // We shadow the generics to replace them.
            GenericParam::Type(t) => {
                let ident = &t.ident;
                let placeholder_ident = format_ident!("PLACEHOLDER_{}", t.ident);
                quote!(type #ident = datatype::GenericPlaceholder<#placeholder_ident>;)
            }
        });

        quote!(#(#g)*)
    };

    let generic_placeholders = generics.params.iter().filter_map(|param| match param {
        GenericParam::Lifetime(_) | GenericParam::Const(_) => None,
        GenericParam::Type(t) => {
            let ident = format_ident!("PLACEHOLDER_{}", t.ident);
            let ident_str = t.ident.to_string();
            Some(quote!(
                pub struct #ident;
                impl datatype::ConstGenericPlaceholder for #ident {
                    const PLACEHOLDER: &'static str = #ident_str;
                }
            ))
        }
    });

    let collect = (cfg!(feature = "DO_NOT_USE_collect") && container_attrs.collect.unwrap_or(true))
        .then(|| {
            let export_fn_name = format_ident!("__push_specta_type_{}", raw_ident);

            let generic_params = generics
                .params
                .iter()
                .filter(|param| matches!(param, syn::GenericParam::Type(_)))
                .map(|_| quote! { () });

            quote! {
                #[allow(non_snake_case)]
                #[#crate_ref::collect::internal::ctor::ctor(anonymous, crate_path = #crate_ref::collect::internal::ctor)]
                unsafe fn #export_fn_name() {
                    #crate_ref::collect::internal::register::<#ident<#(#generic_params),*>>();
                }
            }
        });

    let comments = &container_attrs.common.doc;
    let inline = container_attrs.inline;
    let deprecated = container_attrs.common.deprecated_as_tokens();

    let reference_generics = generics.params.iter().filter_map(|param| match param {
        GenericParam::Lifetime(_) | GenericParam::Const(_) => None,
        GenericParam::Type(t) => {
            let i = &t.ident;
            let i_str = i.to_string();
            Some(quote!((#crate_ref::datatype::Generic::new(#i_str), <#i as #crate_ref::Type>::definition(types))))
        }
    });

    let definition_generics = generics.params.iter().filter_map(|p| match p {
        GenericParam::Type(t) => {
            let ident = t.ident.to_string();
            Some(quote!(std::borrow::Cow::Borrowed(#ident).into()))
        }
        _ => None,
    });

    Ok(quote! {
        #[allow(non_camel_case_types)]
        const _: () = {
            use std::borrow::Cow;
            use #crate_ref::{datatype, internal};

            #[automatically_derived]
            impl #bounds #crate_ref::Type for #ident #type_args #where_bound {
                fn definition(types: &mut #crate_ref::TypeCollection) -> datatype::DataType {
                    #(#generic_placeholders)*

                    static SENTINEL: &str = concat!(module_path!(), "::", stringify!(#raw_ident));
                    datatype::DataType::Reference(
                        datatype::NamedDataType::init_with_sentinel(
                            vec![#(#reference_generics),*],
                            #inline,
                            types,
                            SENTINEL,
                            |types, ndt| {
                                ndt.set_name(Cow::Borrowed(#name));
                                ndt.set_docs(Cow::Borrowed(#comments));
                                ndt.set_deprecated(#deprecated);
                                ndt.set_module_path(Cow::Borrowed(module_path!()));
                                *ndt.generics_mut() = vec![#(#definition_generics),*];
                                ndt.set_ty({
                                    #shadow_generics

                                    datatype::DataType::#dt_type({
                                        #dt_impl
                                        *e.attributes_mut() = vec![#(#lowered_attrs),*];
                                        e
                                    })
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
