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
    LowerCase,
    UpperCase,
    PascalCase,
    CamelCase,
    SnakeCase,
    ScreamingSnakeCase,
    KebabCase,
    ScreamingKebabCase,
}

impl RenameRule {
    fn parse(lit: &LitStr) -> Result<Self> {
        Ok(match lit.value().as_str() {
            "lowercase" => Self::LowerCase,
            "UPPERCASE" => Self::UpperCase,
            "PascalCase" => Self::PascalCase,
            "camelCase" => Self::CamelCase,
            "snake_case" => Self::SnakeCase,
            "SCREAMING_SNAKE_CASE" => Self::ScreamingSnakeCase,
            "kebab-case" => Self::KebabCase,
            "SCREAMING-KEBAB-CASE" => Self::ScreamingKebabCase,
            _ => {
                return Err(syn::Error::new(
                    lit.span(),
                    format!("unsupported serde casing: `{}`", lit.value()),
                ));
            }
        })
    }

    fn to_tokens(self, crate_ref: &TokenStream) -> TokenStream {
        let variant = match self {
            Self::LowerCase => quote!(LowerCase),
            Self::UpperCase => quote!(UpperCase),
            Self::PascalCase => quote!(PascalCase),
            Self::CamelCase => quote!(CamelCase),
            Self::SnakeCase => quote!(SnakeCase),
            Self::ScreamingSnakeCase => quote!(ScreamingSnakeCase),
            Self::KebabCase => quote!(KebabCase),
            Self::ScreamingKebabCase => quote!(ScreamingKebabCase),
        };

        quote!(#crate_ref::datatype::SerdeRenameRule::#variant)
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
    deny_unknown_fields: bool,
    tag: Option<String>,
    content: Option<String>,
    untagged: bool,
    default: Option<String>,
    transparent: bool,
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
    serialize_with: Option<String>,
    has_serialize_with: bool,
    deserialize_with: Option<String>,
    has_deserialize_with: bool,
    with: Option<String>,
    has_with: bool,
    other: bool,
    untagged: bool,
}

#[derive(Default)]
struct FieldAttrs {
    rename_serialize: Option<String>,
    rename_deserialize: Option<String>,
    aliases: Vec<String>,
    default: Option<String>,
    flatten: bool,
    skip_serializing: bool,
    skip_deserializing: bool,
    skip_serializing_if: Option<String>,
    serialize_with: Option<String>,
    has_serialize_with: bool,
    deserialize_with: Option<String>,
    has_deserialize_with: bool,
    with: Option<String>,
    has_with: bool,
}

pub(super) fn lower_runtime_attributes(
    crate_ref: &TokenStream,
    scope: AttributeScope,
    raw_attrs: &[Attribute],
) -> Result<Option<TokenStream>> {
    match scope {
        AttributeScope::Container => parse_container_attrs(raw_attrs)
            .map(|attrs| attrs.map(|attrs| lower_container_attrs(crate_ref, attrs))),
        AttributeScope::Variant => parse_variant_attrs(raw_attrs)
            .map(|attrs| attrs.map(|attrs| lower_variant_attrs(crate_ref, attrs))),
        AttributeScope::Field => parse_field_attrs(raw_attrs)
            .map(|attrs| attrs.map(|attrs| lower_field_attrs(crate_ref, attrs))),
    }
}

fn parse_container_attrs(attrs: &[Attribute]) -> Result<Option<ContainerAttrs>> {
    let mut parsed = ContainerAttrs::default();
    let mut found = false;

    for attr in attrs.iter().filter(|attr| attr.path().is_ident("serde")) {
        let Meta::List(list) = &attr.meta else {
            continue;
        };

        found = true;
        list.parse_nested_meta(|meta| parse_container_meta(&mut parsed, meta))?;
    }

    Ok(found.then_some(parsed))
}

fn parse_variant_attrs(attrs: &[Attribute]) -> Result<Option<VariantAttrs>> {
    let mut parsed = VariantAttrs::default();
    let mut found = false;

    for attr in attrs.iter().filter(|attr| attr.path().is_ident("serde")) {
        let Meta::List(list) = &attr.meta else {
            continue;
        };

        found = true;
        list.parse_nested_meta(|meta| parse_variant_meta(&mut parsed, meta))?;
    }

    Ok(found.then_some(parsed))
}

fn parse_field_attrs(attrs: &[Attribute]) -> Result<Option<FieldAttrs>> {
    let mut parsed = FieldAttrs::default();
    let mut found = false;

    for attr in attrs.iter().filter(|attr| attr.path().is_ident("serde")) {
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
    } else if meta.path.is_ident("deny_unknown_fields") {
        target.deny_unknown_fields = true;
    } else if meta.path.is_ident("tag") {
        target.tag = Some(parse_string_assignment(&meta)?);
    } else if meta.path.is_ident("content") {
        target.content = Some(parse_string_assignment(&meta)?);
    } else if meta.path.is_ident("untagged") {
        target.untagged = true;
    } else if meta.path.is_ident("default") {
        target.default = Some(parse_default_assignment(&meta)?);
    } else if meta.path.is_ident("transparent") {
        target.transparent = true;
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
        target.serialize_with = Some(parse_string_assignment(&meta)?);
    } else if meta.path.is_ident("deserialize_with") {
        target.has_deserialize_with = true;
        target.deserialize_with = Some(parse_string_assignment(&meta)?);
    } else if meta.path.is_ident("with") {
        target.has_with = true;
        target.with = Some(parse_string_assignment(&meta)?);
    } else if meta.path.is_ident("other") {
        target.other = true;
    } else if meta.path.is_ident("untagged") {
        target.untagged = true;
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
        target.default = Some(parse_default_assignment(&meta)?);
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
        target.serialize_with = Some(parse_string_assignment(&meta)?);
    } else if meta.path.is_ident("deserialize_with") {
        target.has_deserialize_with = true;
        target.deserialize_with = Some(parse_string_assignment(&meta)?);
    } else if meta.path.is_ident("with") {
        target.has_with = true;
        target.with = Some(parse_string_assignment(&meta)?);
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

fn lower_container_attrs(crate_ref: &TokenStream, attrs: ContainerAttrs) -> TokenStream {
    let key = quote!(#crate_ref::datatype::SERDE_CONTAINER_ATTRIBUTE_KEY);
    let rename_serialize = option_string(&attrs.rename_serialize);
    let rename_deserialize = option_string(&attrs.rename_deserialize);
    let rename_all_serialize = option_rename_rule(crate_ref, attrs.rename_all_serialize);
    let rename_all_deserialize = option_rename_rule(crate_ref, attrs.rename_all_deserialize);
    let rename_all_fields_serialize =
        option_rename_rule(crate_ref, attrs.rename_all_fields_serialize);
    let rename_all_fields_deserialize =
        option_rename_rule(crate_ref, attrs.rename_all_fields_deserialize);
    let tag = option_string(&attrs.tag);
    let content = option_string(&attrs.content);
    let default = option_string(&attrs.default);
    let from = option_conversion(crate_ref, &attrs.from);
    let try_from = option_conversion(crate_ref, &attrs.try_from);
    let into = option_conversion(crate_ref, &attrs.into);
    let deny_unknown_fields = attrs.deny_unknown_fields;
    let untagged = attrs.untagged;
    let transparent = attrs.transparent;
    let variant_identifier = attrs.variant_identifier;
    let field_identifier = attrs.field_identifier;
    let payload = quote!(#crate_ref::datatype::SerdeContainerAttributeData {
        rename_serialize: #rename_serialize,
        rename_deserialize: #rename_deserialize,
        rename_all_serialize: #rename_all_serialize,
        rename_all_deserialize: #rename_all_deserialize,
        rename_all_fields_serialize: #rename_all_fields_serialize,
        rename_all_fields_deserialize: #rename_all_fields_deserialize,
        deny_unknown_fields: #deny_unknown_fields,
        tag: #tag,
        content: #content,
        untagged: #untagged,
        default: #default,
        transparent: #transparent,
        from: #from,
        try_from: #try_from,
        into: #into,
        variant_identifier: #variant_identifier,
        field_identifier: #field_identifier,
    });

    quote!(attrs.insert_named(#key, #payload);)
}

fn lower_variant_attrs(crate_ref: &TokenStream, attrs: VariantAttrs) -> TokenStream {
    let key = quote!(#crate_ref::datatype::SERDE_VARIANT_ATTRIBUTE_KEY);
    let rename_serialize = option_string(&attrs.rename_serialize);
    let rename_deserialize = option_string(&attrs.rename_deserialize);
    let rename_all_serialize = option_rename_rule(crate_ref, attrs.rename_all_serialize);
    let rename_all_deserialize = option_rename_rule(crate_ref, attrs.rename_all_deserialize);
    let serialize_with = option_string(&attrs.serialize_with);
    let deserialize_with = option_string(&attrs.deserialize_with);
    let with = option_string(&attrs.with);
    let skip_serializing = attrs.skip_serializing;
    let skip_deserializing = attrs.skip_deserializing;
    let has_serialize_with = attrs.has_serialize_with;
    let has_deserialize_with = attrs.has_deserialize_with;
    let has_with = attrs.has_with;
    let other = attrs.other;
    let untagged = attrs.untagged;
    let aliases = attrs
        .aliases
        .iter()
        .map(|value| quote!(::std::string::String::from(#value)));
    let payload = quote!(#crate_ref::datatype::SerdeVariantAttributeData {
        rename_serialize: #rename_serialize,
        rename_deserialize: #rename_deserialize,
        aliases: vec![#(#aliases),*],
        rename_all_serialize: #rename_all_serialize,
        rename_all_deserialize: #rename_all_deserialize,
        skip_serializing: #skip_serializing,
        skip_deserializing: #skip_deserializing,
        serialize_with: #serialize_with,
        has_serialize_with: #has_serialize_with,
        deserialize_with: #deserialize_with,
        has_deserialize_with: #has_deserialize_with,
        with: #with,
        has_with: #has_with,
        other: #other,
        untagged: #untagged,
    });

    quote!(attrs.insert_named(#key, #payload);)
}

fn lower_field_attrs(crate_ref: &TokenStream, attrs: FieldAttrs) -> TokenStream {
    let key = quote!(#crate_ref::datatype::SERDE_FIELD_ATTRIBUTE_KEY);
    let rename_serialize = option_string(&attrs.rename_serialize);
    let rename_deserialize = option_string(&attrs.rename_deserialize);
    let default = option_string(&attrs.default);
    let skip_serializing_if = option_string(&attrs.skip_serializing_if);
    let serialize_with = option_string(&attrs.serialize_with);
    let deserialize_with = option_string(&attrs.deserialize_with);
    let with = option_string(&attrs.with);
    let flatten = attrs.flatten;
    let skip_serializing = attrs.skip_serializing;
    let skip_deserializing = attrs.skip_deserializing;
    let has_serialize_with = attrs.has_serialize_with;
    let has_deserialize_with = attrs.has_deserialize_with;
    let has_with = attrs.has_with;
    let aliases = attrs
        .aliases
        .iter()
        .map(|value| quote!(::std::string::String::from(#value)));
    let payload = quote!(#crate_ref::datatype::SerdeFieldAttributeData {
        rename_serialize: #rename_serialize,
        rename_deserialize: #rename_deserialize,
        aliases: vec![#(#aliases),*],
        default: #default,
        flatten: #flatten,
        skip_serializing: #skip_serializing,
        skip_deserializing: #skip_deserializing,
        skip_serializing_if: #skip_serializing_if,
        serialize_with: #serialize_with,
        has_serialize_with: #has_serialize_with,
        deserialize_with: #deserialize_with,
        has_deserialize_with: #has_deserialize_with,
        with: #with,
        has_with: #has_with,
    });

    quote!(attrs.insert_named(#key, #payload);)
}

fn option_string(value: &Option<String>) -> TokenStream {
    match value {
        Some(value) => quote!(Some(::std::string::String::from(#value))),
        None => quote!(None),
    }
}

fn option_rename_rule(crate_ref: &TokenStream, value: Option<RenameRule>) -> TokenStream {
    match value {
        Some(value) => {
            let value = value.to_tokens(crate_ref);
            quote!(Some(#value))
        }
        None => quote!(None),
    }
}

fn option_conversion(crate_ref: &TokenStream, value: &Option<ConversionType>) -> TokenStream {
    match value {
        Some(value) => {
            let type_src = &value.type_src;
            let ty = &value.ty;
            quote!(Some(#crate_ref::datatype::SerdeConversionTypeData {
                type_src: ::std::string::String::from(#type_src),
                resolved: <#ty as #crate_ref::Type>::definition(types),
            }))
        }
        None => quote!(None),
    }
}
