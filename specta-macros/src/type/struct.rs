use crate::{
    r#type::field::construct_field,
    utils::{parse_attrs_with_filter, unraw_raw_ident},
};
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{DataStruct, Field, Fields, Type, spanned::Spanned};

use super::attr::*;

pub fn decode_field_attrs<'a>(
    field: &'a Field,
    skip_attrs: &[String],
) -> syn::Result<(FieldAttr, &'a [syn::Attribute])> {
    // We pass all the attributes at the start and when decoding them pop them off the list.
    // This means at the end we can check for any that weren't consumed and throw an error.
    let raw_attrs = parse_attrs_with_filter(&field.attrs, skip_attrs)?;
    let mut attrs = raw_attrs.clone();
    let field_attrs = FieldAttr::from_attrs(&mut attrs)?;

    // The expectation is that when an attribute is processed it will be removed so if any are left over we know they are invalid
    // but we only throw errors for Specta-specific attributes so we don't continually break other attributes.
    if let Some(attr) = attrs.iter().find(|attr| attr.source == "specta") {
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
                if let Some(inner_attr) = inner_attrs.first() {
                    return Err(syn::Error::new(
                        inner_attr.key.span(),
                        format!(
                            "specta: Found unsupported field attribute '{}'",
                            inner_attr.key
                        ),
                    ));
                }
                return Err(syn::Error::new(
                    attr.key.span(),
                    "specta: invalid formatted attribute",
                ));
            }
        }
    }

    Ok((field_attrs, &field.attrs))
}

pub fn parse_struct(
    container_attrs: &ContainerAttr,
    data: &DataStruct,
) -> syn::Result<(TokenStream, TokenStream)> {
    if container_attrs.transparent {
        if let Fields::Unit = data.fields {
            return Err(syn::Error::new(
                data.fields.span(),
                "specta: unit structs cannot be transparent",
            ));
        }

        let fields = data
            .fields
            .iter()
            .map(|field| {
                decode_field_attrs(field, &container_attrs.skip_attrs)
                    .map(|(attrs, raw)| (field.ty.clone(), attrs, raw))
            })
            .collect::<syn::Result<Vec<(Type, FieldAttr, &[syn::Attribute])>>>()?
            .into_iter()
            .filter(|(_, attrs, _)| !attrs.skip)
            .collect::<Vec<_>>();

        if fields.len() != 1 {
            return Err(syn::Error::new(
                data.fields.span(),
                "specta: transparent structs must have exactly one field",
            ));
        }

        let (field_ty, field_attrs, raw_attrs) =
            fields.into_iter().next().expect("fields.len() != 1");

        let field = construct_field(container_attrs, field_attrs, &field_ty, raw_attrs)?;

        return Ok((
            quote!(Struct),
            quote!(
                let mut e = datatype::Struct::unit();
                *e.fields_mut() = internal::construct::fields_unnamed(vec![#field], vec![]);
            ),
        ));
    }

    let fields = match &data.fields {
        Fields::Named(_) => {
            let fields = data
                .fields
                .iter()
                .map(|field| {
                    let (field_attrs, raw_attrs) =
                        decode_field_attrs(field, &container_attrs.skip_attrs)?;

                    let field_ident_str =
                        unraw_raw_ident(field.ident.as_ref().ok_or_else(|| {
                            syn::Error::new(
                                field.span(),
                                "specta: named field must have an identifier",
                            )
                        })?);
                    let field_name = field_ident_str.to_token_stream();

                    let inner =
                        construct_field(container_attrs, field_attrs, &field.ty, raw_attrs)?;
                    Ok(quote!((#field_name.into(), #inner)))
                })
                .collect::<syn::Result<Vec<TokenStream>>>()?;

            quote!(internal::construct::fields_named(
                vec![#(#fields),*],
                vec![]
            ))
        }
        Fields::Unnamed(_) => {
            let fields = data
                .fields
                .iter()
                .map(|field| {
                    let (field_attrs, raw_attrs) =
                        decode_field_attrs(field, &container_attrs.skip_attrs)?;
                    construct_field(container_attrs, field_attrs, &field.ty, raw_attrs)
                })
                .collect::<syn::Result<Vec<TokenStream>>>()?;

            quote!(internal::construct::fields_unnamed(
                vec![#(#fields),*],
                vec![]
            ))
        }
        Fields::Unit => quote!(datatype::Fields::Unit),
    };

    Ok((
        quote!(Struct),
        quote!(
            let mut e = datatype::Struct::unit();
            *e.fields_mut() = #fields;
        ),
    ))
}
