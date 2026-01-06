use super::{attr::*, lower_attr::lower_attribute, r#struct::decode_field_attrs};
use crate::{r#type::field::construct_field, utils::*};
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{DataEnum, Fields, spanned::Spanned};

pub fn parse_enum(
    container_attrs: &ContainerAttr,
    data: &DataEnum,
) -> syn::Result<(TokenStream, TokenStream)> {
    if container_attrs.transparent {
        return Err(syn::Error::new(
            data.enum_token.span(),
            "#[specta(transparent)] is not allowed on an enum",
        ));
    }

    let variant_types = data
        .variants
        .iter()
        .map(|v| {
            // We pass all the attributes at the start and when decoding them pop them off the list.
            // This means at the end we can check for any that weren't consumed and throw an error.
            let mut attrs = parse_attrs_with_filter(&v.attrs, &container_attrs.skip_attrs)?;
            let variant_attrs = VariantAttr::from_attrs(&mut attrs)?;

            // The expectation is that when an attribute is processed it will be removed so if any are left over we know they are invalid
            // but we only throw errors for Specta-specific attributes so we don't continually break other attributes.
            if let Some(attr) = attrs.iter().find(|attr| attr.key == "specta") {
                match &attr.value {
                    Some(AttributeValue::Attribute {
                        attr: inner_attrs, ..
                    }) => {
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
            let lowered_variant_attrs = v
                .attrs
                .iter()
                .filter(|attr| {
                    let path = attr.path().to_token_stream().to_string();
                    // Skip attributes that are in the skip_attrs list
                    if container_attrs.skip_attrs.contains(&path) {
                        return false;
                    }
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
                            let (field_attrs, raw_attrs) =
                                decode_field_attrs(field, &container_attrs.skip_attrs)?;
                            construct_field(container_attrs, field_attrs, &field.ty, raw_attrs)
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
                            let (field_attrs, raw_attrs) =
                                decode_field_attrs(field, &container_attrs.skip_attrs)?;

                            let field_ident_str =
                                unraw_raw_ident(field.ident.as_ref().ok_or_else(|| {
                                    syn::Error::new(
                                        field.span(),
                                        "specta: named field must have an identifier",
                                    )
                                })?);

                            let field_name = field_ident_str;

                            let inner = construct_field(
                                container_attrs,
                                field_attrs,
                                &field.ty,
                                raw_attrs,
                            )?;
                            Ok(quote!((#field_name.into(), #inner)))
                        })
                        .collect::<syn::Result<Vec<TokenStream>>>()?;

                    quote!(internal::construct::fields_named(
                        vec![#(#fields),*],
                        vec![]
                    ))
                }
            };

            let deprecated = attrs.common.deprecated_as_tokens();
            let skip = attrs.skip;
            let doc = attrs.common.doc;
            Ok(quote!((#variant_name_str.into(), {
                let mut v = datatype::EnumVariant::unit();
                v.set_skip(#skip);
                v.set_deprecated(#deprecated);
                v.set_docs(#doc.into());
                v.set_fields(#inner);
                v.set_attributes(vec![#(#lowered_variant_attrs),*]);
                v
            })))
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok((
        quote!(Enum),
        quote!(*e.variants_mut() = vec![#(#variant_types),*];),
    ))
}
