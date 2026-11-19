use crate::{
    r#type::field::{FieldTokens, construct_field},
    utils::{parse_attrs_with_filter, unraw_raw_ident},
};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DataStruct, Field, Fields, Type, spanned::Spanned};

use super::{Names, attr::*};

pub fn decode_field_attrs<'a>(
    field: &'a Field,
    skip_attrs: &[String],
) -> syn::Result<(FieldAttr, &'a [syn::Attribute])> {
    // We pass all the attributes at the start and when decoding them pop them off the list.
    // This means at the end we can check for any that weren't consumed and throw an error.
    let mut attrs = parse_attrs_with_filter(&field.attrs, skip_attrs)?;
    let field_attrs = FieldAttr::from_attrs(&mut attrs)?;

    reject_unknown_specta_attrs(&attrs, Scope::Field)?;

    Ok((field_attrs, &field.attrs))
}

pub fn parse_struct(
    names: &Names,
    container_attrs: &ContainerAttr,
    data: &DataStruct,
    used_generic_types: &[syn::Ident],
    bounded_params: &[syn::Ident],
) -> syn::Result<TokenStream> {
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

        let FieldTokens { prelude, value } = construct_field(
            names,
            container_attrs,
            field_attrs,
            &field_ty,
            raw_attrs,
            used_generic_types,
            bounded_params,
            &format_ident!("field"),
        )?;
        let dt = &names.dt;
        return Ok(chained(
            quote!(#dt::Struct::unnamed()),
            vec![(prelude, value)],
        ));
    }

    let dt = &names.dt;

    let fields = match &data.fields {
        Fields::Named(_) => {
            let fields = data
                .fields
                .iter()
                .enumerate()
                .map(|(index, field)| {
                    let (field_attrs, raw_attrs) =
                        decode_field_attrs(field, &container_attrs.skip_attrs)?;

                    let field_name = unraw_raw_ident(field.ident.as_ref().ok_or_else(|| {
                        syn::Error::new(field.span(), "specta: named field must have an identifier")
                    })?);

                    let FieldTokens { prelude, value } = construct_field(
                        names,
                        container_attrs,
                        field_attrs,
                        &field.ty,
                        raw_attrs,
                        used_generic_types,
                        bounded_params,
                        &format_ident!("field{}", index),
                    )?;
                    Ok((prelude, quote!(#field_name, #value)))
                })
                .collect::<syn::Result<Vec<(TokenStream, TokenStream)>>>()?;

            chained(quote!(#dt::Struct::named()), fields)
        }
        Fields::Unnamed(_) => {
            let is_newtype = data.fields.len() == 1;
            let fields = data
                .fields
                .iter()
                .enumerate()
                .map(|(index, field)| {
                    let (mut field_attrs, raw_attrs) =
                        decode_field_attrs(field, &container_attrs.skip_attrs)?;
                    if is_newtype && field_attrs.serde_skip {
                        // Serde ignores `#[serde(skip)]` on the sole field of a
                        // newtype struct and serializes the bare inner value.
                        field_attrs.skip = false;
                        field_attrs.serde_newtype_skip_ignored = true;
                    }
                    construct_field(
                        names,
                        container_attrs,
                        field_attrs,
                        &field.ty,
                        raw_attrs,
                        used_generic_types,
                        bounded_params,
                        &format_ident!("field{}", index),
                    )
                    .map(|FieldTokens { prelude, value }| (prelude, value))
                })
                .collect::<syn::Result<Vec<(TokenStream, TokenStream)>>>()?;

            chained(quote!(#dt::Struct::unnamed()), fields)
        }
        Fields::Unit => quote!(#dt::Struct::unit().into()),
    };

    Ok(fields)
}

/// A struct built by chaining its fields onto the constructor.
///
/// The statements that prepare a field are hoisted ahead of the whole chain
/// rather than inlined into its argument positions, and each field gets its own
/// binding, so one field's metadata statements never nest inside another's
/// arguments.
fn chained(constructor: TokenStream, fields: Vec<(TokenStream, TokenStream)>) -> TokenStream {
    let calls = fields.iter().map(|(_, args)| quote!(.field(#args)));

    if fields.iter().all(|(prelude, _)| prelude.is_empty()) {
        return quote!(#constructor #(#calls)* .build());
    }

    let preludes = fields.iter().map(|(prelude, _)| prelude);
    quote! {{
        #(#preludes)*
        #constructor #(#calls)* .build()
    }}
}
