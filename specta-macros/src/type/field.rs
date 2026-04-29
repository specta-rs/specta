use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

use crate::r#type::attr::deprecated_as_tokens;

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

    let runtime_attrs = build_runtime_attributes(
        crate_ref,
        AttributeScope::Field,
        quote!(field.attributes),
        raw_attrs,
        &container_attrs.skip_attrs,
    )?;

    let field_optional = attrs.optional.then(|| quote!(field.optional = true;));

    let field_deprecated = attrs.common.deprecated.map(|deprecated| {
        let tokens = deprecated_as_tokens(deprecated);
        quote!(field.deprecated = #tokens;)
    });

    let field_docs = (!attrs.common.doc.is_empty()).then(|| {
        let docs = &container_attrs.common.doc;
        quote! {
            field.docs = Cow::Borrowed(#docs);
        }
    });

    let type_overridden_attribute = attrs
        .r#type
        .as_ref()
        .map(|_| quote!(field.attributes.insert("specta:type_override", true);));

    let field_ty = if attrs.skip || variant_skip {
        quote!()
    } else if attrs.inline {
        quote!(field.ty = Some(datatype::inline(types, |types| <#field_ty as #crate_ref::Type>::definition(types)));)
    } else {
        quote!(field.ty = Some(<#field_ty as #crate_ref::Type>::definition(types));)
    };

    Ok(quote!({
        let mut field = datatype::Field::default();
        #field_optional
        #field_deprecated
        #field_docs
        #runtime_attrs
        #type_overridden_attribute
        #field_ty
        field
    }))
}
