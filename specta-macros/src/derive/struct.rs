use crate::derive::field::{FieldTokens, construct_field, decode_field_attrs};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DataStruct, Fields, spanned::Spanned};

use super::{
    Names, attr::*, build_runtime_attributes, generics::BuildKind, serde::AttributeTarget,
};

/// One field's metadata, decoded once for the whole derive.
///
/// A container with used type parameters builds its datatype twice — once per
/// concrete instantiation and once as the canonical declaration — and the scan
/// that finds those parameters also has to read `#[specta(skip)]` and
/// `#[specta(type = ...)]`. None of that depends on which build is being emitted,
/// so decoding it in each pass made an attribute-heavy generic container parse
/// the same attributes three times. The lowered `serde:*` metadata is included
/// for the same reason: [`build_runtime_attributes`] resolves the types written
/// in the attributes, which the placeholder rewrite never touches, so both builds
/// emit the identical tokens.
pub(crate) struct FieldInfo {
    pub attrs: FieldAttr,
    pub runtime_attrs: Option<TokenStream>,
    /// The exported name; `None` for a tuple field.
    pub name: Option<String>,
}

/// One variant's metadata, decoded once for the whole derive. See [`FieldInfo`].
pub(crate) struct VariantInfo {
    pub attrs: VariantAttr,
    pub runtime_attrs: Option<TokenStream>,
    pub name: String,
    /// The identifier the variant's value is bound to in the generated body,
    /// numbered so every variant lands in its own binding.
    pub binding: syn::Ident,
    pub fields: Vec<FieldInfo>,
}

/// Every item of a container, with its attributes already decoded.
pub(crate) enum ContainerItems {
    Struct(Vec<FieldInfo>),
    Enum(Vec<VariantInfo>),
    Union,
}

