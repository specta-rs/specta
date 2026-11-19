use proc_macro2::TokenStream;
use quote::quote;
use syn::{Attribute, LitStr, Meta, Result, Type, meta::ParseNestedMeta};

use super::AttributeScope;

#[derive(Clone)]
struct ConversionType {
    type_src: String,
    ty: Type,
}

#[derive(Copy, Clone)]
enum RenameRule {
    Lower,
    Upper,
    Pascal,
    Camel,
    Snake,
    ScreamingSnake,
    Kebab,
    ScreamingKebab,
}

impl RenameRule {
    fn parse(lit: &LitStr) -> Result<Self> {
        Ok(match lit.value().as_str() {
            "lowercase" => Self::Lower,
            "UPPERCASE" => Self::Upper,
            "PascalCase" => Self::Pascal,
            "camelCase" => Self::Camel,
            "snake_case" => Self::Snake,
            "SCREAMING_SNAKE_CASE" => Self::ScreamingSnake,
            "kebab-case" => Self::Kebab,
            "SCREAMING-KEBAB-CASE" => Self::ScreamingKebab,
            _ => {
                return Err(syn::Error::new(
                    lit.span(),
                    format!("unsupported serde casing: `{}`", lit.value()),
                ));
            }
        })
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Lower => "lowercase",
            Self::Upper => "UPPERCASE",
            Self::Pascal => "PascalCase",
            Self::Camel => "camelCase",
            Self::Snake => "snake_case",
            Self::ScreamingSnake => "SCREAMING_SNAKE_CASE",
            Self::Kebab => "kebab-case",
            Self::ScreamingKebab => "SCREAMING-KEBAB-CASE",
        }
    }
}

#[derive(Default)]
struct ContainerAttrs {
    rename_serialize: Option<String>,
    rename_deserialize: Option<String>,
    rename_all_serialize: Option<RenameRule>,
    rename_all_deserialize: Option<RenameRule>,
    rename_all_fields_serialize: Option<RenameRule>,
    rename_all_fields_deserialize: Option<RenameRule>,
    tag: Option<String>,
    content: Option<String>,
    untagged: bool,
    default: bool,
    transparent: bool,
    deny_unknown_fields: bool,
    from: Option<ConversionType>,
    try_from: Option<ConversionType>,
    into: Option<ConversionType>,
    variant_identifier: bool,
    field_identifier: bool,
}

#[derive(Default)]
struct VariantAttrs {
    rename_serialize: Option<String>,
    rename_deserialize: Option<String>,
    aliases: Vec<String>,
    rename_all_serialize: Option<RenameRule>,
    rename_all_deserialize: Option<RenameRule>,
    skip_serializing: bool,
    skip_deserializing: bool,
    has_serialize_with: bool,
    has_deserialize_with: bool,
    has_with: bool,
    other: bool,
    untagged: bool,
}

#[derive(Default)]
struct FieldAttrs {
    rename_serialize: Option<String>,
    rename_deserialize: Option<String>,
    aliases: Vec<String>,
    default: bool,
    flatten: bool,
    skip_serializing: bool,
    skip_deserializing: bool,
    skip_serializing_if: Option<String>,
    has_serialize_with: bool,
    has_deserialize_with: bool,
    has_with: bool,
}

pub(super) fn lower_runtime_attributes<'a>(
    type_trait: &syn::Path,
    scope: AttributeScope,
    target: &TokenStream,
    raw_attrs: impl Iterator<Item = &'a Attribute>,
) -> Result<Option<TokenStream>> {
    match scope {
        AttributeScope::Container => parse_container_attrs(raw_attrs)
            .map(|attrs| attrs.map(|attrs| lower_container_attrs(type_trait, target, attrs))),
        AttributeScope::Variant => parse_variant_attrs(raw_attrs)
            .map(|attrs| attrs.map(|attrs| lower_variant_attrs(target, attrs))),
        AttributeScope::Field => parse_field_attrs(raw_attrs)
            .map(|attrs| attrs.map(|attrs| lower_field_attrs(target, attrs))),
    }
}

fn parse_container_attrs<'a>(
    attrs: impl Iterator<Item = &'a Attribute>,
) -> Result<Option<ContainerAttrs>> {
    let mut parsed = ContainerAttrs::default();
    let mut found = false;

    for attr in attrs.filter(|attr| attr.path().is_ident("serde")) {
        let Meta::List(list) = &attr.meta else {
            continue;
        };

        found = true;
        list.parse_nested_meta(|meta| parse_container_meta(&mut parsed, meta))?;
    }

    Ok(found.then_some(parsed))
}

