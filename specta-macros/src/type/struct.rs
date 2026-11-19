use crate::{
    r#type::field::{FieldTokens, construct_field},
    utils::{parse_attrs_with_filter, unraw_raw_ident},
};
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{DataStruct, Field, Fields, Type, spanned::Spanned};

use super::{Names, attr::*, generics::BuildKind};

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

/// Lowers a struct to the expression that builds its datatype.
///
/// Also reports whether that expression names one of the container's type
/// parameters, so the caller knows whether the function holding it must be
/// generic.
pub fn parse_struct(
    names: &Names,
    container_attrs: &ContainerAttr,
    data: &DataStruct,
    used_generic_types: &[syn::Ident],
    bounded_params: &[syn::Ident],
    build: BuildKind,
) -> syn::Result<(TokenStream, bool)> {
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

        let FieldTokens { value, uses_params } = construct_field(
            names,
            container_attrs,
            field_attrs,
            &field_ty,
            raw_attrs,
            used_generic_types,
            bounded_params,
            build,
        )?;
        let dt = &names.dt;
        return Ok((
            chained(quote!(#dt::Struct::unnamed()), vec![(None, value)]),
            uses_params,
        ));
    }

    let dt = &names.dt;
    let mut uses_params = false;

    let fields = match &data.fields {
        Fields::Named(_) => {
            let fields = data
                .fields
                .iter()
                .map(|field| {
                    let (field_attrs, raw_attrs) =
                        decode_field_attrs(field, &container_attrs.skip_attrs)?;

                    let field_name = unraw_raw_ident(field.ident.as_ref().ok_or_else(|| {
                        syn::Error::new(field.span(), "specta: named field must have an identifier")
                    })?);

                    let field = construct_field(
                        names,
                        container_attrs,
                        field_attrs,
                        &field.ty,
                        raw_attrs,
                        used_generic_types,
                        bounded_params,
                        build,
                    )?;
                    uses_params |= field.uses_params;
                    let name = field_name.to_token_stream();
                    Ok((Some(name), field.value))
                })
                .collect::<syn::Result<Vec<_>>>()?;

            chained(quote!(#dt::Struct::named()), fields)
        }
        Fields::Unnamed(_) => {
            let is_newtype = data.fields.len() == 1;
            let fields = data
                .fields
                .iter()
                .map(|field| {
                    let (mut field_attrs, raw_attrs) =
                        decode_field_attrs(field, &container_attrs.skip_attrs)?;
                    if is_newtype && field_attrs.serde_skip {
                        // Serde ignores `#[serde(skip)]` on the sole field of a
                        // newtype struct and serializes the bare inner value.
                        field_attrs.skip = false;
                        field_attrs.serde_newtype_skip_ignored = true;
                    }
                    let FieldTokens {
                        value,
                        uses_params: field_uses_params,
                    } = construct_field(
                        names,
                        container_attrs,
                        field_attrs,
                        &field.ty,
                        raw_attrs,
                        used_generic_types,
                        bounded_params,
                        build,
                    )?;
                    uses_params |= field_uses_params;
                    Ok((None, value))
                })
                .collect::<syn::Result<Vec<_>>>()?;

            chained(quote!(#dt::Struct::unnamed()), fields)
        }
        Fields::Unit => quote!(#dt::Struct::unit().into()),
    };

    Ok((fields, uses_params))
}

/// A struct built by chaining its fields onto the constructor.
///
/// The names and values are interpolated directly into the chain template: a
/// separate `quote!` per `.field(..)` call would pay its fixed cost once per
/// field for nothing.
fn chained(
    constructor: TokenStream,
    fields: Vec<(Option<TokenStream>, TokenStream)>,
) -> TokenStream {
    let named = fields.first().is_some_and(|(name, _)| name.is_some());
    let names = fields.iter().map(|(name, _)| name);
    let values = fields.iter().map(|(_, value)| value);

    if named {
        quote!(#constructor #( .field(#names, #values) )* .build())
    } else {
        quote!(#constructor #( .field(#values) )* .build())
    }
}
