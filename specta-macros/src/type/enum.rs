use super::{attr::*, r#struct::decode_field_attrs};
use crate::{r#type::field::construct_field_with_variant_skip, utils::*};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{spanned::Spanned, DataEnum, Fields};

pub fn parse_enum(
    crate_ref: &TokenStream,
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
            if let Some(attr) = attrs.iter().find(|attr| attr.source == "specta") {
                match &attr.value {
                    None
                    | Some(AttributeValue::Lit(_))
                    | Some(AttributeValue::Path(_))
                    | Some(AttributeValue::Expr(_)) => {
                        return Err(syn::Error::new(
                            attr.key.span(),
                            "specta: invalid formatted attribute",
                        ));
                    }
                    Some(AttributeValue::Attribute {
                        attr: inner_attrs, ..
                    }) => {
                        if let Some(inner_attr) = inner_attrs.first() {
                            if let Some(message) =
                                migration_hint(Scope::Variant, &inner_attr.key.to_string())
                            {
                                return Err(syn::Error::new(inner_attr.key.span(), message));
                            }

                            return Err(syn::Error::new(
                                inner_attr.key.span(),
                                format!(
                                    "specta: Found unsupported variant attribute '{}'",
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

            Ok((v, variant_attrs))
        })
        .collect::<syn::Result<Vec<_>>>()?
        .into_iter()
        .map(|(variant, attrs)| {
            let variant_ident_str = unraw_raw_ident(&variant.ident);
            let variant_name_str = variant_ident_str.to_token_stream();
            let variant_skip = attrs.skip;
            let variant_inline = attrs.inline;

            let inner = match &variant.fields {
                Fields::Unit => quote!(datatype::Fields::Unit),
                Fields::Unnamed(fields) => {
                    let fields = fields
                        .unnamed
                        .iter()
                        .enumerate()
                        .map(|(idx, field)| {
                            let (mut field_attrs, raw_attrs) =
                                decode_field_attrs(field, &container_attrs.skip_attrs)?;

                            if variant_inline && idx == 0 {
                                field_attrs.inline = true;
                            }

                            construct_field_with_variant_skip(
                                crate_ref,
                                container_attrs,
                                field_attrs,
                                &field.ty,
                                raw_attrs,
                                variant_skip,
                            )
                        })
                        .collect::<syn::Result<Vec<TokenStream>>>()?;

                    quote!(internal::construct::fields_unnamed(
                        vec![#(#fields),*],
                        datatype::Attributes::default(),
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

                            let inner = construct_field_with_variant_skip(
                                crate_ref,
                                container_attrs,
                                field_attrs,
                                &field.ty,
                                raw_attrs,
                                variant_skip,
                            )?;
                            Ok(quote!((#field_name.into(), #inner)))
                        })
                        .collect::<syn::Result<Vec<TokenStream>>>()?;

                    quote!(internal::construct::fields_named(
                        vec![#(#fields),*],
                        datatype::Attributes::default()
                    ))
                }
            };

            let deprecated = attrs.common.deprecated_as_tokens();
            let skip = variant_skip;
            let doc = attrs.common.doc;
            Ok(quote!((#variant_name_str.into(), {
                let mut v = datatype::EnumVariant::unit();
                v.set_skip(#skip);
                v.set_deprecated(#deprecated);
                v.set_docs(#doc.into());
                v.set_fields(#inner);
                v.set_attributes(datatype::Attributes::default());
                v
            })))
        })
        .collect::<syn::Result<Vec<_>>>()?;

    Ok((
        quote!(Enum),
        quote!(
            let mut e = datatype::Enum::new();
            *e.variants_mut() = vec![#(#variant_types),*];
        ),
    ))
}
