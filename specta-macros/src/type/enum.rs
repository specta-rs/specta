use super::{attr::*, r#struct::decode_field_attrs};
use crate::{r#type::field::construct_field, utils::*};
use proc_macro2::TokenStream;
use quote::{quote, ToTokens};
use syn::{spanned::Spanned, DataEnum, Fields, GenericParam, Generics};

pub fn parse_enum(
    name: &TokenStream,
    enum_attrs: &EnumAttr,
    container_attrs: &ContainerAttr,
    generics: &Generics,
    crate_ref: &TokenStream,
    data: &DataEnum,
) -> syn::Result<(TokenStream, bool)> {
    if container_attrs.transparent {
        return Err(syn::Error::new(
            data.enum_token.span(),
            "#[specta(transparent)] is not allowed on an enum",
        ));
    }

    let definition_generics = generics
        .params
        .iter()
        .filter_map(|p| match p {
            GenericParam::Type(t) => {
                let ident = t.ident.to_string();
                Some(quote!(std::borrow::Cow::Borrowed(#ident).into()))
            },
            _ => None,
        });

    let repr = enum_attrs.tagged()?;
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
                if let Some(attrs) = attrs.iter().find(|attr| attr.key == "specta") {
                    match &attrs.value {
                        Some(AttributeValue::Attribute { attr, .. }) => {
                            if let Some(attr) = attr.first() {
                                return Err(syn::Error::new(
                                    attr.key.span(),
                                    format!(
                                        "specta: Found unsupported enum attribute '{}'",
                                        attr.key
                                    ),
                                ));
                            }
                        }
                        _ => todo!(),
                    }
                }

                Ok((v, variant_attrs))
            })
            .collect::<syn::Result<Vec<_>>>()?
            .into_iter()
            .map(|(variant, attrs)| {
                let variant_ident_str = unraw_raw_ident(&variant.ident);

                let variant_name_str = match (attrs.rename, container_attrs.rename_all) {
                    (Some(name), _) => name,
                    (_, Some(inflection)) => inflection.apply(&variant_ident_str).to_token_stream(),
                    (_, _) => variant_ident_str.to_token_stream(),
                };

                let inner = match &variant.fields {
                    Fields::Unit => quote!(#crate_ref::internal::construct::fields_unit()),
                    Fields::Unnamed(fields) => {
                        let fields = fields
                            .unnamed
                            .iter()
                            .map(|field| {
                                let field_attrs = decode_field_attrs(field)?;
                                Ok(construct_field(crate_ref, generics, FieldAttr {
                                    rename: field_attrs.rename,
                                    r#type: field_attrs.r#type,
                                    inline: field_attrs.inline || attrs.inline,
                                    skip: field_attrs.skip || attrs.skip,
                                    optional: field_attrs.optional,
                                    flatten: field_attrs.flatten,
                                    common: field_attrs.common,
                                }, &field.ty))
                            })
                            .collect::<syn::Result<Vec<TokenStream>>>()?;

                        quote!(#crate_ref::internal::construct::fields_unnamed(
                            vec![#(#fields),*],
                        ))
                    }
                    Fields::Named(fields) => {
                        let fields = fields
                        .named
                        .iter()
                        .map(|field| {
                            let field_attrs = decode_field_attrs(field)?;

                            let field_ident_str =
                                unraw_raw_ident(field.ident.as_ref().unwrap());

                            let field_name = match (field_attrs.rename.clone(), attrs.rename_all) {
                                (Some(name), _) => name,
                                (_, Some(inflection)) => {
                                    let name = inflection.apply(&field_ident_str);
                                    quote::quote!(#name)
                                }
                                (_, _) => quote::quote!(#field_ident_str),
                            };

                            let inner = construct_field(crate_ref, generics, FieldAttr {
                                rename: field_attrs.rename,
                                r#type: field_attrs.r#type,
                                inline: field_attrs.inline || attrs.inline,
                                skip: field_attrs.skip || attrs.skip,
                                optional: field_attrs.optional,
                                flatten: field_attrs.flatten,
                                common: field_attrs.common,
                            }, &field.ty);
                            Ok(quote!((#field_name.into(), #inner)))
                        })
                        .collect::<syn::Result<Vec<TokenStream>>>()?;

                        quote!(#crate_ref::internal::construct::fields_named(vec![#(#fields),*], None))
                    }
                };

                let deprecated = attrs.common.deprecated_as_tokens(crate_ref);
                let skip = attrs.skip;
                let doc = attrs.common.doc;
                Ok(quote!((#variant_name_str.into(), #crate_ref::internal::construct::enum_variant(#skip, #deprecated, #doc.into(), #inner))))
            })
            .collect::<syn::Result<Vec<_>>>()?;

    let (repr, can_flatten) = match repr {
        Tagged::Untagged => (
            quote!(#crate_ref::datatype::EnumRepr::Untagged),
            data.variants
                .iter()
                .any(|v| matches!(&v.fields, Fields::Unit | Fields::Named(_))),
        ),
        Tagged::Externally => (
            quote!(#crate_ref::datatype::EnumRepr::External),
            data.variants.iter().any(|v| match &v.fields {
                Fields::Unnamed(f) if f.unnamed.len() == 1 => true,
                Fields::Named(_) => true,
                _ => false,
            }),
        ),
        Tagged::Adjacently { tag, content } => (
            quote!(#crate_ref::datatype::EnumRepr::Adjacent { tag: #tag.into(), content: #content.into() }),
            true,
        ),
        Tagged::Internally { tag } => (
            quote!(#crate_ref::datatype::EnumRepr::Internal { tag: #tag.into() }),
            data.variants
                .iter()
                .any(|v| matches!(&v.fields, Fields::Unit | Fields::Named(_))),
        ),
    };

    let skip_bigint_checs = enum_attrs.unstable_skip_bigint_checks;

    Ok((
        quote!(#crate_ref::datatype::DataType::Enum(#crate_ref::internal::construct::r#enum(#name.into(), SID, #repr, #skip_bigint_checs, vec![#(#definition_generics),*], vec![#(#variant_types),*]))),
        can_flatten,
    ))
}
