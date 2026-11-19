use super::{
    AttributeScope, Names,
    attr::*,
    build_runtime_attributes,
    generics::{resolve_ty_expr, type_with_inferred_lifetimes},
    r#struct::decode_field_attrs,
};
use crate::{
    r#type::field::{FieldTokens, construct_field_with_variant_skip},
    utils::*,
};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DataEnum, Fields, spanned::Spanned};

/// Lowers an enum to the expression that builds its datatype.
///
/// `attrs_apply` is the container's runtime attributes, already targeting the
/// `Enum` value this function binds as `e`. Writing them there keeps the whole
/// body one level shallower than converting to a `DataType` first.
pub fn parse_enum(
    names: &Names,
    container_attrs: &ContainerAttr,
    data: &DataEnum,
    used_generic_types: &[syn::Ident],
    bounded_params: &[syn::Ident],
    attrs_apply: Option<&TokenStream>,
) -> syn::Result<TokenStream> {
    if container_attrs.transparent {
        return Err(syn::Error::new(
            data.enum_token.span(),
            "#[specta(transparent)] is not allowed on an enum",
        ));
    }

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
                quote!(#binding.attributes),
                &v.attrs,
                &container_attrs.skip_attrs,
            )?;

            Ok((v, variant_attrs, runtime_attrs, binding))
        })
        .collect::<syn::Result<Vec<_>>>()?
        .into_iter()
        .map(|(variant, attrs, runtime_attrs, binding)| {
            let variant_name_str = unraw_raw_ident(&variant.ident);

            // The builder statements are emitted ahead of the variant value so
            // the fields do not nest one level deeper inside its argument.
            let (variant_prelude, variant_value) = if let Some(ref variant_ty) = attrs.r#type {
                let variant_ty = type_with_inferred_lifetimes(variant_ty);
                let resolve =
                    resolve_ty_expr(names, &variant_ty, used_generic_types, bounded_params, true);
                let dt = &names.dt;
                (
                    TokenStream::new(),
                    quote!(#dt::Variant::unnamed().field(#resolve).build()),
                )
            } else {
                match &variant.fields {
                    Fields::Unit => {
                        let dt = &names.dt;
                        (TokenStream::new(), quote!(#dt::Variant::unit()))
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

                                construct_field_with_variant_skip(
                                    names,
                                    container_attrs,
                                    field_attrs,
                                    &field.ty,
                                    raw_attrs,
                                    used_generic_types,
                                    bounded_params,
                                    attrs.skip,
                                    &format_ident!("field{}", idx),
                                )
                                .map(|FieldTokens { prelude, value }| (prelude, value))
                            })
                            .collect::<syn::Result<Vec<(TokenStream, TokenStream)>>>()?;

                        let dt = &names.dt;
                        variant_builder(quote!(#dt::Variant::unnamed()), fields)
                    }
                    Fields::Named(fields) => {
                        let field_calls = fields
                            .named
                            .iter()
                            .enumerate()
                            .map(|(idx, field)| {
                                let (field_attrs, raw_attrs) =
                                    decode_field_attrs(field, &container_attrs.skip_attrs)?;

                                let field_name =
                                    unraw_raw_ident(field.ident.as_ref().ok_or_else(|| {
                                        syn::Error::new(
                                            field.span(),
                                            "specta: named field must have an identifier",
                                        )
                                    })?);

                                construct_field_with_variant_skip(
                                    names,
                                    container_attrs,
                                    field_attrs,
                                    &field.ty,
                                    raw_attrs,
                                    used_generic_types,
                                    bounded_params,
                                    attrs.skip,
                                    &format_ident!("field{}", idx),
                                )
                                .map(|field| {
                                    let FieldTokens { prelude, value } = field;
                                    (prelude, quote!(#field_name, #value))
                                })
                            })
                            .collect::<syn::Result<Vec<(TokenStream, TokenStream)>>>()?;

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
                    #variant_prelude
                    let #binding = #variant_value;
                }
            } else {
                quote! {
                    #variant_prelude
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
        return Ok(quote!(#dt::Enum::default().into()));
    }

    let steps = variant_steps.iter().map(|(steps, _)| steps);
    let variants = variant_steps.iter().enumerate().map(|(index, (_, name))| {
        let binding = format_ident!("v{}", index);
        quote!((#name.into(), #binding))
    });

    Ok(quote!({
        let mut e = #dt::Enum::default();
        #(#steps)*
        e.variants = [#(#variants),*].into();
        #attrs_apply
        e.into()
    }))
}

/// The statements that prepare a variant's fields, plus the variant value.
///
/// `VariantBuilder::field` consumes the builder, so the fields have to be
/// chained; only the statements preparing each field are hoisted out, which
/// keeps them out of the chain's argument positions.
fn variant_builder(
    constructor: TokenStream,
    fields: Vec<(TokenStream, TokenStream)>,
) -> (TokenStream, TokenStream) {
    if fields.is_empty() {
        return (TokenStream::new(), quote!(#constructor.build()));
    }

    let prelude = fields.iter().map(|(prelude, _)| prelude);
    let calls = fields.iter().map(|(_, args)| quote!(.field(#args)));
    (
        quote!(#(#prelude)*),
        quote!(#constructor #(#calls)* .build()),
    )
}
