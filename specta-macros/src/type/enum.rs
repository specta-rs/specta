use super::{attr::*, lower_attr::lower_attribute, r#struct::decode_field_attrs};
use crate::{r#type::field::construct_field, utils::*};
use proc_macro2::{Span, TokenStream};
use quote::{ToTokens, quote};
use syn::{DataEnum, Error, Fields, spanned::Spanned};

pub fn parse_enum(
    enum_attrs: &EnumAttr,
    container_attrs: &ContainerAttr,
    data: &DataEnum,
    // lowered_attrs: &Vec<TokenStream>,
) -> syn::Result<(TokenStream, TokenStream)> {
    if container_attrs.transparent {
        return Err(syn::Error::new(
            data.enum_token.span(),
            "#[specta(transparent)] is not allowed on an enum",
        ));
    }

    let variant_types =
        data.variants
            .iter()
            .map(|v| {
                // We pass all the attributes at the start and when decoding them pop them off the list.
                // This means at the end we can check for any that weren't consumed and throw an error.
                let mut attrs = parse_attrs(&v.attrs)?;
                let variant_attrs = VariantAttr::from_attrs(&mut attrs)?;

                // The expectation is that when an attribute is processed it will be removed so if any are left over we know they are invalid
                // but we only throw errors for Specta-specific attributes so we don't continually break other attributes.
                if let Some(attr) = attrs.iter().find(|attr| attr.key == "specta") {
                    match &attr.value {
                        Some(AttributeValue::Attribute { attr: inner_attrs, .. }) => {
                            if let Some(inner_attr) = inner_attrs.first() {
                                return Err(syn::Error::new(
                                    inner_attr.key.span(),
                                    format!(
                                        "specta: Found unsupported variant attribute '{}'",
                                        inner_attr.key
                                    ),
                                ));
                            }
                        }
                        _ => {
                            return Err(syn::Error::new(
                                attr.key.span(),
                                "specta: invalid formatted attribute",
                            ));
                        }
                    }
                }

                // Lower the variant attributes to RuntimeAttribute tokens
                let lowered_variant_attrs = v.attrs
                    .iter()
                    .filter(|attr| {
                        let path = attr.path().to_token_stream().to_string();
                        path == "serde" || path == "specta"
                    })
                    .map(|attr| lower_attribute(attr).map(|attr| attr.to_tokens()))
                    .collect::<Result<Vec<_>, _>>()?;

                Ok((v, variant_attrs, lowered_variant_attrs))
            })
            .collect::<syn::Result<Vec<_>>>()?
            .into_iter()
            .map(|(variant, attrs, lowered_variant_attrs)| {
                let variant_ident_str = unraw_raw_ident(&variant.ident);
                let variant_name_str = variant_ident_str.to_token_stream();

                let inner = match &variant.fields {
                    Fields::Unit => quote!(datatype::Fields::Unit),
                    Fields::Unnamed(fields) => {
                        let fields = fields
                            .unnamed
                            .iter()
                            .map(|field| {
                                let (field_attrs, raw_attrs) = decode_field_attrs(field)?;
                                Ok(construct_field( container_attrs, field_attrs, &field.ty, &raw_attrs))
                            })
                            .collect::<syn::Result<Vec<TokenStream>>>()?;

                        quote!(internal::construct::fields_unnamed(
                            vec![#(#fields),*],
                            vec![],
                        ))
                    }
                    Fields::Named(fields) => {
                        let fields = fields
                        .named
                        .iter()
                        .map(|field| {
                            let (field_attrs, raw_attrs) = decode_field_attrs(field)?;

                            let field_ident_str =
                                unraw_raw_ident(field.ident.as_ref().unwrap());
                            let field_name = field_ident_str;

                            let inner = construct_field(container_attrs, field_attrs, &field.ty, &raw_attrs);
                            Ok(quote!((#field_name.into(), #inner)))
                        })
                        .collect::<syn::Result<Vec<TokenStream>>>()?;

                        quote!(internal::construct::fields_named(vec![#(#fields),*], vec![]))
                    }
                };

                let deprecated = attrs.common.deprecated_as_tokens();
                let skip = attrs.skip;
                let doc = attrs.common.doc;
                Ok(quote!((#variant_name_str.into(), internal::construct::enum_variant(#skip, #deprecated, #doc.into(), #inner, vec![#(#lowered_variant_attrs),*]))))
            })
            .collect::<syn::Result<Vec<_>>>()?;

    let (can_flatten, repr) = {
        match (enum_attrs.untagged, &enum_attrs.tag, &enum_attrs.content) {
            (None, None, None) => (
                // TODO: We treat the default being externally tagged but that is a bad assumption.
                // Fix this with: https://github.com/specta-rs/specta/issues/384
                data.variants.iter().any(|v| match &v.fields {
                    Fields::Unnamed(f) if f.unnamed.len() == 1 => true,
                    Fields::Named(_) => true,
                    _ => false,
                }),
                quote!(None),
            ),
            (Some(false), None, None) => (
                data.variants.iter().any(|v| match &v.fields {
                    Fields::Unnamed(f) if f.unnamed.len() == 1 => true,
                    Fields::Named(_) => true,
                    _ => false,
                }),
                quote!(Some(datatype::EnumRepr::External)),
            ),
            (Some(false) | None, Some(tag), None) => (
                data.variants
                    .iter()
                    .any(|v| matches!(&v.fields, Fields::Unit | Fields::Named(_))),
                quote!(Some(datatype::EnumRepr::Internal { tag: #tag.into() })),
            ),
            (Some(false) | None, Some(tag), Some(content)) => (
                true,
                quote!(Some(datatype::EnumRepr::Adjacent { tag: #tag.into(), content: #content.into() })),
            ),
            (Some(true), None, None) => (
                data.variants
                    .iter()
                    .any(|v| matches!(&v.fields, Fields::Unit | Fields::Named(_))),
                quote!(Some(datatype::EnumRepr::Untagged)),
            ),
            (Some(true), Some(_), None) => {
                return Err(Error::new(
                    Span::call_site(),
                    "untagged cannot be used with tag",
                ));
            }
            (Some(true), _, Some(_)) => {
                return Err(Error::new(
                    Span::call_site(),
                    "untagged cannot be used with content",
                ));
            }
            (Some(false) | None, None, Some(_)) => {
                return Err(Error::new(
                    Span::call_site(),
                    "content cannot be used without tag",
                ));
            }
        }
    };

    Ok((
        quote!(Enum),
        quote!(*e.variants_mut() = vec![#(#variant_types),*];),
    ))
}
