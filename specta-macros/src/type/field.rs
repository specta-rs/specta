use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

use crate::r#type::attr::deprecated_as_tokens;

use crate::utils::invisible;

use super::{
    Names,
    generics::{BuildKind, resolve_ty_expr, type_with_inferred_lifetimes},
    r#struct::FieldInfo,
};

/// A field's value, as the expression that builds it.
///
/// Field metadata is written with the consuming setters on `datatype::Field`, so
/// the whole field — type resolution and metadata — is one expression. Keeping
/// it out of a `let mut` binding also keeps the compiler from having to hold a
/// `Field` (which owns a map and a `Cow`) alive across statements, with the drop
/// flag and cleanup path that costs once per field.
pub struct FieldTokens {
    pub value: TokenStream,
    /// Whether the type expression names one of the container's parameters, so
    /// the function holding it must be generic.
    pub uses_params: bool,
}

pub fn construct_field(
    names: &Names,
    field: &FieldInfo,
    field_ty: &Type,
    used_generic_types: &[&syn::Ident],
    bounded_params: &[&syn::Ident],
    build: BuildKind,
) -> syn::Result<FieldTokens> {
    construct_field_with_variant_skip(
        names,
        field,
        field_ty,
        used_generic_types,
        bounded_params,
        false,
        build,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn construct_field_with_variant_skip(
    names: &Names,
    field: &FieldInfo,
    field_ty: &Type,
    used_generic_types: &[&syn::Ident],
    bounded_params: &[&syn::Ident],
    variant_skip: bool,
    build: BuildKind,
) -> syn::Result<FieldTokens> {
    crate::phase!("field:construct");

    let attrs = &field.attrs;

    // Checked on the *declared* Rust type, never the `#[specta(type = ...)]`
    // override below: serde only ever sees the real field type, so the
    // override must not influence serde-behavioral markers.
    let declared_ty_is_option = is_option_type(field_ty);

    let field_ty = type_with_inferred_lifetimes(attrs.r#type.as_ref().unwrap_or(field_ty));

    let runtime_attrs = field.runtime_attrs.as_ref();

    let field_optional = attrs.optional.then(|| quote!(.optional()));

    let field_deprecated = attrs.common.deprecated.as_ref().map(|deprecated| {
        let tokens = deprecated_as_tokens(&names.dt, deprecated);
        quote!(.deprecated(#tokens))
    });

    let field_docs = (!attrs.common.doc.is_empty()).then(|| {
        let docs = &attrs.common.doc;
        quote!(.docs(#docs))
    });

    // The keys are interpolated, not written as literals: `quote!` builds a
    // literal written in the template by lexing its source text, which costs an
    // order of magnitude more than turning a `&str` into one.
    let type_override = "specta:type_override";
    let type_overridden_attribute = attrs
        .r#type
        .as_ref()
        .map(|_| quote!(.attribute(#type_override, true)));

    let newtype_skip_ignored = "specta:serde_newtype_skip_ignored";
    let serde_newtype_skip_ignored = attrs
        .serde_newtype_skip_ignored
        .then(|| quote!(.attribute(#newtype_skip_ignored, true)));

    // Whether a field was declared as `Option<T>` matters to consumers
    // beyond the exported datatype: e.g. serde deserializes a missing value
    // into an `Option` as `None` (both for skipped fields and for newtype
    // enum payloads) while other types have stricter requirements. Record it
    // syntactically -- for symmetric `#[serde(skip)]` the type never even
    // enters the datatype graph, and elsewhere the exported datatype may be
    // overridden (`#[specta(type = ...)]`), so the real syntax is the only
    // reliable source.
    let nullable_attribute = declared_ty_is_option.then(|| quote!(.nullable()));

    // A skipped field never enters the datatype graph, so its type is not
    // resolved at all.
    let (field_ty, uses_params) = if attrs.skip || variant_skip {
        (None, false)
    } else {
        // The canonical declaration is built with each generic parameter
        // replaced by a placeholder type that resolves to `Generic`. A concrete
        // build resolves the real parameter.
        let resolved = resolve_ty_expr(names, &field_ty, used_generic_types, bounded_params, build);
        let field_ty = if attrs.inline {
            let dt = &names.dt;
            let resolve = resolved.tokens;
            quote!(#dt::inline(types, |types| #resolve))
        } else {
            resolved.tokens
        };

        (Some(field_ty), resolved.uses_params)
    };

    if field_optional.is_none()
        && field_deprecated.is_none()
        && field_docs.is_none()
        && runtime_attrs.is_none()
        && type_overridden_attribute.is_none()
        && serde_newtype_skip_ignored.is_none()
        && nullable_attribute.is_none()
    {
        // A `DataType` converts into the `Field` that carries it, so the bare
        // case hands the type expression straight to the builder instead of
        // wrapping it in a constructor call.
        let field = match field_ty {
            // Already an invisible group from `resolve_ty_expr`.
            Some(ty) => ty,
            None => {
                let dt = &names.dt;
                invisible(quote!(#dt::Field::default()))
            }
        };
        return Ok(FieldTokens {
            value: field,
            uses_params,
        });
    }

    let dt = &names.dt;
    let field = match field_ty {
        Some(ty) => quote!(#dt::Field::new(#ty)),
        None => quote!(#dt::Field::default()),
    };

    Ok(FieldTokens {
        value: invisible(quote! {
            #field #field_optional #field_deprecated #field_docs #runtime_attrs #type_overridden_attribute #serde_newtype_skip_ignored #nullable_attribute
        }),
        uses_params,
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
