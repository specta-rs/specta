// TODO: Drop this stuff

use std::collections::{BTreeSet, HashSet};
use std::{borrow::Cow, fmt};

pub use crate::inline::inline_and_flatten_ndt;

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
    Type(Cow<'static, str>),
    TypeExtended(Cow<'static, str>, &'static str),
    Field(Cow<'static, str>),
    Variant(Cow<'static, str>),
}

#[derive(Clone)]
pub(crate) struct ExportContext<'a> {
    pub(crate) cfg: &'a Typescript,
    pub(crate) path: Vec<PathItem>,
    // `false` when inline'ing and `true` when exporting as named.
    pub(crate) is_export: bool,
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
                PathItem::Type(v) => v,
                PathItem::TypeExtended(_, loc) => loc,
                PathItem::Field(v) => v,
                PathItem::Variant(v) => v,
            });

            if let Some(next) = path.peek() {
                s.push_str(match next {
                    PathItem::Type(_) => " -> ",
                    PathItem::TypeExtended(_, _) => " -> ",
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

use specta::{SpectaID, TypeCollection};

use crate::reserved_names::RESERVED_TYPE_NAMES;
use crate::{Error, Typescript};
use std::fmt::Write;

use specta::datatype::{
    DataType, DeprecatedType, Enum, EnumRepr, EnumVariant, Fields, FunctionReturnType, Literal,
    NamedDataType, Struct, Tuple,
};
use specta::internal::{skip_fields, skip_fields_named, NonSkipField};

#[allow(missing_docs)]
pub(crate) type Result<T> = std::result::Result<T, Error>;

pub(crate) type Output = Result<String>;

#[allow(clippy::ptr_arg)]
fn inner_comments(
    ctx: ExportContext,
    deprecated: Option<&DeprecatedType>,
    docs: &Cow<'static, str>,
    other: String,
    start_with_newline: bool,
) -> String {
    if !ctx.is_export {
        return other;
    }

    let comments = js_doc_builder(docs, deprecated).build();

    let prefix = match start_with_newline && !comments.is_empty() {
        true => "\n",
        false => "",
    };

    format!("{prefix}{comments}{other}")
}

pub(crate) fn datatype_inner(
    ctx: ExportContext,
    typ: &FunctionReturnType,
    types: &TypeCollection,
    s: &mut String,
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
                    )?;
                    v
                },
                {
                    let mut v = String::new();
                    datatype_inner(ctx, &FunctionReturnType::Value(e.clone()), types, &mut v)?;
                    v
                },
            ];
            variants.dedup();
            s.push_str(&variants.join(" | "));
            return Ok(());
        }
    };

    crate::primitives::datatype(s, ctx.cfg, types, typ, vec![], ctx.is_export, None)
}

// Can be used with `StructUnnamedFields.fields` or `EnumNamedFields.fields`
fn unnamed_fields_datatype(
    ctx: ExportContext,
    fields: &[NonSkipField],
    types: &TypeCollection,
    s: &mut String,
) -> Result<()> {
    Ok(match fields {
        [(field, ty)] => {
            let mut v = String::new();
            datatype_inner(
                ctx.clone(),
                &FunctionReturnType::Value((*ty).clone()),
                types,
                &mut v,
            )?;
            s.push_str(&inner_comments(
                ctx,
                field.deprecated(),
                field.docs(),
                v,
                true,
            ));
        }
        fields => {
            s.push('[');

            for (i, (field, ty)) in fields.iter().enumerate() {
                if i != 0 {
                    s.push_str(", ");
                }

                let mut v = String::new();
                datatype_inner(
                    ctx.clone(),
                    &FunctionReturnType::Value((*ty).clone()),
                    types,
                    &mut v,
                )?;
                s.push_str(&inner_comments(
                    ctx.clone(),
                    field.deprecated(),
                    field.docs(),
                    v,
                    true,
                ));
            }

            s.push(']');
        }
    })
}

