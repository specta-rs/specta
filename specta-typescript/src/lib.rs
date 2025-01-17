//! [TypeScript](https://www.typescriptlang.org) language exporter.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![doc(
    html_logo_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png",
    html_favicon_url = "https://github.com/oscartbeaumont/specta/raw/main/.github/logo-128.png"
)]

use std::borrow::Cow;
use std::fmt::Write;

pub mod comments;
mod context;
mod error;
pub mod formatter;
pub mod js_doc;
mod reserved_terms;
mod typescript;

pub use context::*;
pub use error::*;
use reserved_terms::*;
pub use typescript::*;

use specta::datatype::{
    DataType, DeprecatedType, EnumRepr, EnumType, EnumVariant, FunctionResultVariant,
    LiteralType, NamedDataType, PrimitiveType, Fields, StructType, TupleType,
};
use specta::{
    internal::{detect_duplicate_type_names, skip_fields, skip_fields_named, NonSkipField},
    Generics, NamedType, Type, TypeCollection,
};
use specta_serde::is_valid_ty;

#[allow(missing_docs)]
pub type Result<T> = std::result::Result<T, ExportError>;

pub(crate) type Output = Result<String>;

/// Convert a type which implements [`Type`] to a TypeScript string with an export.
///
/// Eg. `export type Foo = { demo: string; };`
pub fn export_ref<T: NamedType>(_: &T, conf: &Typescript) -> Output {
    export::<T>(conf)
}

/// Convert a type which implements [`Type`] to a TypeScript string with an export.
///
/// Eg. `export type Foo = { demo: string; };`
pub fn export<T: NamedType>(conf: &Typescript) -> Output {
    let mut type_map = TypeCollection::default();
    let named_data_type = T::definition_named_data_type(&mut type_map);
    is_valid_ty(&named_data_type.inner, &type_map)?;
    let result = export_named_datatype(conf, &named_data_type, &type_map);

    if let Some((ty_name, l0, l1)) = detect_duplicate_type_names(&type_map).into_iter().next() {
        return Err(ExportError::DuplicateTypeName(ty_name, l0, l1));
    }

    result
}

/// Convert a type which implements [`Type`] to a TypeScript string.
///
/// Eg. `{ demo: string; };`
pub fn inline_ref<T: Type>(_: &T, conf: &Typescript) -> Output {
    inline::<T>(conf)
}

/// Convert a type which implements [`Type`] to a TypeScript string.
///
/// Eg. `{ demo: string; };`
pub fn inline<T: Type>(conf: &Typescript) -> Output {
    let mut type_map = TypeCollection::default();
    let ty = T::inline(&mut type_map, Generics::NONE);
    is_valid_ty(&ty, &type_map)?;
    let result = datatype(conf, &FunctionResultVariant::Value(ty.clone()), &type_map);

    if let Some((ty_name, l0, l1)) = detect_duplicate_type_names(&type_map).into_iter().next() {
        return Err(ExportError::DuplicateTypeName(ty_name, l0, l1));
    }

    result
}

/// Convert a DataType to a TypeScript string
///
/// Eg. `export Name = { demo: string; }`
pub fn export_named_datatype(
    conf: &Typescript,
    typ: &NamedDataType,
    type_map: &TypeCollection,
) -> Output {
    // TODO: Duplicate type name detection?

    is_valid_ty(&typ.inner, type_map)?;
    export_datatype_inner(
        ExportContext {
            cfg: conf,
            path: vec![],
            is_export: true,
        },
        typ,
        type_map,
    )
}

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

    let comments = ctx
        .cfg
        .comment_exporter
        .map(|v| v(CommentFormatterArgs { docs, deprecated }))
        .unwrap_or_default();

    let prefix = match start_with_newline && !comments.is_empty() {
        true => "\n",
        false => "",
    };

    format!("{prefix}{comments}{other}")
}

