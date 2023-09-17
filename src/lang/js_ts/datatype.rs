use std::borrow::Cow;

use const_format::formatcp;

use crate::{primitive_def, *};

use super::{error::ExportError, BigIntExportBehavior, ExportContext, Output, PathItem, Result};

pub(crate) fn datatype_inner(
    ctx: ExportContext,
    typ: &DataType,
    type_map: &TypeMap,
    empty_tuple_fallback: &'static str,
) -> Output {
    Ok(match &typ {
        DataType::Any => ANY.into(),
        DataType::Primitive(p) => {
            let ctx = ctx.with(PathItem::Type(p.to_rust_str().into()));
            match p {
                primitive_def!(i8 i16 i32 u8 u16 u32 f32 f64) => NUMBER.into(),
                primitive_def!(usize isize i64 u64 i128 u128) => match ctx.cfg.bigint {
                    BigIntExportBehavior::String => STRING.into(),
                    BigIntExportBehavior::Number => NUMBER.into(),
                    BigIntExportBehavior::BigInt => BIGINT.into(),
                    BigIntExportBehavior::Fail => {
                        return Err(ExportError::BigIntForbidden(ctx.export_path()))
                    }
                    BigIntExportBehavior::FailWithReason(reason) => {
                        return Err(ExportError::Other(ctx.export_path(), reason.to_owned()))
                    }
                },
                primitive_def!(String char) => STRING.into(),
                primitive_def!(bool) => BOOLEAN.into(),
            }
        }
        DataType::Literal(literal) => literal.to_ts(),
        DataType::Nullable(def) => {
            let dt = datatype_inner(ctx, def, type_map, NULL)?;

            if dt.ends_with(formatcp!(" | {NULL}")) {
                dt
            } else {
                format!("{dt} | {NULL}")
            }
        }
        DataType::Map(def) => {
            format!(
                // We use this isn't of `Record<K, V>` to avoid issues with circular references.
                "{{ [key in {}]: {} }}",
                datatype_inner(ctx.clone(), &def.0, type_map, NULL)?,
                datatype_inner(ctx, &def.1, type_map, NULL)?
            )
        }
        // We use `T[]` instead of `Array<T>` to avoid issues with circular references.
        DataType::List(def) => {
            let dt = datatype_inner(ctx, def, type_map, NULL)?;
            if dt.contains(' ') && !dt.ends_with('}') {
                format!("({dt})[]")
            } else {
                format!("{dt}[]")
            }
        }
        DataType::Struct(item) => struct_datatype(
            ctx.with(PathItem::Type(item.name().clone())),
            item.name(),
            item,
            type_map,
        )?,
        DataType::Enum(item) => enum_datatype(
            ctx.with(PathItem::Variant(item.name.clone())),
            item,
            type_map,
        )?,
        DataType::Tuple(tuple) => tuple_datatype(ctx, tuple, type_map, empty_tuple_fallback)?,
        DataType::Result(result) => {
            let mut variants = vec![
                datatype_inner(ctx.clone(), &result.0, type_map, NULL)?,
                datatype_inner(ctx, &result.1, type_map, NULL)?,
            ];
            variants.dedup();
            variants.join(" | ")
        }
        DataType::Reference(DataTypeReference { name, generics, .. }) => match &generics[..] {
            [] => name.to_string(),
            generics => {
                let generics = generics
                    .iter()
                    .map(|v| {
                        datatype_inner(
                            ctx.with(PathItem::Type(name.clone())),
                            v,
                            type_map,
                            empty_tuple_fallback,
                        )
                    })
                    .collect::<Result<Vec<_>>>()?
                    .join(", ");

                format!("{name}<{generics}>")
            }
        },
        DataType::Generic(GenericType(ident)) => ident.to_string(),
    })
}

// Can be used with `StructUnnamedFields.fields` or `EnumNamedFields.fields`
fn unnamed_fields_datatype(
    ctx: ExportContext,
    fields: &[Field],
    type_map: &TypeMap,
    empty_tuple_fallback: &'static str,
) -> Output {
    match fields {
        [] => Ok(empty_tuple_fallback.to_string()),
        [field] => datatype_inner(ctx, &field.ty, type_map, NULL),
        fields => Ok(format!(
            "[{}]",
            fields
                .iter()
                .map(|field| datatype_inner(ctx.clone(), &field.ty, type_map, NULL))
                .collect::<Result<Vec<_>>>()?
                .join(", ")
        )),
    }
}

