// TODO: Drop this stuff

use std::{
    borrow::Cow,
    collections::BTreeSet,
    fmt::{self, Write},
};

use specta::{
    TypeCollection,
    datatype::{
        DataType, DeprecatedType, Enum, EnumVariant, Fields, FunctionReturnType, Generic,
        NonSkipField, Reference, Struct, Tuple, skip_fields, skip_fields_named,
    },
};

use crate::{Error, Exporter, reserved_names::RESERVED_TYPE_NAMES};

/// Describes where an error occurred.
#[derive(Debug, PartialEq)]
pub enum NamedLocation {
    Type,
    Field,
    Variant,
}

impl fmt::Display for NamedLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Type => write!(f, "type"),
            Self::Field => write!(f, "field"),
            Self::Variant => write!(f, "variant"),
        }
    }
}

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

    #[doc(hidden)]
    pub fn new_unsafe(path: &str) -> Self {
        Self(path.to_string())
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
    deprecated: Option<&DeprecatedType>,
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
    typ: &FunctionReturnType,
    types: &TypeCollection,
    s: &mut String,
    generics: &[(Generic, DataType)],
) -> Result<()> {
    let typ = match typ {
        FunctionReturnType::Value(t) => t,
        FunctionReturnType::Result(t, e) => {
            let mut variants = vec![
                {
                    let mut v = String::new();
                    datatype_inner(
                        ctx.clone(),
                        &FunctionReturnType::Value(t.clone()),
                        types,
                        &mut v,
                        generics,
                    )?;
                    v
                },
                {
                    let mut v = String::new();
                    datatype_inner(
                        ctx,
                        &FunctionReturnType::Value(e.clone()),
                        types,
                        &mut v,
                        generics,
                    )?;
                    v
                },
            ];
            variants.dedup();
            s.push_str(&variants.join(" | "));
            return Ok(());
        }
    };

    crate::primitives::datatype(s, ctx.cfg, types, typ, vec![], None, "", generics)
}

