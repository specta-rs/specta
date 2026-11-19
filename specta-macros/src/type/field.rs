use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Type};

use crate::r#type::attr::deprecated_as_tokens;

use super::{
    AttributeScope, ContainerAttr, FieldAttr, Names, build_runtime_attributes,
    generics::{resolve_ty_expr, type_with_inferred_lifetimes},
};

/// A field value together with the statements that prepare it.
///
/// Field metadata is written by mutating a `Field`, so those statements are
/// emitted ahead of the field's use rather than inside its argument position,
/// where the pretty-printed form indents every one of them a level deeper.
/// `binding` names the value in those statements, so a caller that hoists
/// several fields into one expression can keep them apart.
pub struct FieldTokens {
    pub prelude: TokenStream,
    pub value: TokenStream,
}

impl FieldTokens {
    fn bare(value: TokenStream) -> Self {
        Self {
            prelude: TokenStream::new(),
            value,
        }
    }
}

pub fn construct_field(
    names: &Names,
    container_attrs: &ContainerAttr,
    attrs: FieldAttr,
    field_ty: &Type,
    raw_attrs: &[syn::Attribute],
    used_generic_types: &[syn::Ident],
    bounded_params: &[syn::Ident],
    binding: &Ident,
) -> syn::Result<FieldTokens> {
    construct_field_with_variant_skip(
        names,
        container_attrs,
        attrs,
        field_ty,
        raw_attrs,
        used_generic_types,
        bounded_params,
        false,
        binding,
    )
}

pub fn construct_field_with_variant_skip(
    names: &Names,
    container_attrs: &ContainerAttr,
    attrs: FieldAttr,
    field_ty: &Type,
    raw_attrs: &[syn::Attribute],
    used_generic_types: &[syn::Ident],
    bounded_params: &[syn::Ident],
    variant_skip: bool,
    binding: &Ident,
) -> syn::Result<FieldTokens> {
    // Checked on the *declared* Rust type, never the `#[specta(type = ...)]`
    // override below: serde only ever sees the real field type, so the
    // override must not influence serde-behavioral markers.
    let declared_ty_is_option = is_option_type(field_ty);

    let field_ty = type_with_inferred_lifetimes(attrs.r#type.as_ref().unwrap_or(field_ty));

    let runtime_attrs = build_runtime_attributes(
        names,
        AttributeScope::Field,
        quote!(#binding.attributes),
        raw_attrs,
        &container_attrs.skip_attrs,
    )?;

    let field_optional = attrs.optional.then(|| quote!(#binding.optional = true;));

    let field_deprecated = attrs.common.deprecated.map(|deprecated| {
        let tokens = deprecated_as_tokens(&names.dt, deprecated);
        quote!(#binding.deprecated = #tokens;)
    });

    let field_docs = (!attrs.common.doc.is_empty()).then(|| {
        let docs = &attrs.common.doc;
        quote! {
            #binding.docs = #docs.into();
        }
    });

    let type_overridden_attribute = attrs
        .r#type
        .as_ref()
        .map(|_| quote!(#binding.attributes.insert("specta:type_override", true);));

    let serde_newtype_skip_ignored = attrs
        .serde_newtype_skip_ignored
        .then(|| quote!(#binding.attributes.insert("specta:serde_newtype_skip_ignored", true);));

    // Whether a field was declared as `Option<T>` matters to consumers
    // beyond the exported datatype: e.g. serde deserializes a missing value
    // into an `Option` as `None` (both for skipped fields and for newtype
    // enum payloads) while other types have stricter requirements. Record it
    // syntactically -- for symmetric `#[serde(skip)]` the type never even
    // enters the datatype graph, and elsewhere the exported datatype may be
    // overridden (`#[specta(type = ...)]`), so the real syntax is the only
    // reliable source.
    let nullable_attribute =
        declared_ty_is_option.then(|| quote!(#binding.attributes.insert("specta:nullable", true);));

    // A skipped field never enters the datatype graph, so its type is not
    // resolved at all.
    let field_ty = if attrs.skip || variant_skip {
        None
    } else {
        // The canonical declaration is built with each generic parameter
        // replaced by a placeholder type that resolves to `Generic`. Concrete
        // instantiation (`declaration == false`) resolves the real parameter.
        let resolve = resolve_ty_expr(names, &field_ty, used_generic_types, bounded_params, true);

        Some(if attrs.inline {
            let dt = &names.dt;
            quote!(#dt::inline(types, |types| #resolve))
        } else {
            resolve
        })
    };

    let has_metadata = field_optional.is_some()
        || field_deprecated.is_some()
        || field_docs.is_some()
        || runtime_attrs.is_some()
        || type_overridden_attribute.is_some()
        || serde_newtype_skip_ignored.is_some()
        || nullable_attribute.is_some();

    if !has_metadata {
        // A `DataType` converts into the `Field` that carries it, so the bare
        // case hands the type expression straight to the builder instead of
        // wrapping it in a constructor call.
        let field = match field_ty {
            Some(ty) => ty,
            None => {
                let dt = &names.dt;
                quote!(#dt::Field::default())
            }
        };
        return Ok(FieldTokens::bare(field));
    }

    // The metadata statements mutate a `Field`, so this case names one.
    let dt = &names.dt;
    let field = match field_ty {
        Some(ty) => quote!(#dt::Field::new(#ty)),
        None => quote!(#dt::Field::default()),
    };

    Ok(FieldTokens {
        prelude: quote! {
            let mut #binding = #field;
            #field_optional
            #field_deprecated
            #field_docs
            #runtime_attrs
            #type_overridden_attribute
            #serde_newtype_skip_ignored
            #nullable_attribute
        },
        value: quote!(#binding),
    })
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
