use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

use super::{ContainerAttr, FieldAttr};
use crate::utils::Attribute;

// Construct a field.
pub fn construct_field(
    container_attrs: &ContainerAttr,
    attrs: FieldAttr,
    field_ty: &Type,
    raw_attrs: &[Attribute],
) -> TokenStream {
    let field_ty = attrs.r#type.as_ref().unwrap_or(field_ty);
    let deprecated = attrs.common.deprecated_as_tokens();
    let optional = attrs.optional;
    let doc = attrs.common.doc;
    let flatten = attrs.flatten;
    let inline = container_attrs.inline || attrs.inline;

    let runtime_attrs = convert_attrs_to_runtime_attrs(raw_attrs);

    // Skip must be handled by the macro so that we don't try and constrain the inner type to `Type` or `Flatten` traits.
    if attrs.skip {
        return quote!(internal::construct::skipped_field(
            #optional,
            #flatten,
            #inline,
            #deprecated,
            #doc.into(),
            #runtime_attrs
        ));
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

    ty
}

// Convert parsed attributes to runtime attributes
fn convert_attrs_to_runtime_attrs(raw_attrs: &[Attribute]) -> TokenStream {
    let mut runtime_attrs = Vec::new();

    for attr in raw_attrs {
        if attr.key == "serde" || attr.key == "specta" {
            let path = attr.key.to_string();
            let kind = match &attr.value {
                Some(value) => {
                    let value_str = match value {
                        crate::utils::AttributeValue::Lit(lit) => quote! { #lit }.to_string(),
                        crate::utils::AttributeValue::Path(path) => quote! { #path }.to_string(),
                        crate::utils::AttributeValue::Attribute { attr, .. } => {
                            // For nested attributes, serialize the inner attributes
                            let inner: Vec<String> =
                                attr.iter().map(|a| format!("{}", a.key)).collect();
                            inner.join(",")
                        }
                    };
                    quote! {
                        datatype::RuntimeMeta::List(vec![
                            datatype::RuntimeNestedMeta::Literal(
                                datatype::RuntimeLiteral::Str(#value_str.to_string())
                            )
                        ])
                    }
                }
                None => {
                    // Note: This is a simplified representation.
                    // For proper attribute handling, use the syn-based lower_attr.rs implementation.
                    // This case represents a path-only attribute like #[serde] with no arguments.
                    let path_name = path.clone();
                    quote! { datatype::RuntimeMeta::Path(#path_name.to_string()) }
                }
            };

            runtime_attrs.push(quote! {
                datatype::RuntimeAttribute {
                    path: #path.to_string(),
                    kind: #kind,
                }
            });
        }
    }

    quote! {
        vec![#(#runtime_attrs),*]
    }
}