// Can be used with `StructUnnamedFields.fields` or `EnumNamedFields.fields`
fn unnamed_fields_datatype(
    ctx: ExportContext,
    fields: &[NonSkipField],
    types: &TypeCollection,
    s: &mut String,
    prefix: &str,
    generics: &[(Generic, DataType)],
    force_inline: bool,
) -> Result<()> {
    match fields {
        [(field, ty)] => {
            let mut v = String::new();
            crate::primitives::datatype_with_inline_attr(
                &mut v,
                ctx.cfg,
                types,
                ty,
                vec![],
                None,
                "",
                generics,
                force_inline || field.inline(),
            )?;
            s.push_str(&inner_comments(
                field.deprecated(),
                field.docs(),
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
                    types,
                    ty,
                    vec![],
                    None,
                    "",
                    generics,
                    force_inline || field.inline(),
                )?;
                s.push_str(&inner_comments(
                    field.deprecated(),
                    field.docs(),
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
    types: &TypeCollection,
    generics: &[(Generic, DataType)],
) -> Output {
    match &tuple.elements() {
        [] => Ok(NULL.to_string()),
        tys => Ok(format!(
            "[{}]",
            tys.iter()
                .map(|v| {
                    let mut s = String::new();
                    datatype_inner(
                        ctx.clone(),
                        &FunctionReturnType::Value(v.clone()),
                        types,
                        &mut s,
                        generics,
                    )
                    .map(|_| s)
                })
                .collect::<Result<Vec<_>>>()?
                .join(", ")
        )),
    }
}

pub(crate) fn struct_datatype(
    ctx: ExportContext,
    _parent_name: Option<&str>,
    strct: &Struct,
    types: &TypeCollection,
    s: &mut String,
    prefix: &str,
    generics: &[(Generic, DataType)],
) -> Result<()> {
    match &strct.fields() {
        Fields::Unit => s.push_str(NULL),
        Fields::Unnamed(unnamed) => unnamed_fields_datatype(
            ctx,
            &skip_fields(unnamed.fields()).collect::<Vec<_>>(),
            types,
            s,
            prefix,
            generics,
            false,
        )?,
        Fields::Named(named) => {
            let fields = skip_fields_named(named.fields()).collect::<Vec<_>>();

            if fields.is_empty() {
                // TODO: Handle this
                // match (named.tag().as_ref(), parent_name) {
                //     (Some(tag), Some(key)) => write!(s, r#"{{ "{tag}": "{key}" }}"#)?,
                //     (_, _) => write!(s, "Record<{STRING}, {NEVER}>")?,
                // }
                write!(s, "Record<{STRING}, {NEVER}>")?;
                return Ok(());
            }

            let (flattened, non_flattened): (Vec<_>, Vec<_>) = fields
                .iter()
                .partition(|(_, (f, _))| specta_serde::is_field_flattened(f));

            let mut field_sections = flattened
                .into_iter()
                .map(|(_key, (field, ty))| {
                    let mut s = String::new();
                    crate::primitives::datatype_with_inline_attr(
                        &mut s,
                        ctx.cfg,
                        types,
                        ty,
                        vec![],
                        None,
                        "",
                        generics,
                        field.inline(),
                    )
                    .map(|_| {
                        inner_comments(
                            field.deprecated(),
                            field.docs(),
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
                    )?;

                    let docs = field
                        .docs()
                        .trim()
                        .is_empty()
                        .then(|| inline_reference_docs(types, (field, ty), false))
                        .flatten()
                        .unwrap_or(field.docs());

                    Ok(inner_comments(
                        field.deprecated(),
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
                field_sections.push(s);
            }

            // TODO: Do this more efficiently
            let field_sections = field_sections
                .into_iter()
                // Remove duplicates + sort
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            s.push_str(&field_sections.join(" & "));
        }
    }

    Ok(())
}

fn enum_variant_datatype(
    ctx: ExportContext,
    types: &TypeCollection,
    name: Cow<'static, str>,
    variant: &EnumVariant,
    prefix: &str,
    generics: &[(Generic, DataType)],
) -> Result<Option<String>> {
    match &variant.fields() {
        Fields::Unit => Ok(Some(sanitise_key(name, false).to_string())),
        Fields::Named(obj) => {
            let all_fields = skip_fields_named(obj.fields()).collect::<Vec<_>>();

            let (flattened, non_flattened): (Vec<_>, Vec<_>) = all_fields
                .iter()
                .partition(|(_, (f, _))| specta_serde::is_field_flattened(f));

            let mut field_sections = flattened
                .into_iter()
                .map(|(_key, (field, ty))| {
                    let mut s = String::new();
                    crate::primitives::datatype_with_inline_attr(
                        &mut s,
                        ctx.cfg,
                        types,
                        ty,
                        vec![],
                        None,
                        "",
                        generics,
                        field.inline(),
                    )
                    .map(|_| {
                        inner_comments(
                            field.deprecated(),
                            field.docs(),
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
                        )?;

                        let docs = field
                            .docs()
                            .trim()
                            .is_empty()
                            .then(|| inline_reference_docs(types, (field, ty), false))
                            .flatten()
                            .unwrap_or(field.docs());

                        Ok(inner_comments(
                            field.deprecated(),
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
                    field_sections.push(format!("{{ {} }}", regular_fields.join("; ")));
                    field_sections.join(" & ")
                }
            }))
        }
        Fields::Unnamed(obj) => {
            let fields = skip_fields(obj.fields())
                .map(|(field, ty)| {
                    let mut s = String::new();
                    crate::primitives::datatype_with_inline_attr(
                        &mut s,
                        ctx.cfg,
                        types,
                        ty,
                        vec![],
                        None,
                        "",
                        generics,
                        field.inline(),
                    )
                    .map(|_| s)
                })
                .collect::<Result<Vec<_>>>()?;

            Ok(match &fields[..] {
                [] => {
                    // If the actual length is 0, we know `#[serde(skip)]` was not used.
                    if obj.fields().is_empty() {
                        Some("[]".to_string())
                    } else {
                        // We wanna render `{tag}` not `{tag}: {type}` (where `{type}` is what this function returns)
                        None
                    }
                }
                // If the actual length is 1, we know `#[serde(skip)]` was not used.
                [field] if obj.fields().len() == 1 => Some(field.to_string()),
                fields => Some(format!("[{}]", fields.join(", "))),
            })
        }
    }
}

pub(crate) fn enum_datatype(
    ctx: ExportContext,
    e: &Enum,
    types: &TypeCollection,
    s: &mut String,
    prefix: &str,
    generics: &[(Generic, DataType)],
) -> Result<()> {
    if e.variants().is_empty() {
        return Ok(write!(s, "{NEVER}")?);
    }

    // After specta_serde::apply, enum tagging is already applied to the variant fields
    // So we can treat all enums the same way - just export the variant fields as-is
    let repr = specta_serde::get_enum_repr(e.attributes());

    let mut variants = e
        .variants()
        .iter()
        .filter(|(_, variant)| !variant.skip())
        .map(|(variant_name, variant)| {
            Ok(inner_comments(
                variant.deprecated(),
                variant.docs(),
                match &repr {
                    // For External and Untagged, handle as before
                    specta_serde::EnumRepr::External => match &variant.fields() {
                        Fields::Unit => {
                            let sanitised_name = sanitise_key(variant_name.clone(), true);
                            sanitised_name.to_string()
                        }
                        _ => {
                            let ts_values = enum_variant_datatype(
                                ctx.with(PathItem::Variant(variant_name.clone())),
                                types,
                                variant_name.clone(),
                                variant,
                                prefix,
                                generics,
                            )?;
                            let sanitised_name = sanitise_key(variant_name.clone(), false);

                            match ts_values {
                                Some(ts_values) => {
                                    format!("{{ {sanitised_name}: {ts_values} }}")
                                }
                                None => format!(r#""{sanitised_name}""#),
                            }
                        }
                    },
                    specta_serde::EnumRepr::Untagged => match &variant.fields() {
                        Fields::Unit => NULL.to_string(),
                        _ => {
                            let ts_values = enum_variant_datatype(
                                ctx.with(PathItem::Variant(variant_name.clone())),
                                types,
                                variant_name.clone(),
                                variant,
                                prefix,
                                generics,
                            )?;

                            ts_values.unwrap_or_else(|| "never".to_string())
                        }
                    },
                    specta_serde::EnumRepr::String { .. } => {
                        let sanitised_name = sanitise_key(variant_name.clone(), true);
                        sanitised_name.to_string()
                    }
                    // For Internal and Adjacent, the tag/content fields are already in the variant
                    // So we can just export them as named fields
                    specta_serde::EnumRepr::Internal { .. }
                    | specta_serde::EnumRepr::Adjacent { .. } => {
                        let ts_values = enum_variant_datatype(
                            ctx.with(PathItem::Variant(variant_name.clone())),
                            types,
                            variant_name.clone(),
                            variant,
                            prefix,
                            generics,
                        )?;

                        let ts_values = ts_values.unwrap_or_else(|| "never".to_string());

                        // `primitives::{export, inline}` can be called on untransformed `DataType`s.
                        // In that mode, an internally tagged enum variant with an inlined untagged
                        // enum of unit variants can collapse to `null` (eg `InternallyTaggedM`).
                        // Preserve the internal tag wrapper for this case so the output shape
                        // remains `{ tag: "Variant" }`.
                        if matches!(repr, specta_serde::EnumRepr::Internal { .. })
                            && ts_values == NULL
                            && matches!(
                                variant.fields(),
                                Fields::Unnamed(unnamed)
                                    if unnamed.fields().iter().any(|field| field.inline())
                            )
                        {
                            if let specta_serde::EnumRepr::Internal { tag } = &repr {
                                let tag = sanitise_key(tag.clone().into(), false);
                                let variant_name = sanitise_key(variant_name.clone(), true);
                                format!("{{ {tag}: {variant_name} }}")
                            } else {
                                unreachable!("matched above")
                            }
                        } else {
                            ts_values
                        }
                    }
                },
                true,
                prefix,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    variants.dedup();

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
    field_ref: NonSkipField,
    types: &TypeCollection,
    s: &mut String,
    generics: &[(Generic, DataType)],
    prefix: &str,
    force_inline: bool,
) -> Result<()> {
    let (field, ty) = field_ref;
    let field_name_safe = sanitise_key(key, false);

    // https://github.com/specta-rs/rspc/issues/100#issuecomment-1373092211
    let (key, ty) = match field.optional() {
        true => (format!("{field_name_safe}?").into(), ty),
        false => (field_name_safe, ty),
    };

    let mut value = String::new();
    crate::primitives::datatype_with_inline_attr(
        &mut value,
        ctx.cfg,
        types,
        ty,
        vec![],
        None,
        prefix,
        generics,
        force_inline || field.inline(),
    )?;

    Ok(write!(s, "{prefix}{key}: {value}",)?)
}

fn inline_reference_docs<'a>(
    types: &'a TypeCollection,
    (field, ty): NonSkipField,
    force_inline: bool,
) -> Option<&'a str> {
    let DataType::Reference(Reference::Named(r)) = ty else {
        return None;
    };

    if !(force_inline || field.inline() || r.inline()) {
        return None;
    }

    r.get(types)
        .filter(|ndt| !ndt.docs().trim().is_empty())
        .map(|ndt| ndt.docs().as_ref())
}

/// sanitise a string to be a valid Typescript key
fn sanitise_key<'a>(field_name: Cow<'static, str>, force_string: bool) -> Cow<'a, str> {
    let valid = field_name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
        && field_name
            .chars()
            .next()
            .map(|first| !first.is_numeric())
            .unwrap_or(true);

    if force_string || !valid {
        format!(r#""{field_name}""#).into()
    } else {
        field_name
    }
}

pub(crate) fn sanitise_type_name(ctx: ExportContext, loc: NamedLocation, ident: &str) -> Output {
    if let Some(name) = RESERVED_TYPE_NAMES.iter().find(|v| **v == ident) {
        return Err(Error::ForbiddenNameLegacy(loc, ctx.export_path(), name));
    }

    if let Some(first_char) = ident.chars().next()
        && !first_char.is_alphabetic()
        && first_char != '_'
    {
        return Err(Error::InvalidNameLegacy(
            loc,
            ctx.export_path(),
            ident.to_string(),
        ));
    }

    if ident
        .find(|c: char| !c.is_alphanumeric() && c != '_')
        .is_some()
    {
        return Err(Error::InvalidNameLegacy(
            loc,
            ctx.export_path(),
            ident.to_string(),
        ));
    }

    Ok(ident.to_string())
}

#[allow(dead_code)]
fn validate_type_for_tagged_intersection(
    ctx: ExportContext,
    ty: DataType,
    types: &TypeCollection,
) -> Result<bool> {
    match ty {
        | DataType::Primitive(_)
        // `T & null` is `never` but `T & (U | null)` (this variant) is `T & U` so it's fine.
        | DataType::Nullable(_)
        | DataType::List(_)
        | DataType::Map(_)
        | DataType::Generic(_) => Ok(false),
        // DataType::Literal(v) => match v {
        //     Literal::None => Ok(true),
        //     _ => Ok(false),
        // },
        DataType::Struct(v) => match v.fields() {
            Fields::Unit => Ok(true),
            Fields::Unnamed(_) => {
                Err(Error::InvalidTaggedVariantContainingTupleStructLegacy(
                   ctx.export_path()
                ))
            }
            Fields::Named(fields) => {
                // TODO
                // if fields.tag().is_none() && fields.fields().is_empty() {
                //     return Ok(true);
                // }

                // Prevent `{ tag: "{tag}" } & Record<string | never>`
                // Note: tag is now on parent Enum attributes, not on NamedFields
                if fields.fields().is_empty() {
                    return Ok(true);
                }

                Ok(false)
            }
        },
        DataType::Enum(_v) => {
            // Simplified: treat all enums as External representation (objects)
            Ok(false)
        }
        // TODO
        // DataType::Enum(v) => {
        //     match v.repr().unwrap_or(&EnumRepr::External) {
        //         EnumRepr::Untagged => {
        //             Ok(v.variants().iter().any(|(_, v)| match &v.fields() {
        //                 // `{ .. } & null` is `never`
        //                 Fields::Unit => true,
        //                     // `{ ... } & Record<string, never>` is not useful
        //                 Fields::Named(v) => v.tag().is_none() && v.fields().is_empty(),
        //                 Fields::Unnamed(_) => false,
        //             }))
        //         },
        //         // All of these repr's are always objects.
        //         EnumRepr::Internal { .. } | EnumRepr::Adjacent { .. } | EnumRepr::External => Ok(false),
        //         // String enums are string literals, not objects
        //         EnumRepr::String { .. } => Ok(false),
        //     }
        // }
        DataType::Tuple(v) => {
            // Empty tuple is `null`
            if v.elements().is_empty() {
                return Ok(true);
            }

            Ok(false)
        }
        DataType::Reference(Reference::Named(r)) => validate_type_for_tagged_intersection(
            ctx,
            r.get(types)
                .ok_or_else(|| Error::DanglingNamedReference {
                    reference: format!("{r:?}"),
                })?
                .ty()
                .clone(),
            types,
        ),
        DataType::Reference(Reference::Opaque(_)) => Ok(true),
    }
}

const STRING: &str = "string";
const NULL: &str = "null";
const NEVER: &str = "never";

// TODO: Merge this into main expoerter
pub(crate) fn js_doc(s: &mut String, docs: &str, deprecated: Option<&DeprecatedType>) {
    // Early return - no-op if nothing to document
    if docs.is_empty() && deprecated.is_none() {
        return;
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

pub(crate) fn deprecated_details(typ: &DeprecatedType) -> Option<String> {
    match typ {
        DeprecatedType::Deprecated => None,
        DeprecatedType::DeprecatedWithSince { note, since } => {
            let note = note.trim();
            let since = since.as_ref().map(|v| v.trim()).filter(|v| !v.is_empty());

            match (note.is_empty(), since) {
                (false, Some(since)) => Some(format!("{note} since {since}")),
                (false, None) => Some(note.to_string()),
                (true, Some(since)) => Some(format!("since {since}")),
                (true, None) => None,
            }
        }
        _ => None,
    }
}

// pub fn typedef_named_datatype(
//     cfg: &Typescript,
//     typ: &NamedDataType,
//     types: &TypeCollection,
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
//     types: &TypeCollection,
// ) -> Output {
//     let name = typ.name();
//     let docs = typ.docs();
//     let deprecated = typ.deprecated();
//     let item = typ.ty();

//     let ctx = ctx.with(PathItem::Type(name.clone()));

//     let name = sanitise_type_name(ctx.clone(), NamedLocation::Type, name)?;

//     let mut inline_ts = String::new();
//     datatype_inner(
//         ctx.clone(),
//         &FunctionReturnType::Value(typ.ty().clone()),
//         types,
//         &mut inline_ts,
//     )?;

//     let mut builder = js_doc_builder(docs, deprecated);

//     typ.generics()
//         .into_iter()
//         .for_each(|generic| builder.push_generic(generic));

//     builder.push_internal(["@typedef { ", &inline_ts, " } ", &name]);

//     Ok(builder.build())
// }
