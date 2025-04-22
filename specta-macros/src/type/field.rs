use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

use super::{ContainerAttr, FieldAttr};

// Construct a field.
pub fn construct_field(
    crate_ref: &TokenStream,
    container_attrs: &ContainerAttr,
    attrs: FieldAttr,
    field_ty: &Type,
) -> TokenStream {
    let field_ty = attrs.r#type.as_ref().unwrap_or(&field_ty);
    let deprecated = attrs.common.deprecated_as_tokens(crate_ref);
    let optional = attrs.optional;
    let doc = attrs.common.doc;
    let flatten = attrs.flatten;
    let inline = container_attrs.inline || attrs.inline;

    // Skip must be handled by the macro so that we don't try and constrain the inner type to `Type` or `Flatten` traits.
    if attrs.skip {
        return quote!(#crate_ref::internal::construct::skipped_field(
            #optional,
            #flatten,
            #inline,
            #deprecated,
            #doc.into()
        ));
    }

    let method = attrs
        .flatten
        .then(|| quote!(field_flattened))
        .unwrap_or_else(|| quote!(field));
    let ty = quote!(#crate_ref::internal::construct::#method::<#field_ty>(
        #optional,
        #inline,
        #deprecated,
        #doc.into(),
        types
    ));

    ty
}