fn parse_variant_attrs<'a>(
    attrs: impl Iterator<Item = &'a Attribute>,
) -> Result<Option<VariantAttrs>> {
    let mut parsed = VariantAttrs::default();
    let mut found = false;

    for attr in attrs.filter(|attr| attr.path().is_ident("serde")) {
        let Meta::List(list) = &attr.meta else {
            continue;
        };

        found = true;
        list.parse_nested_meta(|meta| parse_variant_meta(&mut parsed, meta))?;
    }

    Ok(found.then_some(parsed))
}

fn parse_field_attrs<'a>(attrs: impl Iterator<Item = &'a Attribute>) -> Result<Option<FieldAttrs>> {
    let mut parsed = FieldAttrs::default();
    let mut found = false;

    for attr in attrs.filter(|attr| attr.path().is_ident("serde")) {
        let Meta::List(list) = &attr.meta else {
            continue;
        };

        found = true;
        list.parse_nested_meta(|meta| parse_field_meta(&mut parsed, meta))?;
    }

    Ok(found.then_some(parsed))
}

fn parse_container_meta(target: &mut ContainerAttrs, meta: ParseNestedMeta<'_>) -> Result<()> {
    if meta.path.is_ident("rename") {
        parse_rename(
            &meta,
            &mut target.rename_serialize,
            &mut target.rename_deserialize,
        )?;
    } else if meta.path.is_ident("rename_all") {
        parse_rename_all(
            &meta,
            &mut target.rename_all_serialize,
            &mut target.rename_all_deserialize,
        )?;
    } else if meta.path.is_ident("rename_all_fields") {
        parse_rename_all(
            &meta,
            &mut target.rename_all_fields_serialize,
            &mut target.rename_all_fields_deserialize,
        )?;
    } else if meta.path.is_ident("tag") {
        target.tag = Some(parse_string_assignment(&meta)?);
    } else if meta.path.is_ident("content") {
        target.content = Some(parse_string_assignment(&meta)?);
    } else if meta.path.is_ident("untagged") {
        target.untagged = true;
    } else if meta.path.is_ident("default") {
        parse_default_assignment(&meta)?;
        target.default = true;
    } else if meta.path.is_ident("transparent") {
        target.transparent = true;
    } else if meta.path.is_ident("deny_unknown_fields") {
        target.deny_unknown_fields = true;
    } else if meta.path.is_ident("from") {
        target.from = Some(parse_conversion_assignment(&meta)?);
    } else if meta.path.is_ident("try_from") {
        target.try_from = Some(parse_conversion_assignment(&meta)?);
    } else if meta.path.is_ident("into") {
        target.into = Some(parse_conversion_assignment(&meta)?);
    } else if meta.path.is_ident("variant_identifier") {
        target.variant_identifier = true;
    } else if meta.path.is_ident("field_identifier") {
        target.field_identifier = true;
    } else {
        skip_unknown_meta(&meta)?;
    }

    Ok(())
}

fn parse_variant_meta(target: &mut VariantAttrs, meta: ParseNestedMeta<'_>) -> Result<()> {
    if meta.path.is_ident("rename") {
        parse_rename(
            &meta,
            &mut target.rename_serialize,
            &mut target.rename_deserialize,
        )?;
    } else if meta.path.is_ident("alias") {
        target.aliases.push(parse_string_assignment(&meta)?);
    } else if meta.path.is_ident("rename_all") {
        parse_rename_all(
            &meta,
            &mut target.rename_all_serialize,
            &mut target.rename_all_deserialize,
        )?;
    } else if meta.path.is_ident("skip") {
        target.skip_serializing = true;
        target.skip_deserializing = true;
    } else if meta.path.is_ident("skip_serializing") {
        target.skip_serializing = true;
    } else if meta.path.is_ident("skip_deserializing") {
        target.skip_deserializing = true;
    } else if meta.path.is_ident("serialize_with") {
        target.has_serialize_with = true;
        parse_string_assignment(&meta)?;
    } else if meta.path.is_ident("deserialize_with") {
        target.has_deserialize_with = true;
        parse_string_assignment(&meta)?;
    } else if meta.path.is_ident("with") {
        target.has_with = true;
        parse_string_assignment(&meta)?;
    } else if meta.path.is_ident("other") {
        target.other = true;
    } else if meta.path.is_ident("untagged") {
        target.untagged = true;
    } else {
        skip_unknown_meta(&meta)?;
    }

    Ok(())
}

