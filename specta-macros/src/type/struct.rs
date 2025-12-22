use crate::{
    r#type::field::construct_field,
    utils::{AttributeValue, parse_attrs, unraw_raw_ident},
};
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{DataStruct, Field, Fields, Type, spanned::Spanned};

use super::attr::*;

pub fn decode_field_attrs(field: &Field) -> syn::Result<(FieldAttr, Vec<crate::utils::Attribute>)> {
    // We pass all the attributes at the start and when decoding them pop them off the list.
    // This means at the end we can check for any that weren't consumed and throw an error.
    let raw_attrs = parse_attrs(&field.attrs)?;
    let mut attrs = raw_attrs.clone();
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
                ));
            }
        }
    }

    Ok((field_attrs, raw_attrs))
}

pub fn parse_struct(
    container_attrs: &ContainerAttr,
    crate_ref: &TokenStream,
    data: &DataStruct,
    lowered_attrs: &Vec<TokenStream>, // TODO: Make more typesafe
) -> syn::Result<(TokenStream, bool)> {
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
            .map(|field| {
                decode_field_attrs(field).map(|(attrs, raw)| (field.ty.clone(), attrs, raw))
            })
            .collect::<syn::Result<Vec<(Type, FieldAttr, Vec<crate::utils::Attribute>)>>>()?
            .into_iter()
            .filter(|(_, attrs, _)| !attrs.skip)
            .collect::<Vec<_>>();

        if fields.len() != 1 {
            return Err(syn::Error::new(
                data.fields.span(),
                "specta: transparent structs must have exactly one field",
            ));
        }

        let (field_ty, field_attrs, _raw_attrs) =
            fields.into_iter().next().expect("fields.len() != 1");
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

        //     // quote!(datatype::inline::<#field_ty>(types))
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
                        let (field_attrs, raw_attrs) = decode_field_attrs(field)?;

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
                            construct_field(container_attrs, field_attrs, &field.ty, &raw_attrs);
                        Ok(quote!((#field_name.into(), #inner)))
                    })
                    .collect::<syn::Result<Vec<TokenStream>>>()?;

                quote!(internal::construct::fields_named(vec![#(#fields),*], vec![]))
            }
            Fields::Unnamed(_) => {
                let fields = data
                    .fields
                    .iter()
                    .map(|field| {
                        let (field_attrs, raw_attrs) = decode_field_attrs(field)?;
                        Ok(construct_field(
                            container_attrs,
                            field_attrs,
                            &field.ty,
                            &raw_attrs,
                        ))
                    })
                    .collect::<syn::Result<Vec<TokenStream>>>()?;

                quote!(internal::construct::fields_unnamed(vec![#(#fields),*], vec![]))
            }
            Fields::Unit => quote!(datatype::Fields::Unit),
        };

        quote!(datatype::DataType::Struct(internal::construct::r#struct(#fields, vec![#(#lowered_attrs),*])))
    };

    Ok((definition, true))
}