fn tuple_datatype(
    ctx: ExportContext,
    tuple: &TupleType,
    type_map: &TypeMap,
    empty_tuple_fallback: &'static str,
) -> Output {
    match &tuple.fields[..] {
        [] => Ok(empty_tuple_fallback.to_string()),
        [ty] => datatype_inner(ctx, ty, type_map, NULL),
        tys => Ok(format!(
            "[{}]",
            tys.iter()
                .map(|v| datatype_inner(ctx.clone(), v, type_map, NULL))
                .collect::<Result<Vec<_>>>()?
                .join(", ")
        )),
    }
}

fn struct_datatype(ctx: ExportContext, key: &str, s: &StructType, type_map: &TypeMap) -> Output {
    match &s.fields {
        StructFields::Unit => Ok(NULL.into()),
        StructFields::Unnamed(s) => unnamed_fields_datatype(ctx, &s.fields, type_map, "[]"),
        StructFields::Named(s) => {
            if s.fields.is_empty() {
                return Ok(formatcp!("Record<{STRING}, {NEVER}>").into());
            }

            let (flattened, non_flattened): (Vec<_>, Vec<_>) =
                s.fields.iter().partition(|(_, f)| f.flatten);

            let mut field_sections = flattened
                .into_iter()
                .map(|(key, field)| {
                    datatype_inner(
                        ctx.with(PathItem::Field(key.clone())),
                        &field.ty,
                        type_map,
                        "[]",
                    )
                    .map(|type_str| format!("({type_str})"))
                })
                .collect::<Result<Vec<_>>>()?;

            let mut unflattened_fields = non_flattened
                .into_iter()
                .map(|(key, f)| {
                    object_field_to_ts(
                        ctx.with(PathItem::Field(key.clone())),
                        key.clone(),
                        f,
                        type_map,
                    )
                })
                .collect::<Result<Vec<_>>>()?;

            if let Some(tag) = &s.tag {
                unflattened_fields.push(format!("{tag}: \"{key}\""));
            }

            if !unflattened_fields.is_empty() {
                field_sections.push(format!("{{ {} }}", unflattened_fields.join("; ")));
            }

            Ok(field_sections.join(" & "))
        }
    }
}

fn enum_variant_datatype(
    ctx: ExportContext,
    type_map: &TypeMap,
    name: Cow<'static, str>,
    variant: &EnumVariant,
) -> Output {
    match variant {
        // TODO: Remove unreachable in type system
        EnumVariant::Unit => unreachable!("Unit enum variants have no type!"),
        EnumVariant::Named(obj) => {
            let mut fields = if let Some(tag) = &obj.tag {
                let sanitised_name = sanitise_key(name, true);
                vec![format!("{tag}: {sanitised_name}")]
            } else {
                vec![]
            };

            fields.extend(
                obj.fields
                    .iter()
                    .map(|(name, field)| {
                        object_field_to_ts(
                            ctx.with(PathItem::Field(name.clone())),
                            name.clone(),
                            field,
                            type_map,
                        )
                    })
                    .collect::<Result<Vec<_>>>()?,
            );

            Ok(match &fields[..] {
                [] => formatcp!("Record<{STRING}, {NEVER}>").to_string(),
                fields => format!("{{ {} }}", fields.join("; ")),
            })
        }
        EnumVariant::Unnamed(obj) => {
            let fields = obj
                .fields
                .iter()
                .map(|field| datatype_inner(ctx.clone(), &field.ty, type_map, "[]"))
                .collect::<Result<Vec<_>>>()?;

            Ok(match &fields[..] {
                [] => "[]".to_string(),
                [field] => field.to_string(),
                fields => format!("[{}]", fields.join(", ")),
            })
        }
    }
}