fn export_datatype_inner(
    ctx: ExportContext,
    typ: &NamedDataType,
    type_map: &TypeCollection,
) -> Output {
    let name = typ.name();
    let docs = typ.docs();
    let ext = typ.ext();
    let deprecated = typ.deprecated();
    let item = &typ.inner;

    let ctx = ctx.with(
        ext.clone()
            .map(|v| PathItem::TypeExtended(name.clone(), *v.impl_location()))
            .unwrap_or_else(|| PathItem::Type(name.clone())),
    );
    let name = sanitise_type_name(ctx.clone(), NamedLocation::Type, name)?;

    let generics = item
        .generics()
        .filter(|generics| !generics.is_empty())
        .map(|generics| format!("<{}>", generics.join(", ")))
        .unwrap_or_default();

    let mut inline_ts = String::new();
    datatype_inner(
        ctx.clone(),
        &FunctionResultVariant::Value((typ.inner).clone()),
        type_map,
        &mut inline_ts,
    )?;

    Ok(inner_comments(
        ctx,
        deprecated,
        docs,
        format!("export type {name}{generics} = {inline_ts}"),
        false,
    ))
}

/// Convert a DataType to a TypeScript string
///
/// Eg. `{ demo: string; }`
pub fn datatype(
    conf: &Typescript,
    typ: &FunctionResultVariant,
    type_map: &TypeCollection,
) -> Output {
    // TODO: Duplicate type name detection?

    let mut s = String::new();
    datatype_inner(
        ExportContext {
            cfg: conf,
            path: vec![],
            is_export: false,
        },
        typ,
        type_map,
        &mut s,
    )
    .map(|_| s)
}

macro_rules! primitive_def {
    ($($t:ident)+) => {
        $(PrimitiveType::$t)|+
    }
}

