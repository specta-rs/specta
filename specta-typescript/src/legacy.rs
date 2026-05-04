// TODO: Drop this stuff

use std::{
    borrow::Cow,
    collections::BTreeSet,
    fmt::{self, Write},
};

use specta::{
    Types,
    datatype::{
        DataType, Deprecated, Enum, Field, Fields, GenericReference, NamedReferenceType, Reference,
        Struct, Tuple, Variant,
    },
};

use crate::{Error, Exporter, reserved_names::RESERVED_TYPE_NAMES};

#[derive(Clone, Debug)]
pub(crate) enum PathItem {
    // Type(Cow<'static, str>),
    // TypeExtended(Cow<'static, str>, &'static str),
    Field(Cow<'static, str>),
    Variant(Cow<'static, str>),
}

#[derive(Clone)]
pub(crate) struct ExportContext<'a> {
    pub(crate) cfg: &'a Exporter,
    pub(crate) path: Vec<PathItem>,
}

impl ExportContext<'_> {
    pub(crate) fn with(&self, item: PathItem) -> Self {
        Self {
            path: self.path.iter().cloned().chain([item]).collect(),
            ..*self
        }
    }

    pub(crate) fn export_path(&self) -> ExportPath {
        ExportPath::new(&self.path)
    }
}

/// Represents the path of an error in the export tree.
/// This is designed to be opaque, meaning it's internal format and `Display` impl are subject to change at will.
pub struct ExportPath(String);

impl ExportPath {
    pub(crate) fn new(path: &[PathItem]) -> Self {
        let mut s = String::new();
        let mut path = path.iter().peekable();
        while let Some(item) = path.next() {
            s.push_str(match item {
                // PathItem::Type(v) => v,
                // PathItem::TypeExtended(_, loc) => loc,
                PathItem::Field(v) => v,
                PathItem::Variant(v) => v,
            });

            if let Some(next) = path.peek() {
                s.push_str(match next {
                    // PathItem::Type(_) => " -> ",
                    // PathItem::TypeExtended(_, _) => " -> ",
                    PathItem::Field(_) => ".",
                    PathItem::Variant(_) => "::",
                });
            } else {
                break;
            }
        }

        Self(s)
    }
}

