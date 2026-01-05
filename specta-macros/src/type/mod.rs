use attr::*;
use r#enum::parse_enum;
use quote::{ToTokens, format_ident, quote};
use r#struct::parse_struct;
use syn::{Data, DeriveInput, GenericParam, parse};

use crate::utils::{parse_attrs, unraw_raw_ident};

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
    let mut attrs = parse_attrs(attrs)?;

    // TODO: We wanna drain from `ContainerAttrs` + do on per-field

    let container_attrs = ContainerAttr::from_attrs(&mut attrs)?;
    let crate_ref = container_attrs.crate_name.clone().unwrap_or(quote!(specta));

    let ident = container_attrs
        .remote
        .clone()
        .unwrap_or_else(|| raw_ident.to_token_stream());

    let name = container_attrs.rename.clone().unwrap_or_else(|| {
        unraw_raw_ident(&format_ident!("{}", raw_ident.to_string())).to_token_stream()
    });

    let (dt_type, dt_impl) = match data {
        Data::Struct(data) => parse_struct(&container_attrs, &crate_ref, data),
        Data::Enum(data) => parse_enum(
            &EnumAttr::from_attrs(&container_attrs, &mut attrs)?,
            &container_attrs,
            data,
        ),
        Data::Union(data) => Err(syn::Error::new_spanned(
            data.union_token,
            "specta: Union types are not supported by Specta yet!",
        )),
    }?;

    // let container_attrs = attrs
    //     .iter()
    //     .map(|attr| lower_attribute(attr).map(|attr| attr.to_tokens()))
    //     .collect::<Result<Vec<_>, _>>()?;
    // let inlines = match data {
    //     Data::Struct(_) => quote!(datatype::Struct({
    //         #inlines
    //         // *e.attributes_mut() = vec![];
    //         e
    //     })),
    //     Data::Enum(_) => quote!(datatype::Enum(#inlines)),
    //     Data::Union(_) => unreachable!(),
    // };

    // // The expectation is that when an attribute is processed it will be removed so if any are left over we know they are invalid
    // // but we only throw errors for Specta-specific attributes so we don't continually break other attributes.
    // if let Some(attrs) = attrs.iter().find(|attr| attr.key == "specta") {
    //     match &attrs.value {
    //         Some(AttributeValue::Attribute { attr, .. }) => {
    //             if let Some(attr) = attr.first() {
    //                 return Err(syn::Error::new(
    //                     attr.key.span(),
    //                     format!(
    //                         "specta: Found unsupported container attribute '{}'",
    //                         attr.key
    //                     ),
    //                 ));
    //             }
    //         }
    //         _ => {
    //             return Err(syn::Error::new(
    //                 attrs.key.span(),
    //                 "specta: invalid formatted attribute",
    //             ));
    //         }
    //     }
    // }

    let bounds = generics_with_ident_and_bounds_only(generics);
    let type_args = generics_with_ident_only(generics);
    let where_bound = add_type_to_where_clause(&quote!(#crate_ref::Type), generics);

    // let flatten_impl = can_flatten.then(|| {
    //     quote! {
    //         #[automatically_derived]
    //         impl #bounds #crate_ref::Flatten for #ident #type_args #where_bound {}
    //     }
    // });

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
            Some(quote!((internal::construct::generic_data_type(#i_str), <#i as #crate_ref::Type>::definition(types))))
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

                    static SENTINEL: () = ();
                    datatype::DataType::Reference(
                        datatype::NamedDataType::init_with_sentinel(
                            vec![#(#reference_generics),*],
                            #inline,
                            types,
                            &SENTINEL,
                            |types, ndt| {
                                ndt.set_name(Cow::Borrowed(#name));
                                ndt.set_docs(Cow::Borrowed(#comments));
                                ndt.set_deprecated(#deprecated);
                                ndt.set_module_path(Cow::Borrowed(module_path!()));
                                *ndt.generics_mut() = vec![#(#definition_generics),*];
                                ndt.set_ty({
                                    #shadow_generics

                                    datatype::DataType::#dt_type({
                                        let mut e = datatype::#dt_type::new();
                                        // *e.attributes_mut() = vec![#(#lowered_attrs),*]; // TODO
                                        #dt_impl
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
