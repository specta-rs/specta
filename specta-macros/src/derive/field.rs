use proc_macro2::TokenStream;
use quote::quote;
use std::collections::HashSet;
use syn::{Type, spanned::Spanned};

use super::{
    ContainerAttr, FieldAttr, Scope,
    attr::{
        deprecated_as_tokens, parse_attrs_with_filter, reject_unknown_specta_attrs, unraw_raw_ident,
    },
    build_runtime_attributes,
    generics::{resolve_ty_expr, type_with_inferred_lifetimes},
};

/// Parses a field's attributes, erroring on any `#[specta(...)]` key the derive
/// does not consume.
pub(crate) fn decode_field_attrs<'a>(
    field: &'a syn::Field,
    skip_attrs: &[String],
) -> syn::Result<(FieldAttr, &'a [syn::Attribute])> {
    // We pass all the attributes at the start and when decoding them pop them off the list.
    // This means at the end we can check for any that weren't consumed and throw an error.
    let mut attrs = parse_attrs_with_filter(&field.attrs, skip_attrs)?;
    let field_attrs = FieldAttr::from_attrs(&mut attrs)?;

    reject_unknown_specta_attrs(&attrs, Scope::Field)?;

    Ok((field_attrs, &field.attrs))
}

/// The field's name without its raw-ident prefix.
pub(crate) fn named_field_ident(field: &syn::Field) -> syn::Result<String> {
    field
        .ident
        .as_ref()
        .map(unraw_raw_ident)
        .ok_or_else(|| syn::Error::new(field.span(), "specta: named field must have an identifier"))
}

pub(crate) fn construct_field(
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

pub(crate) fn construct_field_with_variant_skip(
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
        Scope::Field,
        quote!(field.attributes),
        raw_attrs,
        &container_attrs.skip_attrs,
        None,
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
        .map(|_| quote!(field.attributes.insert("specta:type_override", true);));

    let serde_newtype_skip_ignored = attrs
        .serde_newtype_skip_ignored
        .then(|| quote!(field.attributes.insert("specta:serde_newtype_skip_ignored", true);));

    // Whether a field was declared as `Option<T>` matters to consumers
    // beyond the exported datatype: e.g. serde deserializes a missing value
    // into an `Option` as `None` (both for skipped fields and for newtype
    // enum payloads) while other types have stricter requirements. Record it
    // syntactically -- for symmetric `#[serde(skip)]` the type never even
    // enters the datatype graph, and elsewhere the exported datatype may be
    // overridden (`#[specta(type = ...)]`), so the real syntax is the only
    // reliable source.
    let nullable_attribute =
        declared_ty_is_option.then(|| quote!(field.attributes.insert("specta:nullable", true);));

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

    let skipped = attrs.skip || variant_skip;
    let field_ty = if skipped {
        None
    } else if attrs.inline {
        Some(quote!(datatype::inline(types, |types| #resolve)))
    } else {
        Some(resolve)
    };

    // A field without any other metadata is exactly `Field::new`, so skip the
    // `mut field` block: it is most fields, and each statement is type-checked
    // and borrow-checked per derive.
    let has_metadata = field_optional.is_some()
        || field_deprecated.is_some()
        || field_docs.is_some()
        || runtime_attrs.is_some()
        || type_overridden_attribute.is_some()
        || serde_newtype_skip_ignored.is_some()
        || nullable_attribute.is_some();
    if let (Some(ty), false) = (&field_ty, has_metadata) {
        return Ok(quote!(datatype::Field::new(#ty)));
    }

    let field_ty = field_ty.map(|ty| quote!(field.ty = Some(#ty);));

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