fn parse_field_meta(target: &mut FieldAttrs, meta: ParseNestedMeta<'_>) -> Result<()> {
    if meta.path.is_ident("rename") {
        parse_rename(
            &meta,
            &mut target.rename_serialize,
            &mut target.rename_deserialize,
        )?;
    } else if meta.path.is_ident("alias") {
        target.aliases.push(parse_string_assignment(&meta)?);
    } else if meta.path.is_ident("default") {
        parse_default_assignment(&meta)?;
        target.default = true;
    } else if meta.path.is_ident("flatten") {
        target.flatten = true;
    } else if meta.path.is_ident("skip") {
        target.skip_serializing = true;
        target.skip_deserializing = true;
    } else if meta.path.is_ident("skip_serializing") {
        target.skip_serializing = true;
    } else if meta.path.is_ident("skip_deserializing") {
        target.skip_deserializing = true;
    } else if meta.path.is_ident("skip_serializing_if") {
        target.skip_serializing_if = Some(parse_string_assignment(&meta)?);
    } else if meta.path.is_ident("serialize_with") {
        target.has_serialize_with = true;
        parse_string_assignment(&meta)?;
    } else if meta.path.is_ident("deserialize_with") {
        target.has_deserialize_with = true;
        parse_string_assignment(&meta)?;
    } else if meta.path.is_ident("with") {
        target.has_with = true;
        parse_string_assignment(&meta)?;
    } else {
        skip_unknown_meta(&meta)?;
    }

    Ok(())
}

/// Consume the value attached to an unrecognized serde attribute so parsing of the surrounding
/// attribute list can continue (e.g. `#[serde(expecting = "...", rename_all = "camelCase")]`).
///
/// These attributes don't affect the exported type's wire shape, so specta intentionally ignores
/// them — just like it already does for unrecognized path-only attributes (e.g. `field_identifier`
/// on a struct). Without this, `syn`'s `parse_nested_meta` would error with `expected `,`` on the
/// leftover `= value` or `(...)` tokens.
fn skip_unknown_meta(meta: &ParseNestedMeta<'_>) -> Result<()> {
    if meta.input.peek(syn::Token![=]) {
        meta.value()?.parse::<syn::Expr>()?;
    } else if meta.input.peek(syn::token::Paren) {
        meta.parse_nested_meta(|meta| skip_unknown_meta(&meta))?;
    }

    Ok(())
}

fn parse_rename(
    meta: &ParseNestedMeta<'_>,
    rename_serialize: &mut Option<String>,
    rename_deserialize: &mut Option<String>,
) -> Result<()> {
    if meta.input.peek(syn::Token![=]) {
        let value = parse_string_assignment(meta)?;
        *rename_serialize = Some(value.clone());
        *rename_deserialize = Some(value);
        return Ok(());
    }

    meta.parse_nested_meta(|meta| {
        if meta.path.is_ident("serialize") {
            *rename_serialize = Some(parse_string_assignment(&meta)?);
        } else if meta.path.is_ident("deserialize") {
            *rename_deserialize = Some(parse_string_assignment(&meta)?);
        }

        Ok(())
    })
}

fn parse_rename_all(
    meta: &ParseNestedMeta<'_>,
    rename_serialize: &mut Option<RenameRule>,
    rename_deserialize: &mut Option<RenameRule>,
) -> Result<()> {
    if meta.input.peek(syn::Token![=]) {
        let lit = parse_lit_str(meta)?;
        let rule = RenameRule::parse(&lit)?;
        *rename_serialize = Some(rule);
        *rename_deserialize = Some(rule);
        return Ok(());
    }

    meta.parse_nested_meta(|meta| {
        if meta.path.is_ident("serialize") {
            *rename_serialize = Some(RenameRule::parse(&parse_lit_str(&meta)?)?);
        } else if meta.path.is_ident("deserialize") {
            *rename_deserialize = Some(RenameRule::parse(&parse_lit_str(&meta)?)?);
        }

        Ok(())
    })
}