fn enum_datatype(ctx: ExportContext, e: &EnumType, type_map: &TypeMap) -> Output {
    if e.variants().is_empty() {
        return Ok(NEVER.to_string());
    }

    Ok(match &e.repr {
        EnumRepr::Untagged => {
            let mut variants = e
                .variants
                .iter()
                .map(|(name, variant)| {
                    Ok(match variant {
                        EnumVariant::Unit => NULL.to_string(),
                        v => enum_variant_datatype(
                            ctx.with(PathItem::Variant(name.clone())),
                            type_map,
                            name.clone(),
                            v,
                        )?,
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            variants.dedup();
            variants.join(" | ")
        }
        repr => {
            let mut variants = e
                .variants
                .iter()
                .map(|(variant_name, variant)| {
                    let sanitised_name = sanitise_key(variant_name.clone(), true);

                    Ok(match (repr, variant) {
                        (EnumRepr::Untagged, _) => unreachable!(),
                        (EnumRepr::Internal { tag }, EnumVariant::Unit) => {
                            format!("{{ {tag}: {sanitised_name} }}")
                        }
                        (EnumRepr::Internal { tag }, EnumVariant::Unnamed(tuple)) => {
                            let mut typ = unnamed_fields_datatype(
                                ctx.clone(),
                                &tuple.fields,
                                type_map,
                                "[]",
                            )?;

                            // TODO: This `null` check is a bad fix for an internally tagged type with a `null` variant being exported as `{ type: "A" } & null` (which is `never` in TS)
                            // TODO: Move this check into the macros so it can apply to any language cause it should (it's just hard to do in the macros)
                            if typ == "null" {
                                format!("({{ {tag}: {sanitised_name} }})")
                            } else {
                                // We wanna be sure `... & ... | ...` becomes `... & (... | ...)`
                                if typ.contains('|') {
                                    typ = format!("({typ})");
                                }
                                format!("({{ {tag}: {sanitised_name} }} & {typ})")
                            }
                        }
                        (EnumRepr::Internal { tag }, EnumVariant::Named(obj)) => {
                            let mut fields = vec![format!("{tag}: {sanitised_name}")];

                            fields.extend(
                                obj.fields
                                    .iter()
                                    .map(|(name, field)| {
                                        object_field_to_ts(
                                            ctx.with(PathItem::Field(name.clone())),
                                            name.clone(),
                                            field,
                                            type_map,
                                        )
                                    })
                                    .collect::<Result<Vec<_>>>()?,
                            );

                            format!("{{ {} }}", fields.join("; "))
                        }
                        (EnumRepr::External, EnumVariant::Unit) => sanitised_name.to_string(),

                        (EnumRepr::External, v) => {
                            let ts_values = enum_variant_datatype(
                                ctx.with(PathItem::Variant(variant_name.clone())),
                                type_map,
                                variant_name.clone(),
                                v,
                            )?;
                            let sanitised_name = sanitise_key(variant_name.clone(), false);

                            format!("{{ {sanitised_name}: {ts_values} }}")
                        }
                        (EnumRepr::Adjacent { tag, .. }, EnumVariant::Unit) => {
                            format!("{{ {tag}: {sanitised_name} }}")
                        }
                        (EnumRepr::Adjacent { tag, content }, v) => {
                            let ts_values = enum_variant_datatype(
                                ctx.with(PathItem::Variant(variant_name.clone())),
                                type_map,
                                variant_name.clone(),
                                v,
                            )?;

                            format!("{{ {tag}: {sanitised_name}; {content}: {ts_values} }}")
                        }
                    })
                })
                .collect::<Result<Vec<_>>>()?;
            variants.dedup();
            variants.join(" | ")
        }
    })
}

impl LiteralType {
    fn to_ts(&self) -> String {
        match self {
            Self::i8(v) => v.to_string(),
            Self::i16(v) => v.to_string(),
            Self::i32(v) => v.to_string(),
            Self::u8(v) => v.to_string(),
            Self::u16(v) => v.to_string(),
            Self::u32(v) => v.to_string(),
            Self::f32(v) => v.to_string(),
            Self::f64(v) => v.to_string(),
            Self::bool(v) => v.to_string(),
            Self::String(v) => format!(r#""{v}""#),
            Self::char(v) => format!(r#""{v}""#),
            Self::None => NULL.to_string(),
        }
    }
}

/// convert an object field into a Typescript string
fn object_field_to_ts(
    ctx: ExportContext,
    key: Cow<'static, str>,
    field: &Field,
    type_map: &TypeMap,
) -> Output {
    let field_name_safe = sanitise_key(key, false);

    // https://github.com/oscartbeaumont/rspc/issues/100#issuecomment-1373092211
    let (key, ty) = match field.optional {
        true => (format!("{field_name_safe}?").into(), &field.ty),
        false => (field_name_safe, &field.ty),
    };

    Ok(format!(
        "{key}: {}",
        datatype_inner(ctx, ty, type_map, NULL)?
    ))
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

const ANY: &str = "any";
const NUMBER: &str = "number";
const STRING: &str = "string";
const BOOLEAN: &str = "boolean";
const NULL: &str = "null";
const NEVER: &str = "never";
const BIGINT: &str = "BigInt";
