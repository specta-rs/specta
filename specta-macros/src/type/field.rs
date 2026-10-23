use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::Type;

use crate::r#type::attr::deprecated_as_tokens;

use super::{
    AttributeScope, ContainerAttr, FieldAttr, build_runtime_attributes,
    generics::{resolve_ty_expr, type_with_inferred_lifetimes},
};

pub fn construct_field(
    crate_ref: &TokenStream,
    container_attrs: &ContainerAttr,
    attrs: FieldAttr,
    field_ty: &Type,
    raw_attrs: &[syn::Attribute],
    used_generic_types: &[syn::Ident],
    bounded_params: &HashSet<String>,
) -> syn::Result<TokenStream> {
    construct_field_with_variant_skip(
        crate_ref,
        container_attrs,
        attrs,
        field_ty,
        raw_attrs,
        used_generic_types,
        bounded_params,
        false,
    )
}

pub fn construct_field_with_variant_skip(
    crate_ref: &TokenStream,
    container_attrs: &ContainerAttr,
    attrs: FieldAttr,
    field_ty: &Type,
    raw_attrs: &[syn::Attribute],
    used_generic_types: &[syn::Ident],
    bounded_params: &HashSet<String>,
    variant_skip: bool,
) -> syn::Result<TokenStream> {
    // Checked on the *declared* Rust type, never the `#[specta(type = ...)]`
    // override below: serde only ever sees the real field type, so the
    // override must not influence serde-behavioral markers.
    let declared_ty_is_option = is_option_type(field_ty);

    let field_ty = type_with_inferred_lifetimes(attrs.r#type.as_ref().unwrap_or(field_ty));

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
        let docs = &attrs.common.doc;
        quote! {
            field.docs = Cow::Borrowed(#docs);
        }
    });

    let type_overridden_attribute = attrs
        .r#type
        .as_ref()
        .map(|_| quote!(field.attributes.insert(#crate_ref::markers::TYPE_OVERRIDE_MARKER, true);));

    let serde_newtype_skip_ignored = attrs.serde_newtype_skip_ignored.then(|| {
        quote!(field.attributes.insert(#crate_ref::markers::SERDE_NEWTYPE_SKIP_IGNORED_MARKER, true);)
    });

    // Whether a field was declared as `Option<T>` matters to consumers
    // beyond the exported datatype: e.g. serde deserializes a missing value
    // into an `Option` as `None` (both for skipped fields and for newtype
    // enum payloads) while other types have stricter requirements. Record it
    // syntactically -- for symmetric `#[serde(skip)]` the type never even
    // enters the datatype graph, and elsewhere the exported datatype may be
    // overridden (`#[specta(type = ...)]`), so the real syntax is the only
    // reliable source.
    let nullable_attribute = declared_ty_is_option
        .then(|| quote!(field.attributes.insert(#crate_ref::markers::NULLABLE_MARKER, true);));

    // The canonical declaration is built with each generic parameter replaced by
    // a placeholder type that resolves to `Generic`. Concrete instantiation
    // (`declaration == false`) resolves the real parameter.
    let resolve = resolve_ty_expr(
        crate_ref,
        &field_ty,
        used_generic_types,
        bounded_params,
        true,
    );

    let field_ty = if attrs.skip || variant_skip {
        quote!()
    } else if attrs.inline {
        quote!(field.ty = Some(datatype::inline(types, |types| #resolve));)
    } else {
        quote!(field.ty = Some(#resolve);)
    };

    Ok(quote!({
        let mut field = datatype::Field::default();
        #field_optional
        #field_deprecated
        #field_docs
        #runtime_attrs
        #type_overridden_attribute
        #serde_newtype_skip_ignored
        #nullable_attribute
        #field_ty
        field
    }))
}

/// Whether a type is syntactically the standard `Option<T>`.
///
/// A bare single-segment `Option` is assumed to be the prelude's (a local
/// type shadowing it is the accepted false-positive tradeoff of syntactic
/// detection). Multi-segment paths only match the real
/// `std::option::Option`/`core::option::Option` spellings -- a user-defined
/// path that merely ends in `Option` (e.g. `wire::Option<T>`) gets none of
/// serde's `Option` special-casing and must not match. Aliases of `Option`
/// are not detectable and conservatively treated as non-`Option`.
fn is_option_type(ty: &Type) -> bool {
    match ty {
        Type::Path(path) => {
            let segments = &path.path.segments;
            match segments.len() {
                1 => segments[0].ident == "Option",
                3 => {
                    (segments[0].ident == "std" || segments[0].ident == "core")
                        && segments[1].ident == "option"
                        && segments[2].ident == "Option"
                }
                _ => false,
            }
        }
        Type::Group(group) => is_option_type(&group.elem),
        Type::Paren(paren) => is_option_type(&paren.elem),
        _ => false,
    }
}
