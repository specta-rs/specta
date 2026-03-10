use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::Type;

use super::{ContainerAttr, FieldAttr, lower_attr::lower_attribute};

pub fn construct_field(
    crate_ref: &TokenStream,
    container_attrs: &ContainerAttr,
    attrs: FieldAttr,
    field_ty: &Type,
    raw_attrs: &[syn::Attribute],
) -> syn::Result<TokenStream> {
    construct_field_with_variant_skip(
        crate_ref,
        container_attrs,
        attrs,
        field_ty,
        raw_attrs,
        false,
    )
}

pub fn construct_field_with_variant_skip(
    crate_ref: &TokenStream,
    container_attrs: &ContainerAttr,
    attrs: FieldAttr,
    field_ty: &Type,
    raw_attrs: &[syn::Attribute],
    variant_skip: bool,
) -> syn::Result<TokenStream> {
    let field_ty = attrs.r#type.as_ref().unwrap_or(field_ty);
    let deprecated = attrs.common.deprecated_as_tokens();
    let optional = attrs.optional;
    let doc = attrs.common.doc;
    let inline = attrs.inline;

    let lowered_field_attrs = raw_attrs
        .iter()
        .filter(|attr| {
            let path = attr.path().to_token_stream().to_string();
            !container_attrs.skip_attrs.contains(&path) && path != "specta"
        })
        .filter_map(|attr| lower_attribute(attr).transpose())
        .map(|result| result.map(|attr| attr.to_tokens()))
        .collect::<Result<Vec<_>, _>>()?;

    let ty = if attrs.skip || variant_skip {
        quote!(None)
    } else {
        quote!(Some(<#field_ty as #crate_ref::Type>::definition(types)))
    };

    Ok(quote!(internal::construct::field(
        #optional,
        #deprecated,
        #doc.into(),
        #inline,
        vec![#(#lowered_field_attrs),*],
        #ty
    )))
}