pub(crate) fn datatype_inner(
    ctx: ExportContext,
    typ: &FunctionResultVariant,
    type_map: &TypeCollection,
    s: &mut String,
) -> Result<()> {
    let typ = match typ {
        FunctionResultVariant::Value(t) => t,
        FunctionResultVariant::Result(t, e) => {
            let mut variants = vec![
                {
                    let mut v = String::new();
                    datatype_inner(
                        ctx.clone(),
                        &FunctionResultVariant::Value(t.clone()),
                        type_map,
                        &mut v,
                    )?;
                    v
                },
                {
                    let mut v = String::new();
                    datatype_inner(
                        ctx,
                        &FunctionResultVariant::Value(e.clone()),
                        type_map,
                        &mut v,
                    )?;
                    v
                },
            ];
            variants.dedup();
            s.push_str(&variants.join(" | "));
            return Ok(());
        }
    };

    Ok(match &typ {
        DataType::Any => s.push_str(ANY),
        DataType::Unknown => s.push_str(UNKNOWN),
        DataType::Primitive(p) => {
            let ctx = ctx.with(PathItem::Type(p.to_rust_str().into()));
            let str = match p {
                primitive_def!(i8 i16 i32 u8 u16 u32 f32 f64) => NUMBER,
                primitive_def!(usize isize i64 u64 i128 u128) => match ctx.cfg.bigint {
                    BigIntExportBehavior::String => STRING,
                    BigIntExportBehavior::Number => NUMBER,
                    BigIntExportBehavior::BigInt => BIGINT,
                    BigIntExportBehavior::Fail => {
                        return Err(ExportError::BigIntForbidden(ctx.export_path()));
                    }
                    BigIntExportBehavior::FailWithReason(reason) => {
                        return Err(ExportError::Other(ctx.export_path(), reason.to_owned()))
                    }
                },
                primitive_def!(String char) => STRING,
                primitive_def!(bool) => BOOLEAN,
            };

            s.push_str(str);
        }
        DataType::Literal(literal) => match literal {
            LiteralType::i8(v) => write!(s, "{v}")?,
            LiteralType::i16(v) => write!(s, "{v}")?,
            LiteralType::i32(v) => write!(s, "{v}")?,
            LiteralType::u8(v) => write!(s, "{v}")?,
            LiteralType::u16(v) => write!(s, "{v}")?,
            LiteralType::u32(v) => write!(s, "{v}")?,
            LiteralType::f32(v) => write!(s, "{v}")?,
            LiteralType::f64(v) => write!(s, "{v}")?,
            LiteralType::bool(v) => write!(s, "{v}")?,
            LiteralType::String(v) => write!(s, r#""{v}""#)?,
            LiteralType::char(v) => write!(s, r#""{v}""#)?,
            LiteralType::None => s.write_str(NULL)?,
            _ => unreachable!(),
        },
        DataType::Nullable(def) => {
            datatype_inner(
                ctx,
                &FunctionResultVariant::Value((**def).clone()),
                type_map,
                s,
            )?;

            let or_null = format!(" | {NULL}");
            if !s.ends_with(&or_null) {
                s.push_str(&or_null);
            }
        }
        DataType::Map(def) => {
            // We use `{ [key in K]: V }` instead of `Record<K, V>` to avoid issues with circular references.
            // Wrapped in Partial<> because otherwise TypeScript would enforce exhaustiveness.
            s.push_str("Partial<{ [key in ");
            datatype_inner(
                ctx.clone(),
                &FunctionResultVariant::Value(def.key_ty().clone()),
                type_map,
                s,
            )?;
            s.push_str("]: ");
            datatype_inner(
                ctx.clone(),
                &FunctionResultVariant::Value(def.value_ty().clone()),
                type_map,
                s,
            )?;
            s.push_str(" }>");
        }
        // We use `T[]` instead of `Array<T>` to avoid issues with circular references.
        DataType::List(def) => {
            let mut dt = String::new();
            datatype_inner(
                ctx,
                &FunctionResultVariant::Value(def.ty().clone()),
                type_map,
                &mut dt,
            )?;

            let dt = if (dt.contains(' ') && !dt.ends_with('}'))
                // This is to do with maintaining order of operations.
                // Eg `{} | {}` must be wrapped in parens like `({} | {})[]` but `{}` doesn't cause `{}[]` is valid
                || (dt.contains(' ') && (dt.contains('&') || dt.contains('|')))
            {
                format!("({dt})")
            } else {
                dt
            };

            if let Some(length) = def.length() {
                s.push('[');

                for n in 0..length {
                    if n != 0 {
                        s.push_str(", ");
                    }

                    s.push_str(&dt);
                }

                s.push(']');
            } else {
                write!(s, "{dt}[]")?;
            }
        }
        DataType::Struct(item) => struct_datatype(
            ctx.with(
                item.sid()
                    .and_then(|sid| type_map.get(*sid))
                    .and_then(|v| v.ext())
                    .map(|v| PathItem::TypeExtended(item.name().clone(), *v.impl_location()))
                    .unwrap_or_else(|| PathItem::Type(item.name().clone())),
            ),
            item.name(),
            item,
            type_map,
            s,
        )?,
        DataType::Enum(item) => {
            let mut ctx = ctx.clone();
            let cfg = ctx.cfg.clone().bigint(BigIntExportBehavior::Number);
            if item.skip_bigint_checks() {
                ctx.cfg = &cfg;
            }

            enum_datatype(
                ctx.with(PathItem::Variant(item.name().clone())),
                item,
                type_map,
                s,
            )?
        }
        DataType::Tuple(tuple) => s.push_str(&tuple_datatype(ctx, tuple, type_map)?),
        DataType::Reference(reference) => {
            let definition = type_map.get(reference.sid()).unwrap(); // TODO: Error handling

            match &reference.generics()[..] {
                [] => s.push_str(&definition.name()),
                generics => {
                    s.push_str(&definition.name());
                    s.push('<');

                    for (i, (_, v)) in generics.iter().enumerate() {
                        if i != 0 {
                            s.push_str(", ");
                        }

                        datatype_inner(
                            ctx.with(PathItem::Type(definition.name().clone())),
                            &FunctionResultVariant::Value(v.clone()),
                            type_map,
                            s,
                        )?;
                    }

                    s.push('>');
                }
            }
        },
        DataType::Generic(ident) => s.push_str(&ident.to_string()),
    })
}

// Can be used with `StructUnnamedFields.fields` or `EnumNamedFields.fields`
fn unnamed_fields_datatype(
    ctx: ExportContext,
    fields: &[NonSkipField],
    type_map: &TypeCollection,
    s: &mut String,
) -> Result<()> {
    Ok(match fields {
        [(field, ty)] => {
            let mut v = String::new();
            datatype_inner(
                ctx.clone(),
                &FunctionResultVariant::Value((*ty).clone()),
                type_map,
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
                    &FunctionResultVariant::Value((*ty).clone()),
                    type_map,
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

fn tuple_datatype(ctx: ExportContext, tuple: &TupleType, type_map: &TypeCollection) -> Output {
    match &tuple.elements()[..] {
        [] => Ok(NULL.to_string()),
        tys => Ok(format!(
            "[{}]",
            tys.iter()
                .map(|v| {
                    let mut s = String::new();
                    datatype_inner(
                        ctx.clone(),
                        &FunctionResultVariant::Value(v.clone()),
                        type_map,
                        &mut s,
                    )
                    .map(|_| s)
                })
                .collect::<Result<Vec<_>>>()?
                .join(", ")
        )),
    }
}

fn struct_datatype(
    ctx: ExportContext,
    key: &str,
    strct: &StructType,
    type_map: &TypeCollection,
    s: &mut String,
) -> Result<()> {
    Ok(match &strct.fields() {
        Fields::Unit => s.push_str(NULL),
        Fields::Unnamed(unnamed) => unnamed_fields_datatype(
            ctx,
            &skip_fields(unnamed.fields()).collect::<Vec<_>>(),
            type_map,
            s,
        )?,
        Fields::Named(named) => {
            let fields = skip_fields_named(named.fields()).collect::<Vec<_>>();

            if fields.is_empty() {
                return Ok(match named.tag().as_ref() {
                    Some(tag) => write!(s, r#"{{ "{tag}": "{key}" }}"#)?,
                    None => write!(s, "Record<{STRING}, {NEVER}>")?,
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
                        &FunctionResultVariant::Value(ty.clone()),
                        type_map,
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
                        type_map,
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

            if let Some(tag) = &named.tag() {
                unflattened_fields.push(format!("{tag}: \"{key}\""));
            }

            if !unflattened_fields.is_empty() {
                field_sections.push(format!("{{ {} }}", unflattened_fields.join("; ")));
            }

            s.push_str(&field_sections.join(" & "));
        }
    })
}

fn enum_variant_datatype(
    ctx: ExportContext,
    type_map: &TypeCollection,
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
                            type_map,
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
                        &FunctionResultVariant::Value(ty.clone()),
                        type_map,
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

fn enum_datatype(
    ctx: ExportContext,
    e: &EnumType,
    type_map: &TypeCollection,
    s: &mut String,
) -> Result<()> {
    if e.variants().is_empty() {
        return Ok(write!(s, "{NEVER}")?);
    }

    Ok(match &e.repr() {
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
                                type_map,
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
                                        type_map,
                                    )?
                                } else {
                                    false
                                };

                                let mut typ = String::new();

                                unnamed_fields_datatype(ctx.clone(), &fields, type_map, &mut typ)?;

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
                                        type_map,
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
                                    type_map,
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
                                    type_map,
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
    type_map: &TypeCollection,
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
        &FunctionResultVariant::Value(ty.clone()),
        type_map,
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
        return Err(ExportError::ForbiddenName(loc, ctx.export_path(), name));
    }

    if let Some(first_char) = ident.chars().next() {
        if !first_char.is_alphabetic() && first_char != '_' {
            return Err(ExportError::InvalidName(
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
        return Err(ExportError::InvalidName(
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
    type_map: &TypeCollection,
) -> Result<bool> {
    match ty {
        DataType::Any
        | DataType::Unknown
        | DataType::Primitive(_)
        // `T & null` is `never` but `T & (U | null)` (this variant) is `T & U` so it's fine.
        | DataType::Nullable(_)
        | DataType::List(_)
        | DataType::Map(_)
        | DataType::Generic(_) => Ok(false),
        DataType::Literal(v) => match v {
            LiteralType::None => Ok(true),
            _ => Ok(false),
        },
        DataType::Struct(v) => match v.fields() {
            Fields::Unit => Ok(true),
            Fields::Unnamed(_) => {
                Err(ExportError::InvalidTaggedVariantContainingTupleStruct(
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
            match v.repr() {
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
            type_map
                .get(r.sid())
                .expect("TypeCollection should have been populated by now")
                .inner
                .clone(),
            type_map,
        ),
    }
}

const ANY: &str = "any";
const UNKNOWN: &str = "unknown";
const NUMBER: &str = "number";
const STRING: &str = "string";
const BOOLEAN: &str = "boolean";
const NULL: &str = "null";
const NEVER: &str = "never";
const BIGINT: &str = "bigint";
