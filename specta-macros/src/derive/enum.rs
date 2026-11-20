use super::{
    Names,
    attr::*,
    field::construct_field_with_variant_skip,
    generics::{BuildKind, resolve_ty_expr, type_with_inferred_lifetimes},
    r#struct::VariantInfo,
};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DataEnum, Fields, spanned::Spanned};

use crate::derive::attr::parse::invisible;

/// Lowers an enum to the expression that builds its datatype.
///
/// `variants` are the decoded variant metadata, parallel to `data.variants`.
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
    variants: &[VariantInfo],
    used_generic_types: &[&syn::Ident],
    bounded_params: &[&syn::Ident],
    attrs_apply: Option<&TokenStream>,
    build: BuildKind,
) -> syn::Result<(TokenStream, bool)> {
    crate::phase!("enum:build");

    if container_attrs.transparent {
        return Err(syn::Error::new(
            data.enum_token.span(),
            "#[specta(transparent)] is not allowed on an enum",
        ));
    }

    let mut uses_params = false;
    let mut variant_steps = Vec::with_capacity(variants.len());

    for (variant, info) in data.variants.iter().zip(variants) {
        crate::phase!("variant:build");

        let attrs = &info.attrs;
        let binding = &info.binding;

        // A skipped variant is not exported, so resolving its override would add
        // an unnecessary `Type` bound to the container. It still keeps the Rust
        // field layout with untyped fields, because Serde may serialize that
        // payload even when Specta hides it, and specta-serde reads the layout to
        // recognise the wire shape.
        let variant_value = if attrs.skip {
            let dt = &names.dt;
            let calls = info
                .fields
                .iter()
                .map(|field| (field.name.as_deref(), quote!(#dt::Field::default())))
                .collect::<Vec<_>>();
            // A unit variant is already a value; the other shapes need the
            // builder to be finalised.
            let variant = match &variant.fields {
                Fields::Unit => quote!(#dt::Variant::unit()),
                Fields::Unnamed(_) => variant_builder(quote!(#dt::Variant::unnamed()), calls),
                Fields::Named(_) => variant_builder(quote!(#dt::Variant::named()), calls),
            };
            invisible(variant)
        } else if let Some(ref variant_ty) = attrs.r#type {
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
            invisible(quote!(#dt::Variant::unnamed().field(#resolve).build()))
        } else {
            match &variant.fields {
                Fields::Unit => {
                    let dt = &names.dt;
                    invisible(quote!(#dt::Variant::unit()))
                }
                Fields::Unnamed(fields) => {
                    let mut calls = Vec::with_capacity(fields.unnamed.len());

                    for (field, field_info) in fields.unnamed.iter().zip(&info.fields) {
                        let field = construct_field_with_variant_skip(
                            names,
                            field_info,
                            &field.ty,
                            used_generic_types,
                            bounded_params,
                            attrs.skip,
                            build,
                        )?;
                        uses_params |= field.uses_params;
                        calls.push((None, field.value));
                    }

                    let dt = &names.dt;
                    invisible(variant_builder(quote!(#dt::Variant::unnamed()), calls))
                }
                Fields::Named(fields) => {
                    let mut calls = Vec::with_capacity(fields.named.len());

                    for (field, field_info) in fields.named.iter().zip(&info.fields) {
                        let field_name = field_info.name.as_deref().ok_or_else(|| {
                            syn::Error::new(
                                field.span(),
                                "specta: named field must have an identifier",
                            )
                        })?;

                        let field = construct_field_with_variant_skip(
                            names,
                            field_info,
                            &field.ty,
                            used_generic_types,
                            bounded_params,
                            attrs.skip,
                            build,
                        )?;
                        uses_params |= field.uses_params;
                        calls.push((Some(field_name), field.value));
                    }

                    let dt = &names.dt;
                    invisible(variant_builder(quote!(#dt::Variant::named()), calls))
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
        let field_deprecated = attrs.common.deprecated.as_ref().map(|deprecated| {
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
            || info.runtime_attrs.is_some();

        let runtime_attrs = info.runtime_attrs.as_ref();
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

        variant_steps.push(invisible(steps));
    }

    // The variants are built as statements and only then collected into the
    // list: building them inside the literal nests each one (and everything in
    // it) a level deeper, and the derives are printed back out that way.
    let dt = &names.dt;
    if variant_steps.is_empty() && attrs_apply.is_none() {
        return Ok((quote!(#dt::Enum::default().into()), uses_params));
    }

    let steps = variant_steps.iter();
    let variants = variant_steps.iter().zip(variants).map(|(_, info)| {
        let name = &info.name;
        let binding = &info.binding;
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
    fields: Vec<(Option<&str>, TokenStream)>,
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
