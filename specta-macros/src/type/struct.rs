use crate::{
    r#type::field::construct_field,
    utils::{parse_attrs, unraw_raw_ident, AttributeValue},
};
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
    let definition_generics = generics.params.iter().filter_map(|p| match p {
        GenericParam::Type(t) => {
            let ident = t.ident.to_string();
            Some(quote!(std::borrow::Cow::Borrowed(#ident).into()))
        }
        _ => None,
    });

    // todo!("{:?}", container_attrs.transparent);
    let definition = if container_attrs.transparent {
        if let Fields::Unit = data.fields {
            return Err(syn::Error::new(
                data.fields.span(),
                "specta: unit structs cannot be transparent",
            ));
        }

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

        let (field_ty, field_attrs) = fields.into_iter().next().expect("fields.len() != 1");
        let field_ty = field_attrs.r#type.as_ref().unwrap_or(&field_ty);

        // TODO: Should we check container too?
        // if container_attrs.inline || field_attrs.inline {
        //     // TODO: Duplicate of code in `field.rs` we should refactor out into helper.
        //     // let generics = generics.params.iter().filter_map(|p| match p {
        //     //     GenericParam::Const(..) | GenericParam::Lifetime(..) => None,
        //     //     GenericParam::Type(p) => {
        //     //         let ident = &p.ident;
        //     //         let ident_str = p.ident.to_string();

        //     //         quote!((std::borrow::Cow::Borrowed(#ident_str).into(), <#ident as #crate_ref::Type>::definition(types))).into()
        //     //     }
        //     // });

        //     // quote!(#crate_ref::datatype::inline::<#field_ty>(types))
        //     todo!();
        // } else {
        //     quote!(<#field_ty as #crate_ref::Type>::definition(types))
        // }

        // TODO: How can we passthrough the inline to this reference?
        quote!(<#field_ty as #crate_ref::Type>::definition(types))
    } else {
        let fields = match &data.fields {
            Fields::Named(_) => {
                let fields = data
                    .fields
                    .iter()
                    .map(|field| {
                        let field_attrs = decode_field_attrs(field)?;

                        let field_ident_str = unraw_raw_ident(field.ident.as_ref().unwrap());
                        let field_name =
                            match (field_attrs.rename.clone(), container_attrs.rename_all) {
                                (Some(name), _) => name,
                                (_, Some(inflection)) => {
                                    inflection.apply(&field_ident_str).to_token_stream()
                                }
                                (_, _) => field_ident_str.to_token_stream(),
                            };

                        let inner =
                            construct_field(crate_ref, container_attrs, field_attrs, &field.ty);
                        Ok(quote!((#field_name.into(), #inner)))
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
                        Ok(construct_field(
                            crate_ref,
                            container_attrs,
                            field_attrs,
                            &field.ty,
                        ))
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
