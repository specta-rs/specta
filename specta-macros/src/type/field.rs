use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::Type;

use super::{ContainerAttr, FieldAttr, lower_attr::lower_attribute};

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
    let inline = container_attrs.inline || attrs.inline;

    let lowered_field_attrs = raw_attrs
        .iter()
        .filter(|attr| {
            let path = attr.path().to_token_stream().to_string();
            !container_attrs.skip_attrs.contains(&path) && (path == "serde" || path == "specta")
        })
        .filter_map(|attr| lower_attribute(attr).transpose())
        .map(|result| result.map(|attr| attr.to_tokens()))
        .collect::<Result<Vec<_>, _>>()?;

    let ty = if attrs.skip {
        quote!(None)
    } else {
        quote!(Some(<#field_ty as specta::Type>::definition(types)))
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
