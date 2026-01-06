// TODO: Drop this stuff

use std::borrow::Borrow;
use std::collections::BTreeSet;
use std::{borrow::Cow, fmt};

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

use specta::TypeCollection;

use crate::reserved_names::RESERVED_TYPE_NAMES;
use crate::{Error, Typescript};
use std::fmt::Write;

use specta::datatype::{
    DataType, DeprecatedType, Enum, EnumVariant, Field, Fields, FunctionReturnType, Generic,
    Reference, RuntimeMeta, Struct, Tuple,
};
use specta::internal::{NonSkipField, skip_fields, skip_fields_named};

#[allow(missing_docs)]
pub(crate) type Result<T> = std::result::Result<T, Error>;

pub(crate) type Output = Result<String>;

/// Check if a field has the `#[serde(flatten)]` or `#[specta(flatten)]` attribute
fn is_field_flattened(field: &Field) -> bool {
    field.attributes().iter().any(|attr| {
        if attr.path == "serde" || attr.path == "specta" {
            match &attr.kind {
                RuntimeMeta::Path(path) => path == "flatten",
                RuntimeMeta::List(items) => items.iter().any(|item| {
                    matches!(item, specta::datatype::RuntimeNestedMeta::Meta(RuntimeMeta::Path(path)) if path == "flatten")
                }),
                _ => false,
            }
        } else {
            false
        }
    })
}

#[allow(clippy::ptr_arg)]
fn inner_comments(
    ctx: ExportContext,
    deprecated: Option<&DeprecatedType>,
    docs: &Cow<'static, str>,
    other: String,
    start_with_newline: bool,
    prefix: &str,
) -> String {
    if !ctx.is_export {
        return other;
    }

    let comments = js_doc(docs, deprecated);

    let (prefix_a, prefix_b) = match start_with_newline && !comments.is_empty() {
        true => ("\n", prefix),
        false => ("", ""),
    };

    format!("{prefix_a}{prefix_b}{comments}{other}")
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

    crate::primitives::datatype(s, ctx.cfg, types, typ, vec![], ctx.is_export, None, "")
}

// Can be used with `StructUnnamedFields.fields` or `EnumNamedFields.fields`
fn unnamed_fields_datatype(
    ctx: ExportContext,
    fields: &[NonSkipField],
    types: &TypeCollection,
    s: &mut String,
    prefix: &str,
) -> Result<()> {
    match fields {
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
                    prefix,
                ));
            }

            s.push(']');
        }
    }

    Ok(())
}

pub(crate) fn tuple_datatype(ctx: ExportContext, tuple: &Tuple, types: &TypeCollection) -> Output {
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
    parent_name: Option<&str>,
    strct: &Struct,
    types: &TypeCollection,
    s: &mut String,
    prefix: &str,
) -> Result<()> {
    match &strct.fields() {
        Fields::Unit => s.push_str(NULL),
        Fields::Unnamed(unnamed) => unnamed_fields_datatype(
            ctx,
            &skip_fields(unnamed.fields()).collect::<Vec<_>>(),
            types,
            s,
            prefix,
        )?,
        Fields::Named(named) => {
            let fields = skip_fields_named(named.fields()).collect::<Vec<_>>();

            if fields.is_empty() {
                // Note: tag is now on parent Enum/Struct attributes, not on NamedFields
                // For proper tag support, parent context would need to be passed down
                // For now, we treat all empty named fields as Record<string, never>
                write!(s, "Record<{STRING}, {NEVER}>")?;
                return Ok(());
            }

            let (flattened, non_flattened): (Vec<_>, Vec<_>) =
                fields.iter().partition(|(_, (f, _))| is_field_flattened(f));

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
                            prefix,
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
                        prefix,
                    ))
                })
                .collect::<Result<Vec<_>>>()?;

            // Note: tag is now on parent Enum/Struct attributes, not on NamedFields
            // For proper tag support, parent context would need to be passed down
            // Skipping tag field for now

            if !unflattened_fields.is_empty() {
                let mut s = "{ ".to_string();

                for field in unflattened_fields {
                    // TODO: Inline or not for newline?
                    // s.push_str(&format!("{field}; "));
                    s.push_str(&format!("\n{prefix}\t{field},"));
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
) -> Result<Option<String>> {
    match &variant.fields() {
        // TODO: Remove unreachable in type system
        Fields::Unit => unreachable!("Unit enum variants have no type!"),
        Fields::Named(obj) => {
            // Note: tag is now on parent Enum attributes, not on NamedFields
            // For proper tag support, parent context would need to be passed down
            let mut fields = vec![];

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
                            prefix,
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
    prefix: &str,
) -> Result<()> {
    if e.variants().is_empty() {
        return Ok(write!(s, "{NEVER}")?);
    }

    // Simplified: use External representation only
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
                // Always use External representation
                match &variant.fields() {
                    Fields::Unit => sanitised_name.to_string(),
                    _ => {
                        let ts_values = enum_variant_datatype(
                            ctx.with(PathItem::Variant(variant_name.clone())),
                            types,
                            variant_name.clone(),
                            variant,
                            prefix,
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
                true,
                prefix,
            ))
        })
        .collect::<Result<Vec<_>>>()?;
    variants.dedup();
    s.push_str(&variants.join(" | "));

    Ok(())
}

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
                // Prevent `{ tag: "{tag}" } & Record<string | never>`
                // Note: tag is now on parent Enum attributes, not on NamedFields
                if fields.fields().is_empty() {
                    return Ok(true);
                }

                Ok(false)
            }
        },
        DataType::Enum(v) => {
            // Simplified: treat all enums as External representation (objects)
            Ok(false)
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
            r.get(types)
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

// TODO: Merge this into main expoerter
pub(crate) fn js_doc(docs: &str, deprecated: Option<&DeprecatedType>) -> String {
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

    let mut builder = Builder::default();

    if !docs.is_empty() {
        builder.extend(docs.split('\n'));
    }

    if let Some(deprecated) = deprecated {
        builder.push_deprecated(deprecated);
    }

    builder.build()
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
//             // TODO: Should JS doc support per field or variant comments???
//             is_export: false,
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