fn parse_default_assignment(meta: &ParseNestedMeta<'_>) -> Result<String> {
    if meta.input.peek(syn::Token![=]) {
        parse_string_assignment(meta)
    } else {
        Ok("__default__".to_owned())
    }
}

fn parse_conversion_assignment(meta: &ParseNestedMeta<'_>) -> Result<ConversionType> {
    let lit = parse_lit_str(meta)?;
    let type_src = lit.value();
    let ty = syn::parse_str::<Type>(&type_src)
        .map_err(|err| syn::Error::new(lit.span(), format!("invalid type literal: {err}")))?;

    Ok(ConversionType { type_src, ty })
}

fn parse_string_assignment(meta: &ParseNestedMeta<'_>) -> Result<String> {
    parse_lit_str(meta).map(|lit| lit.value())
}

fn parse_lit_str(meta: &ParseNestedMeta<'_>) -> Result<LitStr> {
    meta.value()?.parse()
}

fn lower_container_attrs(
    type_trait: &syn::Path,
    target: &TokenStream,
    attrs: ContainerAttrs,
) -> TokenStream {
    let mut inserts = Vec::new();
    push_opt_string(
        &mut inserts,
        target,
        "serde:container:rename_serialize",
        &attrs.rename_serialize,
    );
    push_opt_string(
        &mut inserts,
        target,
        "serde:container:rename_deserialize",
        &attrs.rename_deserialize,
    );
    push_opt_rename_rule(
        &mut inserts,
        target,
        "serde:container:rename_all_serialize",
        attrs.rename_all_serialize,
    );
    push_opt_rename_rule(
        &mut inserts,
        target,
        "serde:container:rename_all_deserialize",
        attrs.rename_all_deserialize,
    );
    push_opt_rename_rule(
        &mut inserts,
        target,
        "serde:container:rename_all_fields_serialize",
        attrs.rename_all_fields_serialize,
    );
    push_opt_rename_rule(
        &mut inserts,
        target,
        "serde:container:rename_all_fields_deserialize",
        attrs.rename_all_fields_deserialize,
    );
    push_opt_string(&mut inserts, target, "serde:container:tag", &attrs.tag);
    push_opt_string(
        &mut inserts,
        target,
        "serde:container:content",
        &attrs.content,
    );
    push_bool(
        &mut inserts,
        target,
        "serde:container:untagged",
        attrs.untagged,
    );
    push_bool(
        &mut inserts,
        target,
        "serde:container:default",
        attrs.default,
    );
    push_bool(
        &mut inserts,
        target,
        "serde:container:transparent",
        attrs.transparent,
    );
    push_bool(
        &mut inserts,
        target,
        "serde:container:deny_unknown_fields",
        attrs.deny_unknown_fields,
    );
    push_opt_conversion(
        &mut inserts,
        target,
        type_trait,
        "serde:container:from_type_src",
        "serde:container:from_resolved",
        &attrs.from,
    );
    push_opt_conversion(
        &mut inserts,
        target,
        type_trait,
        "serde:container:try_from_type_src",
        "serde:container:try_from_resolved",
        &attrs.try_from,
    );
    push_opt_conversion(
        &mut inserts,
        target,
        type_trait,
        "serde:container:into_type_src",
        "serde:container:into_resolved",
        &attrs.into,
    );
    push_bool(
        &mut inserts,
        target,
        "serde:container:variant_identifier",
        attrs.variant_identifier,
    );
    push_bool(
        &mut inserts,
        target,
        "serde:container:field_identifier",
        attrs.field_identifier,
    );

    quote!(#(#inserts)*)
}

fn lower_variant_attrs(target: &TokenStream, attrs: VariantAttrs) -> TokenStream {
    let mut inserts = Vec::new();
    push_opt_string(
        &mut inserts,
        target,
        "serde:variant:rename_serialize",
        &attrs.rename_serialize,
    );
    push_opt_string(
        &mut inserts,
        target,
        "serde:variant:rename_deserialize",
        &attrs.rename_deserialize,
    );
    push_vec_string(
        &mut inserts,
        target,
        "serde:variant:aliases",
        &attrs.aliases,
    );
    push_opt_rename_rule(
        &mut inserts,
        target,
        "serde:variant:rename_all_serialize",
        attrs.rename_all_serialize,
    );
    push_opt_rename_rule(
        &mut inserts,
        target,
        "serde:variant:rename_all_deserialize",
        attrs.rename_all_deserialize,
    );
    push_bool(
        &mut inserts,
        target,
        "serde:variant:skip_serializing",
        attrs.skip_serializing,
    );
    push_bool(
        &mut inserts,
        target,
        "serde:variant:skip_deserializing",
        attrs.skip_deserializing,
    );
    push_bool(
        &mut inserts,
        target,
        "serde:variant:has_serialize_with",
        attrs.has_serialize_with,
    );
    push_bool(
        &mut inserts,
        target,
        "serde:variant:has_deserialize_with",
        attrs.has_deserialize_with,
    );
    push_bool(
        &mut inserts,
        target,
        "serde:variant:has_with",
        attrs.has_with,
    );
    push_bool(&mut inserts, target, "serde:variant:other", attrs.other);
    push_bool(
        &mut inserts,
        target,
        "serde:variant:untagged",
        attrs.untagged,
    );

    quote!(#(#inserts)*)
}

fn lower_field_attrs(target: &TokenStream, attrs: FieldAttrs) -> TokenStream {
    let mut inserts = Vec::new();
    push_opt_string(
        &mut inserts,
        target,
        "serde:field:rename_serialize",
        &attrs.rename_serialize,
    );
    push_opt_string(
        &mut inserts,
        target,
        "serde:field:rename_deserialize",
        &attrs.rename_deserialize,
    );
    push_vec_string(&mut inserts, target, "serde:field:aliases", &attrs.aliases);
    push_bool(&mut inserts, target, "serde:field:default", attrs.default);
    push_bool(&mut inserts, target, "serde:field:flatten", attrs.flatten);
    push_bool(
        &mut inserts,
        target,
        "serde:field:skip_serializing",
        attrs.skip_serializing,
    );
    push_bool(
        &mut inserts,
        target,
        "serde:field:skip_deserializing",
        attrs.skip_deserializing,
    );
    push_opt_string(
        &mut inserts,
        target,
        "serde:field:skip_serializing_if",
        &attrs.skip_serializing_if,
    );
    push_bool(
        &mut inserts,
        target,
        "serde:field:has_serialize_with",
        attrs.has_serialize_with,
    );
    push_bool(
        &mut inserts,
        target,
        "serde:field:has_deserialize_with",
        attrs.has_deserialize_with,
    );
    push_bool(&mut inserts, target, "serde:field:has_with", attrs.has_with);

    quote!(#(#inserts)*)
}

fn push_opt_string(
    inserts: &mut Vec<TokenStream>,
    target: &TokenStream,
    key: &str,
    value: &Option<String>,
) {
    if let Some(value) = value {
        inserts.push(quote!(#target.insert(#key, #value.to_owned());));
    }
}

fn push_vec_string(
    inserts: &mut Vec<TokenStream>,
    target: &TokenStream,
    key: &str,
    value: &[String],
) {
    if value.is_empty() {
        return;
    }

    let value = value.iter().map(|value| quote!(#value.to_owned()));
    // `Vec::from([..])` instead of `vec![..]` keeps the `vec!` box machinery out
    // of every lowered attribute.
    inserts.push(quote!(#target.insert(#key, ::std::vec::Vec::from([#(#value),*]));));
}

fn push_bool(inserts: &mut Vec<TokenStream>, target: &TokenStream, key: &str, value: bool) {
    if value {
        inserts.push(quote!(#target.insert(#key, true);));
    }
}

fn push_opt_rename_rule(
    inserts: &mut Vec<TokenStream>,
    target: &TokenStream,
    key: &str,
    value: Option<RenameRule>,
) {
    if let Some(value) = value {
        let value = value.as_str();
        inserts.push(quote!(#target.insert(#key, #value.to_owned());));
    }
}

fn push_opt_conversion(
    inserts: &mut Vec<TokenStream>,
    target: &TokenStream,
    type_trait: &syn::Path,
    src_key: &str,
    resolved_key: &str,
    value: &Option<ConversionType>,
) {
    if let Some(value) = value {
        let type_src = &value.type_src;
        let ty = &value.ty;
        inserts.push(quote!(#target.insert(#src_key, #type_src.to_owned());));
        inserts
            .push(quote!(#target.insert(#resolved_key, <#ty as #type_trait>::definition(types));));
    }
}