/// Decodes the attributes of every field and variant of `data` exactly once.
pub(crate) fn decode_items(
    names: &Names,
    container_attrs: &ContainerAttr,
    data: &syn::Data,
) -> syn::Result<ContainerItems> {
    match data {
        syn::Data::Struct(data) => {
            let mut fields = decode_fields(names, container_attrs, &data.fields)?;

            // Serde ignores `#[serde(skip)]` on the sole field of a newtype
            // struct and serializes the bare inner value.
            if matches!(data.fields, Fields::Unnamed(_)) && data.fields.len() == 1 {
                let field = &mut fields[0].attrs;
                if field.serde_skip {
                    field.skip = false;
                    field.serde_newtype_skip_ignored = true;
                }
            }

            Ok(ContainerItems::Struct(fields))
        }
        syn::Data::Enum(data) => {
            let mut variants = Vec::with_capacity(data.variants.len());

            for (index, variant) in data.variants.iter().enumerate() {
                let binding = quote::format_ident!("v{}", index);

                let mut attrs =
                    parse_attrs_with_filter(&variant.attrs, &container_attrs.skip_attrs)?;
                let variant_attrs = VariantAttr::from_attrs(&mut attrs)?;
                reject_unknown_specta_attrs(&attrs, Scope::Variant)?;

                let runtime_attrs = build_runtime_attributes(
                    names,
                    Scope::Variant,
                    AttributeTarget::Statements(quote!(#binding.attributes)),
                    &variant.attrs,
                    &container_attrs.skip_attrs,
                    None,
                )?;

                let mut fields = decode_fields(names, container_attrs, &variant.fields)?;
                // `#[specta(inline)]` on a variant inlines its first field.
                if variant_attrs.inline
                    && let Some(field) = fields.first_mut()
                {
                    field.attrs.inline = true;
                }

                variants.push(VariantInfo {
                    attrs: variant_attrs,
                    runtime_attrs,
                    name: unraw_raw_ident(&variant.ident),
                    binding,
                    fields,
                });
            }

            Ok(ContainerItems::Enum(variants))
        }
        syn::Data::Union(_) => Ok(ContainerItems::Union),
    }
}

fn decode_fields(
    names: &Names,
    container_attrs: &ContainerAttr,
    fields: &Fields,
) -> syn::Result<Vec<FieldInfo>> {
    fields
        .iter()
        .map(|field| {
            let (attrs, raw_attrs) = decode_field_attrs(field, &container_attrs.skip_attrs)?;
            let name = field.ident.as_ref().map(unraw_raw_ident);
            let runtime_attrs = build_runtime_attributes(
                names,
                Scope::Field,
                AttributeTarget::Chain,
                raw_attrs,
                &container_attrs.skip_attrs,
                None,
            )?;

            Ok(FieldInfo {
                attrs,
                runtime_attrs,
                name,
            })
        })
        .collect()
}

/// Lowers a struct to the expression that builds its datatype.
///
/// `fields` are the decoded field metadata, parallel to `data.fields`.
///
/// Also reports whether that expression names one of the container's type
/// parameters, so the caller knows whether the function holding it must be
/// generic.
pub(crate) fn parse_struct(
    names: &Names,
    container_attrs: &ContainerAttr,
    data: &DataStruct,
    fields: &[FieldInfo],
    used_generic_types: &[&syn::Ident],
    bounded_params: &[&syn::Ident],
    build: BuildKind,
) -> syn::Result<(TokenStream, bool)> {
    if container_attrs.transparent {
        if let Fields::Unit = data.fields {
            return Err(syn::Error::new(
                data.fields.span(),
                "specta: unit structs cannot be transparent",
            ));
        }

        let mut visible = data
            .fields
            .iter()
            .zip(fields)
            .filter(|(_, info)| !info.attrs.skip);

        let (field, info) = visible.next().ok_or_else(|| {
            syn::Error::new(
                data.fields.span(),
                "specta: transparent structs must have exactly one field",
            )
        })?;

        if visible.next().is_some() {
            return Err(syn::Error::new(
                data.fields.span(),
                "specta: transparent structs must have exactly one field",
            ));
        }

        let FieldTokens { value, uses_params } = construct_field(
            names,
            info,
            &field.ty,
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

    let items = match &data.fields {
        Fields::Named(_) => {
            let mut items = Vec::with_capacity(fields.len());

            for (field, info) in data.fields.iter().zip(fields) {
                let field_name = info.name.as_deref().ok_or_else(|| {
                    syn::Error::new(field.span(), "specta: named field must have an identifier")
                })?;

                let field = construct_field(
                    names,
                    info,
                    &field.ty,
                    used_generic_types,
                    bounded_params,
                    build,
                )?;
                uses_params |= field.uses_params;
                items.push((Some(field_name), field.value));
            }

            chained(quote!(#dt::Struct::named()), items)
        }
        Fields::Unnamed(_) => {
            let mut items = Vec::with_capacity(fields.len());

            for (field, info) in data.fields.iter().zip(fields) {
                let FieldTokens {
                    value,
                    uses_params: field_uses_params,
                } = construct_field(
                    names,
                    info,
                    &field.ty,
                    used_generic_types,
                    bounded_params,
                    build,
                )?;
                uses_params |= field_uses_params;
                items.push((None, value));
            }

            chained(quote!(#dt::Struct::unnamed()), items)
        }
        Fields::Unit => quote!(#dt::Struct::unit().into()),
    };

    Ok((items, uses_params))
}

/// A struct built by chaining its fields onto the constructor.
///
/// The names and values are interpolated directly into the chain template: a
/// separate `quote!` per `.field(..)` call would pay its fixed cost once per
/// field for nothing.
fn chained(constructor: TokenStream, fields: Vec<(Option<&str>, TokenStream)>) -> TokenStream {
    let named = fields.first().is_some_and(|(name, _)| name.is_some());
    let names = fields.iter().map(|(name, _)| name);
    let values = fields.iter().map(|(_, value)| value);

    if named {
        quote!(#constructor #( .field(#names, #values) )* .build())
    } else {
        quote!(#constructor #( .field(#values) )* .build())
    }
}
