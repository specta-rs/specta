use crate::utils::{parse_attrs, unraw_raw_ident, AttributeValue};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{spanned::Spanned, DataStruct, Field, Fields, GenericParam, Generics};

use super::attr::*;

pub fn decode_field_attrs(field: &Field) -> syn::Result<FieldAttr> {
    // We pass all the attributes at the start and when decoding them pop them off the list.
    // This means at the end we can check for any that weren't consumed and throw an error.
    let mut attrs = parse_attrs(&field.attrs)?;
    let field_attrs = FieldAttr::from_attrs(&mut attrs)?;

    // The expectation is that when an attribute is processed it will be removed so if any are left over we know they are invalid
    // but we only throw errors for Specta-specific attributes so we don't continually break other attributes.
    if let Some(attrs) = attrs.iter().find(|attr| attr.key == "specta") {
        match &attrs.value {
            Some(AttributeValue::Attribute { attr, .. }) => {
                if let Some(attr) = attr.first() {
                    return Err(syn::Error::new(
                        attr.key.span(),
                        format!("specta: Found unsupported field attribute '{}'", attr.key),
                    ));
                }
            }
            _ => {
                return Err(syn::Error::new(
                    attrs.key.span(),
                    "specta: invalid formatted attribute",
                ))
            }
        }
    }

    Ok(field_attrs)
}

pub fn parse_struct(
    name: &TokenStream,
    container_attrs: &ContainerAttr,
    generics: &Generics,
    crate_ref: &TokenStream,
    data: &DataStruct,
) -> syn::Result<(TokenStream, bool)> {
    let generic_idents = generics
        .params
        .iter()
        .filter_map(|p| match p {
            GenericParam::Type(t) => Some(&t.ident),
            _ => None,
        })
        .enumerate()
        .collect::<Vec<_>>();

    let reference_generics = generic_idents.iter().map(|(i, ident)| {
        quote! {
            generics
                .get(#i)
                .cloned()
                .unwrap_or_else(|| <#ident as #crate_ref::Type>::reference(type_map, &[]).inner)
        }
    });

    let definition_generics = generic_idents.iter().map(|(_, ident)| {
        let ident = ident.to_string();
        quote!(std::borrow::Cow::Borrowed(#ident).into())
    });

    let definition = if container_attrs.transparent {
        if let Fields::Unit = data.fields {
            return Err(syn::Error::new(
                data.fields.span(),
                "specta: unit structs cannot be transparent",
            ));
        }

        let (field_ty, field_attrs) = match data.fields {
            Fields::Named(_) => {
                let fields = data
                    .fields
                    .iter()
                    .map(|field| decode_field_attrs(field).map(|v| (field.ty.clone(), v)))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .filter(|(_, attrs)| !attrs.skip)
                    .collect::<Vec<_>>();

                if fields.len() != 1 {
                    return Err(syn::Error::new(
                        data.fields.span(),
                        "specta: transparent structs must have exactly one field",
                    ));
                }

                fields.into_iter().next().expect("fields.len() != 1")
            }
            Fields::Unnamed(_) => {
                let fields = data
                    .fields
                    .iter()
                    .map(|field| decode_field_attrs(field).map(|v| (field.ty.clone(), v)))
                    .collect::<Result<Vec<_>, _>>()?
                    .into_iter()
                    .filter(|(_, attrs)| !attrs.skip)
                    .collect::<Vec<_>>();

                if fields.len() != 1 {
                    return Err(syn::Error::new(
                        data.fields.span(),
                        "specta: transparent structs must have exactly one field",
                    ));
                }

                fields.into_iter().next().expect("fields.len() != 1")
            }
            Fields::Unit => {
                return Err(syn::Error::new(
                    data.fields.span(),
                    "specta: transparent structs must have exactly one field",
                ));
            }
        };

        let field_ty = field_attrs.r#type.as_ref().unwrap_or(&field_ty);

        // let ty = construct_datatype(
        //     format_ident!("ty"),
        //     field_ty,
        //     &generic_idents,
        //     crate_ref,
        //     field_attrs.inline,
        // )?;

        if field_attrs.inline {
            todo!();
        }

        quote!(Some(<#field_ty as #crate_ref::Type>::definition(type_map)))
    } else {
        let fields = match &data.fields {
            Fields::Named(_) => {
                let fields =
                    data.fields
                        .iter()
                        .map(|field| {
                            let field_attrs = decode_field_attrs(field)?;
                            let field_ty = field_attrs.r#type.as_ref().unwrap_or(&field.ty);

                            let field_ident_str = unraw_raw_ident(field.ident.as_ref().unwrap());

                            let field_name =
                                match (field_attrs.rename.clone(), container_attrs.rename_all) {
                                    (Some(name), _) => name,
                                    (_, Some(inflection)) => {
                                        inflection.apply(&field_ident_str).to_token_stream()
                                    }
                                    (_, _) => field_ident_str.to_token_stream(),
                                };

                            let deprecated = field_attrs.common.deprecated_as_tokens(crate_ref);
                            let optional = field_attrs.optional;
                            let flatten = field_attrs.flatten;
                            let doc = field_attrs.common.doc;

                            let ty = field_attrs.skip.then(|| quote!(None)).unwrap_or_else(
                                || {
                                    if field_attrs.skip {
                                        todo!();
                                    }

                                    if field_attrs.flatten {
                                        todo!();
                                        // quote! {
                                        //     fn validate_flatten<T: #crate_ref::Flatten>() {}
                                        //     validate_flatten::<#field_ty>();
                                        //     #crate_ref::internal::flatten::<#field_ty>(SID, type_map, &generics)
                                        // }
                                    }

                                    quote!(Some(<#field_ty as #crate_ref::Type>::definition(type_map)))
                                },
                            );

                            Ok(
                                quote!((#field_name.into(), #crate_ref::internal::construct::field(
                                    #optional,
                                    #flatten,
                                    #deprecated,
                                    #doc.into(),
                                    #ty
                                ))),
                            )
                        })
                        .collect::<syn::Result<Vec<TokenStream>>>()?;

                let tag = container_attrs
                    .tag
                    .as_ref()
                    .map(|t| quote!(Some(#t.into())))
                    .unwrap_or(quote!(None));

                quote!(#crate_ref::internal::construct::fields_named(vec![#(#fields),*], #tag))
            }
            Fields::Unnamed(_) => {
                let fields = data
                    .fields
                    .iter()
                    .map(|field| {
                        let field_attrs = decode_field_attrs(field)?;
                        let field_ty = field_attrs.r#type.as_ref().unwrap_or(&field.ty);

                        let deprecated = field_attrs.common.deprecated_as_tokens(crate_ref);
                        let optional = field_attrs.optional;
                        let flatten = field_attrs.flatten;
                        let doc = field_attrs.common.doc;

                        let ty = field_attrs.skip.then(|| quote!(None))
                            .unwrap_or_else(|| {
                                quote!(Some(<#field_ty as #crate_ref::Type>::definition(type_map)))
                            });

                        Ok(quote!(#crate_ref::internal::construct::field(#optional, #flatten, #deprecated, #doc.into(), #ty)))
                    })
                    .collect::<syn::Result<Vec<TokenStream>>>()?;

                quote!(#crate_ref::internal::construct::fields_unnamed(vec![#(#fields),*]))
            }
            Fields::Unit => quote!(#crate_ref::internal::construct::fields_unit()),
        };

        quote!(#crate_ref::datatype::DataType::Struct(#crate_ref::internal::construct::r#struct(#name.into(), Some(SID), vec![#(#definition_generics),*], #fields)))
    };

    Ok((definition, true))
}