pub(crate) fn tuple_datatype(ctx: ExportContext, tuple: &Tuple, types: &TypeCollection) -> Output {
    match &tuple.elements()[..] {
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
    sid: Option<SpectaID>,
    strct: &Struct,
    types: &TypeCollection,
    s: &mut String,
) -> Result<()> {
    Ok(match &strct.fields() {
        Fields::Unit => s.push_str(NULL),
        Fields::Unnamed(unnamed) => unnamed_fields_datatype(
            ctx,
            &skip_fields(unnamed.fields()).collect::<Vec<_>>(),
            types,
            s,
        )?,
        Fields::Named(named) => {
            let fields = skip_fields_named(named.fields()).collect::<Vec<_>>();

            if fields.is_empty() {
                return Ok(match (named.tag().as_ref(), sid) {
                    (Some(tag), Some(sid)) => {
                        let key = types.get(sid).unwrap().name();
                        write!(s, r#"{{ "{tag}": "{key}" }}"#)?
                    }
                    (_, _) => write!(s, "Record<{STRING}, {NEVER}>")?,
                });
            }

            let (flattened, non_flattened): (Vec<_>, Vec<_>) =
                fields.iter().partition(|(_, (f, _))| f.flatten());

            let mut field_sections = flattened
                .into_iter()
                .map(|(key, (field, ty))| {
                    let mut s = String::new();
                    datatype_inner(
                        ctx.with(PathItem::Field(key.clone())),
                        &FunctionReturnType::Value(ty.clone()),
                        types,
                        &mut s,
                    )
                    .map(|_| {
                        inner_comments(
                            ctx.clone(),
                            field.deprecated(),
                            field.docs(),
                            format!("({s})"),
                            true,
                        )
                    })
                })
                .collect::<Result<Vec<_>>>()?;

            let mut unflattened_fields = non_flattened
                .into_iter()
                .map(|(key, field_ref)| {
                    let (field, _) = field_ref;

                    let mut other = String::new();
                    object_field_to_ts(
                        ctx.with(PathItem::Field(key.clone())),
                        key.clone(),
                        field_ref,
                        types,
                        &mut other,
                    )?;

                    Ok(inner_comments(
                        ctx.clone(),
                        field.deprecated(),
                        field.docs(),
                        other,
                        true,
                    ))
                })
                .collect::<Result<Vec<_>>>()?;

            if let (Some(tag), Some(sid)) = (&named.tag(), sid) {
                let key = types.get(sid).unwrap().name();
                unflattened_fields.push(format!("{tag}: \"{key}\""));
            }

            if !unflattened_fields.is_empty() {
                field_sections.push(format!("{{ {} }}", unflattened_fields.join("; ")));
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
    })
}

fn enum_variant_datatype(
    ctx: ExportContext,
    types: &TypeCollection,
    name: Cow<'static, str>,
    variant: &EnumVariant,
) -> Result<Option<String>> {
    match &variant.fields() {
        // TODO: Remove unreachable in type system
        Fields::Unit => unreachable!("Unit enum variants have no type!"),
        Fields::Named(obj) => {
            let mut fields = if let Some(tag) = &obj.tag() {
                let sanitised_name = sanitise_key(name, true);
                vec![format!("{tag}: {sanitised_name}")]
            } else {
                vec![]
            };

            fields.extend(
                skip_fields_named(obj.fields())
                    .map(|(name, field_ref)| {
                        let (field, _) = field_ref;

                        let mut other = String::new();
                        object_field_to_ts(
                            ctx.with(PathItem::Field(name.clone())),
                            name.clone(),
                            field_ref,
                            types,
                            &mut other,
                        )?;

                        Ok(inner_comments(
                            ctx.clone(),
                            field.deprecated(),
                            field.docs(),
                            other,
                            true,
                        ))
                    })
                    .collect::<Result<Vec<_>>>()?,
            );

            Ok(Some(match &fields[..] {
                [] => format!("Record<{STRING}, {NEVER}>").to_string(),
                fields => format!("{{ {} }}", fields.join("; ")),
            }))
        }
        Fields::Unnamed(obj) => {
            let fields = skip_fields(obj.fields())
                .map(|(_, ty)| {
                    let mut s = String::new();
                    datatype_inner(
                        ctx.clone(),
                        &FunctionReturnType::Value(ty.clone()),
                        types,
                        &mut s,
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
) -> Result<()> {
    if e.variants().is_empty() {
        return Ok(write!(s, "{NEVER}")?);
    }

    Ok(match &e.repr().unwrap_or(&EnumRepr::External) {
        EnumRepr::Untagged => {
            let mut variants = e
                .variants()
                .iter()
                .filter(|(_, variant)| !variant.skip())
                .map(|(name, variant)| {
                    Ok(match variant.fields() {
                        Fields::Unit => NULL.to_string(),
                        _ => inner_comments(
                            ctx.clone(),
                            variant.deprecated(),
                            variant.docs(),
                            enum_variant_datatype(
                                ctx.with(PathItem::Variant(name.clone())),
                                types,
                                name.clone(),
                                variant,
                            )?
                            .expect("Invalid Serde type"),
                            true,
                        ),
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            variants.dedup();
            s.push_str(&variants.join(" | "));
        }
        repr => {
            let mut variants = e
                .variants()
                .iter()
                .filter(|(_, variant)| !variant.skip())
                .map(|(variant_name, variant)| {
                    let sanitised_name = sanitise_key(variant_name.clone(), true);

                    Ok(inner_comments(
                        ctx.clone(),
                        variant.deprecated(),
                        variant.docs(),
                        match (repr, &variant.fields()) {
                            (EnumRepr::Untagged, _) => unreachable!(),
                            (EnumRepr::Internal { tag }, Fields::Unit) => {
                                format!("{{ {tag}: {sanitised_name} }}")
                            }
                            (EnumRepr::Internal { tag }, Fields::Unnamed(tuple)) => {
                                let fields = skip_fields(tuple.fields()).collect::<Vec<_>>();

                                // This field is only required for `{ty}` not `[...]` so we only need to check when there one field
                                let dont_join_ty = if tuple.fields().len() == 1 {
                                    let (_, ty) = fields.first().expect("checked length above");
                                    validate_type_for_tagged_intersection(
                                        ctx.clone(),
                                        (**ty).clone(),
                                        types,
                                    )?
                                } else {
                                    false
                                };

                                let mut typ = String::new();

                                unnamed_fields_datatype(ctx.clone(), &fields, types, &mut typ)?;

                                if dont_join_ty {
                                    format!("({{ {tag}: {sanitised_name} }})")
                                } else {
                                    // We wanna be sure `... & ... | ...` becomes `... & (... | ...)`
                                    if typ.contains('|') {
                                        typ = format!("({typ})");
                                    }
                                    format!("({{ {tag}: {sanitised_name} }} & {typ})")
                                }
                            }
                            (EnumRepr::Internal { tag }, Fields::Named(obj)) => {
                                let mut fields = vec![format!("{tag}: {sanitised_name}")];

                                for (name, field) in skip_fields_named(obj.fields()) {
                                    let mut other = String::new();
                                    object_field_to_ts(
                                        ctx.with(PathItem::Field(name.clone())),
                                        name.clone(),
                                        field,
                                        types,
                                        &mut other,
                                    )?;
                                    fields.push(other);
                                }

                                format!("{{ {} }}", fields.join("; "))
                            }
                            (EnumRepr::External, Fields::Unit) => sanitised_name.to_string(),
                            (EnumRepr::External, _) => {
                                let ts_values = enum_variant_datatype(
                                    ctx.with(PathItem::Variant(variant_name.clone())),
                                    types,
                                    variant_name.clone(),
                                    variant,
                                )?;
                                let sanitised_name = sanitise_key(variant_name.clone(), false);

                                match ts_values {
                                    Some(ts_values) => {
                                        format!("{{ {sanitised_name}: {ts_values} }}")
                                    }
                                    None => format!(r#""{sanitised_name}""#),
                                }
                            }
                            (EnumRepr::Adjacent { tag, .. }, Fields::Unit) => {
                                format!("{{ {tag}: {sanitised_name} }}")
                            }
                            (EnumRepr::Adjacent { tag, content }, _) => {
                                let ts_value = enum_variant_datatype(
                                    ctx.with(PathItem::Variant(variant_name.clone())),
                                    types,
                                    variant_name.clone(),
                                    variant,
                                )?;

                                let mut s = String::new();

                                s.push_str("{ ");

                                write!(s, "{tag}: {sanitised_name}")?;
                                if let Some(ts_value) = ts_value {
                                    write!(s, "; {content}: {ts_value}")?;
                                }

                                s.push_str(" }");

                                s
                            }
                            (EnumRepr::String { rename_all }, Fields::Unit) => {
                                // Generate string literal for string enums
                                let string_value = match rename_all.as_deref() {
                                    Some("snake_case") => variant_name.to_lowercase(),
                                    Some("UPPERCASE") => variant_name.to_uppercase(),
                                    Some("camelCase") => {
                                        let mut chars = variant_name.chars();
                                        match chars.next() {
                                            None => String::new(),
                                            Some(first) => {
                                                first.to_lowercase().chain(chars).collect()
                                            }
                                        }
                                    }
                                    Some("PascalCase") => {
                                        let mut chars = variant_name.chars();
                                        match chars.next() {
                                            None => String::new(),
                                            Some(first) => {
                                                first.to_uppercase().chain(chars).collect()
                                            }
                                        }
                                    }
                                    Some("kebab-case") => {
                                        variant_name.to_lowercase().replace('_', "-")
                                    }
                                    _ => variant_name.to_lowercase(),
                                };
                                format!(r#""{string_value}""#)
                            }
                            (EnumRepr::String { .. }, _) => {
                                // String enums should only have unit variants
                                return Err(Error::InvalidName {
                                    path: format!("enum variant '{}'", variant_name),
                                    name: "String enum variants cannot have fields".into(),
                                });
                            }
                        },
                        true,
                    ))
                })
                .collect::<Result<Vec<_>>>()?;
            variants.dedup();
            s.push_str(&variants.join(" | "));
        }
    })
}

// impl std::fmt::Display for LiteralType {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         match self {
//             Self::i8(v) => write!(f, "{v}"),
//             Self::i16(v) => write!(f, "{v}"),
//             Self::i32(v) => write!(f, "{v}"),
//             Self::u8(v) => write!(f, "{v}"),
//             Self::u16(v) => write!(f, "{v}"),
//             Self::u32(v) => write!(f, "{v}"),
//             Self::f32(v) => write!(f, "{v}"),
//             Self::f64(v) => write!(f, "{v}"),
//             Self::bool(v) => write!(f, "{v}"),
//             Self::String(v) => write!(f, r#""{v}""#),
//             Self::char(v) => write!(f, r#""{v}""#),
//             Self::None => f.write_str(NULL),
//         }
//     }
// }

/// convert an object field into a Typescript string
fn object_field_to_ts(
    ctx: ExportContext,
    key: Cow<'static, str>,
    (field, ty): NonSkipField,
    types: &TypeCollection,
    s: &mut String,
) -> Result<()> {
    let field_name_safe = sanitise_key(key, false);

    // https://github.com/oscartbeaumont/rspc/issues/100#issuecomment-1373092211
    let (key, ty) = match field.optional() {
        true => (format!("{field_name_safe}?").into(), ty),
        false => (field_name_safe, ty),
    };

    let mut value = String::new();
    datatype_inner(
        ctx,
        &FunctionReturnType::Value(ty.clone()),
        types,
        &mut value,
    )?;

    Ok(write!(s, "{key}: {value}",)?)
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

    if let Some(first_char) = ident.chars().next() {
        if !first_char.is_alphabetic() && first_char != '_' {
            return Err(Error::InvalidNameLegacy(
                loc,
                ctx.export_path(),
                ident.to_string(),
            ));
        }
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
        DataType::Literal(v) => match v {
            Literal::None => Ok(true),
            _ => Ok(false),
        },
        DataType::Struct(v) => match v.fields() {
            Fields::Unit => Ok(true),
            Fields::Unnamed(_) => {
                Err(Error::InvalidTaggedVariantContainingTupleStructLegacy(
                   ctx.export_path()
                ))
            }
            Fields::Named(fields) => {
                // Prevent `{ tag: "{tag}" } & Record<string | never>`
                if fields.tag().is_none() && fields.fields().is_empty() {
                    return Ok(true);
                }

                Ok(false)
            }
        },
        DataType::Enum(v) => {
            match v.repr().unwrap_or(&EnumRepr::External) {
                EnumRepr::Untagged => {
                    Ok(v.variants().iter().any(|(_, v)| match &v.fields() {
                        // `{ .. } & null` is `never`
                        Fields::Unit => true,
                         // `{ ... } & Record<string, never>` is not useful
                        Fields::Named(v) => v.tag().is_none() && v.fields().is_empty(),
                        Fields::Unnamed(_) => false,
                    }))
                },
                // All of these repr's are always objects.
                EnumRepr::Internal { .. } | EnumRepr::Adjacent { .. } | EnumRepr::External => Ok(false),
                // String enums are string literals, not objects
                EnumRepr::String { .. } => Ok(false),
            }
        }
        DataType::Tuple(v) => {
            // Empty tuple is `null`
            if v.elements().is_empty() {
                return Ok(true);
            }

            Ok(false)
        }
        DataType::Reference(r) => validate_type_for_tagged_intersection(
            ctx,
            types
                .get(r.sid())
                .expect("TypeCollection should have been populated by now")
                .ty()
                .clone(),
            types,
        ),
    }
}

const ANY: &str = "any";
const UNKNOWN: &str = "unknown";
const STRING: &str = "string";
const NULL: &str = "null";
const NEVER: &str = "never";

use std::borrow::Borrow;

use specta::datatype::Generic;

// TODO: Merge this into main expoerter
pub(crate) fn js_doc_builder(docs: &str, deprecated: Option<&DeprecatedType>) -> Builder {
    let mut builder = Builder::default();

    if !docs.is_empty() {
        builder.extend(docs.split('\n'));
    }

    if let Some(deprecated) = deprecated {
        builder.push_deprecated(deprecated);
    }

    builder
}

pub fn typedef_named_datatype(
    cfg: &Typescript,
    typ: &NamedDataType,
    types: &TypeCollection,
) -> Output {
    typedef_named_datatype_inner(
        &ExportContext {
            cfg,
            path: vec![],
            // TODO: Should JS doc support per field or variant comments???
            is_export: false,
        },
        typ,
        types,
    )
}

fn typedef_named_datatype_inner(
    ctx: &ExportContext,
    typ: &NamedDataType,
    types: &TypeCollection,
) -> Output {
    let name = typ.name();
    let docs = typ.docs();
    let deprecated = typ.deprecated();
    let item = typ.ty();

    let ctx = ctx.with(PathItem::Type(name.clone()));

    let name = sanitise_type_name(ctx.clone(), NamedLocation::Type, name)?;

    let mut inline_ts = String::new();
    datatype_inner(
        ctx.clone(),
        &FunctionReturnType::Value(typ.ty().clone()),
        types,
        &mut inline_ts,
    )?;

    let mut builder = js_doc_builder(docs, deprecated);

    typ.generics()
        .into_iter()
        .for_each(|generic| builder.push_generic(generic));

    builder.push_internal(["@typedef { ", &inline_ts, " } ", &name]);

    Ok(builder.build())
}

const START: &str = "/**\n";

pub struct Builder {
    value: String,
}

impl Builder {
    pub fn push(&mut self, line: &str) {
        self.push_internal([line.trim()]);
    }

    pub(crate) fn push_internal<'a>(&mut self, parts: impl IntoIterator<Item = &'a str>) {
        self.value.push_str(" * ");

        for part in parts.into_iter() {
            self.value.push_str(part);
        }

        self.value.push('\n');
    }

    pub fn push_deprecated(&mut self, typ: &DeprecatedType) {
        self.push_internal(
            ["@deprecated"].into_iter().chain(
                match typ {
                    DeprecatedType::DeprecatedWithSince {
                        note: message,
                        since,
                    } => Some((since.as_ref(), message)),
                    _ => None,
                }
                .map(|(since, message)| {
                    [" ", message.trim()].into_iter().chain(
                        since
                            .map(|since| [" since ", since.trim()])
                            .into_iter()
                            .flatten(),
                    )
                })
                .into_iter()
                .flatten(),
            ),
        );
    }

    pub fn push_generic(&mut self, generic: &Generic) {
        self.push_internal(["@template ", generic.borrow()])
    }

    pub fn build(mut self) -> String {
        if self.value == START {
            return String::new();
        }

        self.value.push_str(" */\n");
        self.value
    }
}

impl Default for Builder {
    fn default() -> Self {
        Self {
            value: START.to_string(),
        }
    }
}

impl<T: AsRef<str>> Extend<T> for Builder {
    fn extend<I: IntoIterator<Item = T>>(&mut self, iter: I) {
        for item in iter {
            self.push(item.as_ref());
        }
    }
}
