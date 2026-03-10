use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

use super::{AttributeScope, ContainerAttr, FieldAttr, build_runtime_attributes};

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
    let runtime_attrs = build_runtime_attributes(
        AttributeScope::Field,
        raw_attrs,
        &container_attrs.skip_attrs,
    );

    let ty = if attrs.skip || variant_skip {
        quote!(None)
    } else {
        quote!(Some(<#field_ty as #crate_ref::Type>::definition(types)))
    };

    Ok(quote!(internal::construct::field(
        #optional,
        false,
        #deprecated,
        #doc.into(),
        #inline,
        #runtime_attrs,
        #ty
    )))
}
