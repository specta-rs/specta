use super::{
    AttributeScope, Names,
    attr::*,
    build_runtime_attributes,
    generics::{BuildKind, resolve_ty_expr, type_with_inferred_lifetimes},
    serde::AttributeTarget,
    r#struct::decode_field_attrs,
};
use crate::{r#type::field::construct_field_with_variant_skip, utils::*};
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::{DataEnum, Fields, spanned::Spanned};

/// Lowers an enum to the expression that builds its datatype.
///
/// `attrs_apply` is the container's runtime attributes, already targeting the
/// `Enum` value this function binds as `e`. Writing them there keeps the whole
/// body one level shallower than converting to a `DataType` first.
///
/// Also reports whether the expression names one of the container's type
/// parameters, so the caller knows whether the function holding it must be
/// generic.
#[allow(clippy::too_many_arguments)]
pub fn parse_enum(
    names: &Names,
    container_attrs: &ContainerAttr,
    data: &DataEnum,
    used_generic_types: &[syn::Ident],
    bounded_params: &[syn::Ident],
    attrs_apply: Option<&TokenStream>,
    build: BuildKind,
) -> syn::Result<(TokenStream, bool)> {
    if container_attrs.transparent {
        return Err(syn::Error::new(
            data.enum_token.span(),
            "#[specta(transparent)] is not allowed on an enum",
        ));
    }

    let mut uses_params = false;
    let variant_steps = data
        .variants
        .iter()
        .enumerate()
        .map(|(index, v)| {
            // Numbered so every variant lands in its own binding, which the
            // variant list below collects in one assignment.
            let binding = format_ident!("v{}", index);

            // We pass all the attributes at the start and when decoding them pop them off the list.
            // This means at the end we can check for any that weren't consumed and throw an error.
            let mut attrs = parse_attrs_with_filter(&v.attrs, &container_attrs.skip_attrs)?;
            let variant_attrs = VariantAttr::from_attrs(&mut attrs)?;

            reject_unknown_specta_attrs(&attrs, Scope::Variant)?;

            let runtime_attrs = build_runtime_attributes(
                names,
                AttributeScope::Variant,
                AttributeTarget::Statements(quote!(#binding.attributes)),
                &v.attrs,
                &container_attrs.skip_attrs,
            )?;

            Ok((v, variant_attrs, runtime_attrs, binding))
        })
        .collect::<syn::Result<Vec<_>>>()?
        .into_iter()
        .map(|(variant, attrs, runtime_attrs, binding)| {
            let variant_name_str = unraw_raw_ident(&variant.ident);

            let variant_value = if let Some(ref variant_ty) = attrs.r#type {
                let variant_ty = type_with_inferred_lifetimes(variant_ty);
                let resolved = resolve_ty_expr(
                    names,
                    &variant_ty,
                    used_generic_types,
                    bounded_params,
                    build,
                );
                uses_params |= resolved.uses_params;
                let dt = &names.dt;
                let resolve = resolved.tokens;
                quote!(#dt::Variant::unnamed().field(#resolve).build())
            } else {
                match &variant.fields {
                    Fields::Unit => {
                        let dt = &names.dt;
                        quote!(#dt::Variant::unit())
                    }
                    Fields::Unnamed(fields) => {
                        let fields = fields
                            .unnamed
                            .iter()
                            .enumerate()
                            .map(|(idx, field)| {
                                let (mut field_attrs, raw_attrs) =
                                    decode_field_attrs(field, &container_attrs.skip_attrs)?;

                                if attrs.inline && idx == 0 {
                                    field_attrs.inline = true;
                                }

                                let field = construct_field_with_variant_skip(
                                    names,
                                    container_attrs,
                                    field_attrs,
                                    &field.ty,
                                    raw_attrs,
                                    used_generic_types,
                                    bounded_params,
                                    attrs.skip,
                                    build,
                                )?;
                                uses_params |= field.uses_params;
                                Ok((None, field.value))
                            })
                            .collect::<syn::Result<Vec<_>>>()?;

                        let dt = &names.dt;
                        variant_builder(quote!(#dt::Variant::unnamed()), fields)
                    }
                    Fields::Named(fields) => {
                        let field_calls = fields
                            .named
                            .iter()
                            .map(|field| {
                                let (field_attrs, raw_attrs) =
                                    decode_field_attrs(field, &container_attrs.skip_attrs)?;

                                let field_name =
                                    unraw_raw_ident(field.ident.as_ref().ok_or_else(|| {
                                        syn::Error::new(
                                            field.span(),
                                            "specta: named field must have an identifier",
                                        )
                                    })?);

                                let field = construct_field_with_variant_skip(
                                    names,
                                    container_attrs,
                                    field_attrs,
                                    &field.ty,
                                    raw_attrs,
                                    used_generic_types,
                                    bounded_params,
                                    attrs.skip,
                                    build,
                                )?;
                                uses_params |= field.uses_params;
                                let name = field_name.to_token_stream();
                                Ok((Some(name), field.value))
                            })
                            .collect::<syn::Result<Vec<_>>>()?;

                        let dt = &names.dt;
                        variant_builder(quote!(#dt::Variant::named()), field_calls)
                    }
                }
            };

            let variant_skip = attrs.skip.then(|| quote!( #binding.skip = true;));

            let variant_docs = (!attrs.common.doc.is_empty()).then(|| {
                let docs = &attrs.common.doc;
                quote! {
                    #binding.docs = #docs.into();
                }
            });
            let field_deprecated = attrs.common.deprecated.map(|deprecated| {
                let tokens = deprecated_as_tokens(&names.dt, deprecated);
                quote!(#binding.deprecated = #tokens;)
            });

            let type_overridden_attribute = attrs
                .r#type
                .as_ref()
                .map(|_| quote!(#binding.attributes.insert("specta:type_override", true);));

            let variant_metadata = attrs.skip
                || attrs.r#type.is_some()
                || !attrs.common.doc.is_empty()
                || field_deprecated.is_some()
                || runtime_attrs.is_some();

            let steps = if !variant_metadata {
                quote! {
                    let #binding = #variant_value;
                }
            } else {
                quote! {
                    let mut #binding = #variant_value;
                    #variant_skip
                    #field_deprecated
                    #variant_docs
                    #runtime_attrs
                    #type_overridden_attribute
                }
            };

            Ok((steps, variant_name_str))
        })
        .collect::<syn::Result<Vec<(TokenStream, String)>>>()?;

    // The variants are built as statements and only then collected into the
    // list: building them inside the literal nests each one (and everything in
    // it) a level deeper, and the derives are printed back out that way.
    let dt = &names.dt;
    if variant_steps.is_empty() && attrs_apply.is_none() {
        return Ok((quote!(#dt::Enum::default().into()), uses_params));
    }

    let steps = variant_steps.iter().map(|(steps, _)| steps);
    let variants = variant_steps.iter().enumerate().map(|(index, (_, name))| {
        let binding = format_ident!("v{}", index);
        quote!((#name.into(), #binding))
    });

    Ok((
        quote!({
            let mut e = #dt::Enum::default();
            #(#steps)*
            e.variants = [#(#variants),*].into();
            #attrs_apply
            e.into()
        }),
        uses_params,
    ))
}

/// The expression that builds a variant's value.
///
/// `VariantBuilder::field` consumes the builder, so the fields have to be
/// chained; each field's own value is a self-contained expression.
fn variant_builder(
    constructor: TokenStream,
    fields: Vec<(Option<TokenStream>, TokenStream)>,
) -> TokenStream {
    if fields.is_empty() {
        return quote!(#constructor.build());
    }

    let named = fields.first().is_some_and(|(name, _)| name.is_some());
    let names = fields.iter().map(|(name, _)| name);
    let values = fields.iter().map(|(_, value)| value);

    if named {
        quote!(#constructor #( .field(#names, #values) )* .build())
    } else {
        quote!(#constructor #( .field(#values) )* .build())
    }
}
