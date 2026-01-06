use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::Type;

use super::{ContainerAttr, FieldAttr, lower_attr::lower_attribute};

// Construct a field.
pub fn construct_field(
    container_attrs: &ContainerAttr,
    attrs: FieldAttr,
    field_ty: &Type,
    raw_attrs: &[syn::Attribute],
) -> syn::Result<TokenStream> {
    let field_ty = attrs.r#type.as_ref().unwrap_or(field_ty);
    let deprecated = attrs.common.deprecated_as_tokens();
    let optional = attrs.optional;
    let doc = attrs.common.doc;
    let flatten = attrs.flatten;
    let inline = container_attrs.inline || attrs.inline;

    // Lower the field attributes to RuntimeAttribute tokens (same as enum variants)
    let lowered_field_attrs = raw_attrs
        .iter()
        .filter(|attr| {
            let path = attr.path().to_token_stream().to_string();
            path == "serde" || path == "specta"
        })
        .map(|attr| lower_attribute(attr).map(|attr| attr.to_tokens()))
        .collect::<Result<Vec<_>, _>>()?;

    let runtime_attrs = quote! { vec![#(#lowered_field_attrs),*] };

    // Skip must be handled by the macro so that we don't try and constrain the inner type to `Type` or `Flatten` traits.
    if attrs.skip {
        return Ok(quote!(internal::construct::skipped_field(
            #optional,
            #flatten,
            #inline,
            #deprecated,
            #doc.into(),
            #runtime_attrs
        )));
    }

    let method = attrs
        .flatten
        .then(|| quote!(field_flattened))
        .unwrap_or_else(|| quote!(field));
    let ty = quote!(internal::construct::#method::<#field_ty>(
        #optional,
        #inline,
        #deprecated,
        #doc.into(),
        types,
        #runtime_attrs
    ));

    Ok(ty)
}