impl PartialEq for ExportPath {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl fmt::Debug for ExportPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl fmt::Display for ExportPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[allow(missing_docs)]
pub(crate) type Result<T> = std::result::Result<T, Error>;

pub(crate) type Output = Result<String>;

#[allow(clippy::ptr_arg)]
fn inner_comments(
    deprecated: Option<&Deprecated>,
    docs: &str,
    other: String,
    start_with_newline: bool,
    prefix: &str,
) -> String {
    let mut comments = String::new();
    js_doc(&mut comments, docs, deprecated);
    if comments.is_empty() {
        return other;
    }

    let mut out = String::new();
    if start_with_newline {
        out.push('\n');
    }

    for line in comments.lines() {
        out.push_str(prefix);
        out.push_str(line);
        out.push('\n');
    }

    out.push_str(&other);
    out
}

pub(crate) fn datatype_inner(
    ctx: ExportContext,
    typ: &DataType,
    types: &Types,
    s: &mut String,
    generics: &[(GenericReference, DataType)],
) -> Result<()> {
    crate::primitives::datatype(s, ctx.cfg, None, types, typ, vec![], None, "", generics)
}

// Can be used with `StructUnnamedFields.fields` or `EnumNamedFields.fields`
fn unnamed_fields_datatype(
    ctx: ExportContext,
    fields: &[(&Field, &DataType)],
    types: &Types,
    s: &mut String,
    prefix: &str,
    generics: &[(GenericReference, DataType)],
    force_inline: bool,
) -> Result<()> {
    match fields {
        [(field, ty)] => {
            let mut v = String::new();
            crate::primitives::datatype_with_inline_attr(
                &mut v,
                ctx.cfg,
                None,
                types,
                ty,
                vec![],
                None,
                "",
                generics,
                force_inline,
            )?;
            s.push_str(&inner_comments(
                field.deprecated.as_ref(),
                &field.docs,
                v,
                true,
                prefix,
            ));
        }
        fields => {
            s.push('[');

            for (i, (field, ty)) in fields.iter().enumerate() {
                if i != 0 {
                    s.push_str(", ");
                }

                let mut v = String::new();
                crate::primitives::datatype_with_inline_attr(
                    &mut v,
                    ctx.cfg,
                    None,
                    types,
                    ty,
                    vec![],
                    None,
                    "",
                    generics,
                    force_inline,
                )?;
                s.push_str(&inner_comments(
                    field.deprecated.as_ref(),
                    &field.docs,
                    v,
                    true,
                    prefix,
                ));
            }

            s.push(']');
        }
    }

    Ok(())
}

pub(crate) fn tuple_datatype(
    ctx: ExportContext,
    tuple: &Tuple,
    types: &Types,
    generics: &[(GenericReference, DataType)],
) -> Output {
    match tuple.elements.as_slice() {
        [] => Ok(NULL.to_string()),
        tys => Ok(format!(
            "[{}]",
            tys.iter()
                .map(|v| {
                    let mut s = String::new();
                    datatype_inner(ctx.clone(), v, types, &mut s, generics).map(|_| s)
                })
                .collect::<Result<Vec<_>>>()?
                .join(", ")
        )),
    }
}

pub(crate) fn struct_datatype(
    ctx: ExportContext,
    strct: &Struct,
    types: &Types,
    s: &mut String,
    prefix: &str,
    generics: &[(GenericReference, DataType)],
) -> Result<()> {
    match &strct.fields {
        Fields::Unit => s.push_str(NULL),
        Fields::Unnamed(unnamed) => unnamed_fields_datatype(
            ctx,
            &unnamed
                .fields
                .iter()
                .filter_map(|field| field.ty.as_ref().map(|ty| (field, ty)))
                .collect::<Vec<_>>(),
            types,
            s,
            prefix,
            generics,
            false,
        )?,
        Fields::Named(named) => {
            let fields = named
                .fields
                .iter()
                .filter_map(|(name, field)| field.ty.as_ref().map(|ty| (name, (field, ty))))
                .collect::<Vec<_>>();

            if fields.is_empty() {
                // TODO: Handle this
                // match (named.tag().as_ref(), parent_name) {
                //     (Some(tag), Some(key)) => write!(s, r#"{{ "{tag}": "{key}" }}"#)?,
                //     (_, _) => write!(s, "Record<{STRING}, {NEVER}>")?,
                // }
                write!(s, "Record<{STRING}, {NEVER}>")?;
                return Ok(());
            }

            let flattened: Vec<(&Cow<'static, str>, (&Field, &DataType))> = Vec::new();
            let non_flattened = fields.clone();

            let mut flattened_sections = flattened
                .into_iter()
                .map(|(_key, (field, ty))| {
                    let mut s = String::new();
                    crate::primitives::datatype_with_inline_attr(
                        &mut s,
                        ctx.cfg,
                        None,
                        types,
                        ty,
                        vec![],
                        None,
                        "",
                        generics,
                        false,
                    )
                    .map(|_| {
                        inner_comments(
                            field.deprecated.as_ref(),
                            &field.docs,
                            format!("({s})"),
                            true,
                            prefix,
                        )
                    })
                })
                .collect::<Result<Vec<_>>>()?;

            let unflattened_fields = non_flattened
                .into_iter()
                .map(|(key, field_ref)| {
                    let (field, ty) = field_ref;
                    let field_prefix = format!("{prefix}\t");

                    let mut other = String::new();
                    object_field_to_ts(
                        ctx.with(PathItem::Field(key.clone())),
                        key.clone(),
                        (field, ty),
                        types,
                        &mut other,
                        generics,
                        &field_prefix,
                        false,
                        None,
                    )?;

                    let docs = field
                        .docs
                        .trim()
                        .is_empty()
                        .then(|| inline_reference_docs(types, (field, ty), false))
                        .flatten()
                        .unwrap_or(&field.docs);

                    Ok(inner_comments(
                        field.deprecated.as_ref(),
                        docs,
                        other,
                        false,
                        &field_prefix,
                    ))
                })
                .collect::<Result<Vec<_>>>()?;

            // TODO: Handle this
            // if let (Some(tag), Some(key)) = (&named.tag(), parent_name) {
            //     unflattened_fields.push(format!("{tag}: \"{key}\""));
            // }

            if !unflattened_fields.is_empty() {
                let mut s = "{".to_string();

                for field in unflattened_fields {
                    s.push('\n');
                    s.push_str(&field);
                    s.push(',');
                }

                s.push('\n');
                s.push_str(prefix);
                s.push('}');
                flattened_sections.insert(0, s);
            }

            // Remove duplicates while preserving source order.
            let mut seen = BTreeSet::new();
            flattened_sections.retain(|section| seen.insert(section.clone()));
            s.push_str(&flattened_sections.join(" & "));
        }
    }

    Ok(())
}

fn enum_variant_datatype(
    ctx: ExportContext,
    types: &Types,
    name: Cow<'static, str>,
    variant: &Variant,
    prefix: &str,
    generics: &[(GenericReference, DataType)],
    ty_override: Option<VariantTypeOverride<'_>>,
) -> Result<Option<String>> {
    match &variant.fields {
        Fields::Unit if name.is_empty() => Err(Error::invalid_name_legacy(
            ctx.export_path(),
            "anonymous unit enum variants cannot be exported to Typescript".to_string(),
        )),
        Fields::Unit => Ok(Some(sanitise_key(name, true).to_string())),
        Fields::Named(_) if name.is_empty() => Err(Error::invalid_name_legacy(
            ctx.export_path(),
            "anonymous named-field enum variants cannot be exported to Typescript".to_string(),
        )),
        Fields::Named(obj) => {
            let all_fields = obj
                .fields
                .iter()
                .filter_map(|(name, field)| field.ty.as_ref().map(|ty| (name, (field, ty))))
                .collect::<Vec<_>>();

            let flattened: Vec<(&Cow<'static, str>, (&Field, &DataType))> = Vec::new();
            let non_flattened = all_fields.clone();

            let field_sections = flattened
                .into_iter()
                .map(|(_key, (field, ty))| {
                    let mut s = String::new();
                    crate::primitives::datatype_with_inline_attr(
                        &mut s,
                        ctx.cfg,
                        None,
                        types,
                        ty,
                        vec![],
                        None,
                        "",
                        generics,
                        false,
                    )
                    .map(|_| {
                        inner_comments(
                            field.deprecated.as_ref(),
                            &field.docs,
                            format!("({s})"),
                            true,
                            prefix,
                        )
                    })
                })
                .collect::<Result<Vec<_>>>()?;

            let mut regular_fields = vec![];
            // TODO
            // let mut regular_fields = if let Some(tag) = &obj.tag() {
            //     let sanitised_name = sanitise_key(name, true);
            //     vec![format!("{tag}: {sanitised_name}")]
            // } else {
            //     vec![]
            // };

            regular_fields.extend(
                non_flattened
                    .into_iter()
                    .map(|(name, field_ref)| {
                        let (field, ty) = field_ref;

                        let mut other = String::new();
                        object_field_to_ts(
                            ctx.with(PathItem::Field(name.clone())),
                            name.clone(),
                            (field, ty),
                            types,
                            &mut other,
                            generics,
                            "",
                            false,
                            ty_override
                                .as_ref()
                                .filter(|override_ty| override_ty.key == name.as_ref())
                                .map(|override_ty| override_ty.ty),
                        )?;

                        let docs = field
                            .docs
                            .trim()
                            .is_empty()
                            .then(|| inline_reference_docs(types, (field, ty), false))
                            .flatten()
                            .unwrap_or(&field.docs);

                        Ok(inner_comments(
                            field.deprecated.as_ref(),
                            docs,
                            other,
                            true,
                            prefix,
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?,
            );

            Ok(Some(match (&field_sections[..], &regular_fields[..]) {
                ([], []) => format!("Record<{STRING}, {NEVER}>").to_string(),
                ([], fields) => format!("{{ {} }}", fields.join("; ")),
                (_, []) => field_sections.join(" & "),
                (_, _) => {
                    let mut sections = vec![format!("{{ {} }}", regular_fields.join("; "))];
                    sections.extend(field_sections);
                    sections.join(" & ")
                }
            }))
        }
        Fields::Unnamed(obj) => {
            let fields = obj
                .fields
                .iter()
                .filter_map(|field| field.ty.as_ref())
                .map(|ty| {
                    let mut s = String::new();
                    crate::primitives::datatype_with_inline_attr(
                        &mut s,
                        ctx.cfg,
                        None,
                        types,
                        ty,
                        vec![],
                        None,
                        "",
                        generics,
                        false,
                    )
                    .map(|_| s)
                })
                .collect::<Result<Vec<_>>>()?;

            Ok(match &fields[..] {
                [] => {
                    // If the actual length is 0, we know `#[serde(skip)]` was not used.
                    if obj.fields.is_empty() {
                        Some("[]".to_string())
                    } else {
                        // We wanna render `{tag}` not `{tag}: {type}` (where `{type}` is what this function returns)
                        None
                    }
                }
                // If the actual length is 1, we know `#[serde(skip)]` was not used.
                [field] if obj.fields.len() == 1 => Some(field.to_string()),
                fields => Some(format!("[{}]", fields.join(", "))),
            })
        }
    }
}

struct EnumVariantOutput {
    value: String,
    strict_keys: Option<BTreeSet<String>>,
}

#[derive(Debug, Clone)]
struct DiscriminatorAnalysis {
    key: String,
    known_literals: Vec<String>,
    fallback_variant_idx: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
struct VariantTypeOverride<'a> {
    key: &'a str,
    ty: &'a str,
}

#[derive(Debug, Clone)]
enum DiscriminatorValue {
    StringLiteral(String),
    String,
}

fn analyze_discriminator(
    variants: &[&(Cow<'static, str>, Variant)],
) -> Option<DiscriminatorAnalysis> {
    if variants.iter().any(|(name, _)| name.is_empty()) {
        return None;
    }

    let mut key = None::<String>;
    let mut known_literals = BTreeSet::new();
    let mut fallback_variant_idx = None;

    for (idx, (_, variant)) in variants.iter().enumerate() {
        let (variant_key, value) = variant_discriminator(variant)?;

        if let Some(expected) = &key {
            if expected != &variant_key {
                return None;
            }
        } else {
            key = Some(variant_key.clone());
        }

        match value {
            DiscriminatorValue::StringLiteral(value) => {
                known_literals.insert(value);
            }
            DiscriminatorValue::String => {
                if fallback_variant_idx.replace(idx).is_some() {
                    return None;
                }
            }
        }
    }

    if known_literals.is_empty() {
        return None;
    }

    Some(DiscriminatorAnalysis {
        key: key.expect("at least one variant when called"),
        known_literals: known_literals.into_iter().collect(),
        fallback_variant_idx,
    })
}

fn variant_discriminator(variant: &Variant) -> Option<(String, DiscriminatorValue)> {
    let Fields::Named(named) = &variant.fields else {
        return None;
    };

    let (name, field) = named.fields.iter().find(|(_, field)| !field.optional)?;
    let ty = field.ty.as_ref()?;

    if matches!(ty, DataType::Primitive(specta::datatype::Primitive::str)) {
        return Some((name.to_string(), DiscriminatorValue::String));
    }

    string_literal_datatype_value(ty)
        .map(|value| (name.to_string(), DiscriminatorValue::StringLiteral(value)))
}

fn string_literal_datatype_value(ty: &DataType) -> Option<String> {
    let DataType::Enum(enm) = ty else {
        return None;
    };

    let mut variants = enm.variants.iter();
    let (name, variant) = variants.next()?;

    if variants.next().is_some() {
        return None;
    }

    if !matches!(&variant.fields, Fields::Unit) {
        return None;
    }

    Some(name.to_string())
}

fn exclude_known_literals_type(literals: &[String]) -> Option<String> {
    if literals.is_empty() {
        return None;
    }

    let known = literals
        .iter()
        .map(|value| format!("\"{}\"", escape_typescript_string_literal(value.as_str())))
        .collect::<Vec<_>>()
        .join(" | ");

    Some(format!("Exclude<string, {known}>"))
}

fn untagged_strict_keys(variant: &Variant) -> Option<BTreeSet<String>> {
    match &variant.fields {
        Fields::Named(obj) => {
            let all_fields = obj
                .fields
                .iter()
                .filter_map(|(name, field)| field.ty.as_ref().map(|ty| (name, (field, ty))))
                .collect::<Vec<_>>();
            Some(
                all_fields
                    .into_iter()
                    .map(|(name, _)| sanitise_key(name.clone(), false).to_string())
                    .collect(),
            )
        }
        _ => None,
    }
}

fn has_anonymous_variant(variants: &[&(Cow<'static, str>, Variant)]) -> bool {
    variants.iter().any(|(name, _)| name.is_empty())
}

fn strictify_enum_variants(variants: &mut [EnumVariantOutput]) {
    let strict_key_universe = variants
        .iter()
        .filter_map(|variant| variant.strict_keys.as_ref())
        .flat_map(|keys| keys.iter().cloned())
        .collect::<BTreeSet<_>>();

    if strict_key_universe.len() < 2 {
        return;
    }

    for variant in variants {
        let Some(keys) = variant.strict_keys.as_ref() else {
            continue;
        };

        let missing_keys = strict_key_universe
            .iter()
            .filter(|key| !keys.contains(*key))
            .map(|key| format!("{key}?: {NEVER}"))
            .collect::<Vec<_>>();

        if missing_keys.is_empty() {
            continue;
        }

        variant.value = format!("({}) & {{ {} }}", variant.value, missing_keys.join("; "));
    }
}

pub(crate) fn enum_datatype(
    ctx: ExportContext,
    e: &Enum,
    types: &Types,
    s: &mut String,
    prefix: &str,
    generics: &[(GenericReference, DataType)],
) -> Result<()> {
    if e.variants.is_empty() {
        return Ok(write!(s, "{NEVER}")?);
    }

    let filtered_variants = e
        .variants
        .iter()
        .filter(|(_, variant)| !variant.skip)
        .collect::<Vec<_>>();

    let discriminator = analyze_discriminator(&filtered_variants);
    let fallback_override = discriminator.as_ref().and_then(|discriminator| {
        discriminator.fallback_variant_idx.and_then(|idx| {
            exclude_known_literals_type(&discriminator.known_literals)
                .map(|ty| (idx, discriminator.key.as_str(), ty))
        })
    });

    let mut rendered_variants = Vec::with_capacity(filtered_variants.len());
    for (idx, (variant_name, variant)) in filtered_variants.iter().enumerate() {
        let variant_override = fallback_override
            .as_ref()
            .and_then(|(fallback_idx, key, ty)| {
                if *fallback_idx == idx {
                    Some(VariantTypeOverride {
                        key,
                        ty: ty.as_str(),
                    })
                } else {
                    None
                }
            });

        let ts_values = enum_variant_datatype(
            ctx.with(PathItem::Variant(variant_name.clone())),
            types,
            variant_name.clone(),
            variant,
            prefix,
            generics,
            variant_override,
        )?;

        rendered_variants.push(EnumVariantOutput {
            value: ts_values.unwrap_or_else(|| NEVER.to_string()),
            strict_keys: untagged_strict_keys(variant),
        });
    }

    if discriminator.is_none() && !has_anonymous_variant(&filtered_variants) {
        strictify_enum_variants(&mut rendered_variants);
    }

    let mut variants = filtered_variants
        .into_iter()
        .zip(rendered_variants)
        .map(|((_, variant), rendered)| {
            inner_comments(
                variant.deprecated.as_ref(),
                &variant.docs,
                rendered.value,
                true,
                prefix,
            )
        })
        .collect::<Vec<_>>();

    let mut seen = BTreeSet::new();
    variants.retain(|variant| seen.insert(variant.clone()));

    // If all variants are skipped, the enum has no valid values
    if variants.is_empty() {
        s.push_str(NEVER);
    } else {
        s.push_str(&variants.join(" | "));
    }

    Ok(())
}

/// convert an object field into a Typescript string
fn object_field_to_ts(
    ctx: ExportContext,
    key: Cow<'static, str>,
    field_ref: (&Field, &DataType),
    types: &Types,
    s: &mut String,
    generics: &[(GenericReference, DataType)],
    prefix: &str,
    force_inline: bool,
    ty_override: Option<&str>,
) -> Result<()> {
    let (field, ty) = field_ref;
    let field_name_safe = sanitise_key(key, false);

    // https://github.com/specta-rs/rspc/issues/100#issuecomment-1373092211
    let (key, ty) = match field.optional {
        true => (format!("{field_name_safe}?").into(), ty),
        false => (field_name_safe, ty),
    };

    let value = match ty_override {
        Some(ty_override) => ty_override.to_string(),
        None => {
            let mut value = String::new();
            crate::primitives::datatype_with_inline_attr(
                &mut value,
                ctx.cfg,
                None,
                types,
                ty,
                vec![],
                None,
                prefix,
                generics,
                force_inline,
            )?;
            value
        }
    };

    Ok(write!(s, "{prefix}{key}: {value}",)?)
}

fn inline_reference_docs<'a>(
    types: &'a Types,
    (_field, ty): (&Field, &'a DataType),
    force_inline: bool,
) -> Option<&'a str> {
    let DataType::Reference(Reference::Named(r)) = ty else {
        return None;
    };

    if !force_inline {
        return None;
    }

    match &r.inner {
        NamedReferenceType::Reference { .. } => types
            .get(r)
            .filter(|ndt| !ndt.docs.trim().is_empty())
            .map(|ndt| ndt.docs.as_ref()),
        NamedReferenceType::Inline { .. } | NamedReferenceType::Recursive => None,
    }
}

/// sanitise a string to be a valid Typescript key
fn sanitise_key<'a>(field_name: Cow<'static, str>, force_string: bool) -> Cow<'a, str> {
    let valid = is_identifier(&field_name);

    if force_string || !valid {
        format!(r#""{}""#, escape_typescript_string_literal(&field_name)).into()
    } else {
        field_name
    }
}

pub(crate) fn is_identifier(name: &str) -> bool {
    let mut chars = name.chars();
    let Some(first) = chars.next() else {
        return false;
    };

    (first.is_ascii_alphabetic() || first == '_' || first == '$')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '$')
}

pub(crate) fn escape_typescript_string_literal(value: &str) -> Cow<'_, str> {
    if !value.chars().any(|ch| {
        ch == '"' || ch == '\\' || ch == '\u{2028}' || ch == '\u{2029}' || ch.is_control()
    }) {
        return Cow::Borrowed(value);
    }

    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '"' => escaped.push_str(r#"\""#),
            '\\' => escaped.push_str(r#"\\"#),
            '\n' => escaped.push_str(r#"\n"#),
            '\r' => escaped.push_str(r#"\r"#),
            '\t' => escaped.push_str(r#"\t"#),
            '\u{2028}' => escaped.push_str(r#"\u2028"#),
            '\u{2029}' => escaped.push_str(r#"\u2029"#),
            ch if ch.is_control() => {
                write!(escaped, r#"\u{:04X}"#, ch as u32).expect("infallible");
            }
            _ => escaped.push(ch),
        }
    }

    Cow::Owned(escaped)
}

pub(crate) fn sanitise_type_name(ctx: ExportContext, ident: &str) -> Output {
    if let Some(name) = RESERVED_TYPE_NAMES.iter().find(|v| **v == ident) {
        return Err(Error::forbidden_name_legacy(ctx.export_path(), name));
    }

    if let Some(first_char) = ident.chars().next()
        && !first_char.is_alphabetic()
        && first_char != '_'
    {
        return Err(Error::invalid_name_legacy(
            ctx.export_path(),
            ident.to_string(),
        ));
    }

    if ident
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .is_some()
    {
        return Err(Error::invalid_name_legacy(
            ctx.export_path(),
            ident.to_string(),
        ));
    }

    Ok(ident.to_string())
}

const STRING: &str = "string";
const NULL: &str = "null";
const NEVER: &str = "never";

// TODO: Merge this into main expoerter
pub(crate) fn js_doc(s: &mut String, docs: &str, deprecated: Option<&Deprecated>) {
    // Early return - no-op if nothing to document
    if docs.is_empty() && deprecated.is_none() {
        return;
    }

    if deprecated.is_none() {
        let mut lines = docs.lines();
        if let (Some(line), None) = (lines.next(), lines.next()) {
            s.push_str("/** ");
            s.push_str(&escape_jsdoc_text(line));
            s.push_str(" */\n");
            return;
        }
    }

    // Start JSDoc comment
    s.push_str("/**\n");

    // Add documentation lines
    if !docs.is_empty() {
        for line in docs.lines() {
            s.push_str(" * ");
            s.push_str(&escape_jsdoc_text(line));
            s.push('\n');
        }
    }

    // Add @deprecated tag if present
    if let Some(typ) = deprecated {
        s.push_str(" * @deprecated");

        if let Some(details) = deprecated_details(typ) {
            s.push(' ');
            s.push_str(&details);
        }

        s.push('\n');
    }

    // Close JSDoc comment
    s.push_str(" */\n");
}

pub(crate) fn escape_jsdoc_text(text: &str) -> Cow<'_, str> {
    if text.contains("*/") {
        Cow::Owned(text.replace("*/", "*\\/"))
    } else {
        Cow::Borrowed(text)
    }
}

pub(crate) fn deprecated_details(typ: &Deprecated) -> Option<String> {
    let note = typ.note.as_deref().map(str::trim).filter(|v| !v.is_empty());
    let since: Option<&str> = None;

    match (note, since) {
        (Some(note), Some(since)) => Some(format!("{note} since {since}")),
        (Some(note), None) => Some(note.to_string()),
        (None, Some(since)) => Some(format!("since {since}")),
        (None, None) => None,
    }
}

// pub fn typedef_named_datatype(
//     cfg: &Typescript,
//     typ: &NamedDataType,
//     types: &Types,
// ) -> Output {
//     typedef_named_datatype_inner(
//         &ExportContext {
//             cfg,
//             path: vec![],
//         },
//         typ,
//         types,
//     )
// }

// fn typedef_named_datatype_inner(
//     ctx: &ExportContext,
//     typ: &NamedDataType,
//     types: &Types,
// ) -> Output {
//     let name = typ.name();
//     let docs = typ.docs();
//     let deprecated = typ.deprecated();
//     let item = typ.ty();

//     let ctx = ctx.with(PathItem::Type(name.clone()));

//     let name = sanitise_type_name(ctx.clone(), name)?;

//     let mut inline_ts = String::new();
//     datatype_inner(
//         ctx.clone(),
//         &FunctionReturnType::Value(typ.ty().clone()),
//         types,
//         &mut inline_ts,
//     )?;

//     let mut builder = js_doc_builder(docs, deprecated);

//     typ.generics()()
//         .into_iter()
//         .for_each(|generic| builder.push_generic(generic));

//     builder.push_internal(["@typedef { ", &inline_ts, " } ", &name]);

//     Ok(builder.build())
// }
