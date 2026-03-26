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
        crate_ref,
        AttributeScope::Field,
        raw_attrs,
        &container_attrs.skip_attrs,
    )?;
    let type_overridden = attrs.r#type.is_some();

    let ty = if attrs.skip || variant_skip {
        quote!(None)
    } else {
        quote!(Some(<#field_ty as #crate_ref::Type>::definition(types)))
    };

    Ok(quote!({
        let mut field = datatype::Field::default();
        field.set_optional(#optional);
        field.set_deprecated(#deprecated);
        field.set_docs(#doc.into());
        field.set_inline(#inline);
        field.set_type_overridden(#type_overridden);
        field.set_attributes(#runtime_attrs);
        if let Some(ty) = #ty {
            field.set_ty(ty);
        }
        field
    }))
}
