mod comments;
mod error;
mod export_config;

pub use comments::*;
pub use error::*;
pub use export_config::*;

use crate::*;

/// Convert a type which implements [`Type`](crate::Type) to a TypeScript string with an export.
/// Eg. `export type Foo = { demo: string; };`
pub fn export<T: Type>(conf: &ExportConfiguration) -> Result<String, TsExportError> {
    export_datatype(
        conf,
        &T::definition(DefOpts {
            parent_inline: true,
            type_map: &mut TypeDefs::default(),
        }),
    )
}

/// Convert a type which implements [`Type`](crate::Type) to a TypeScript string.
/// Eg. `{ demo: string; };`
pub fn inline<T: Type>(conf: &ExportConfiguration) -> Result<String, TsExportError> {
    datatype(
        conf,
        &T::inline(
            DefOpts {
                parent_inline: true,
                type_map: &mut TypeDefs::default(),
            },
            &[],
        ),
    )
}

/// Convert a DataType to a TypeScript string with an export.
/// Eg. `export type Foo = { demo: string; };`
///
// TODO: Accept `DataTypeExt` or `DataType`. This is hard because we take it by reference
pub fn export_datatype(
    conf: &ExportConfiguration,
    typ: &DataType,
) -> Result<String, TsExportError> {
    let inline_ts = datatype(conf, &typ).map_err(|err| TsExportError::WithCtx {
        ty_name: Some("TODO"), // TODO: Fix this
        field_name: None,
        err: Box::new(err),
    })?;

    let (declaration, comments) = match &typ {
        // Named struct
        DataType::Object(CustomDataType::Named {
            name,
            comments,
            item: ObjectType {
                generics, fields, ..
            },
            ..
        }) => {
            if name.is_empty() {
                return Err(TsExportError::AnonymousType);
            } else if let Some(name) = RESERVED_WORDS.iter().find(|v| **v == *name) {
                return Err(TsExportError::ForbiddenTypeName(name));
            }

            (
                match fields.len() {
                    0 => format!("type {name} = {inline_ts}"),
                    _ => {
                        let generics = match generics.len() {
                            0 => "".into(),
                            _ => format!("<{}>", generics.to_vec().join(", ")),
                        };

                        format!("type {name}{generics} = {inline_ts}")
                    }
                },
                *comments,
            )
        }
        // Enum
        DataType::Enum(CustomDataType::Named {
            name,
            comments,
            item: EnumType { generics, .. },
            ..
        }) => {
            if name.is_empty() {
                return Err(TsExportError::AnonymousType);
            } else if let Some(name) = RESERVED_WORDS.iter().find(|v| **v == *name) {
                return Err(TsExportError::ForbiddenTypeName(name));
            }

            let generics = match generics.len() {
                0 => "".into(),
                _ => format!("<{}>", generics.to_vec().join(", ")),
            };

            (format!("type {name}{generics} = {inline_ts}"), *comments)
        }
        // Struct with unnamed fields
        DataType::Tuple(CustomDataType::Named {
            name,
            comments,
            item: TupleType { generics, .. },
            ..
        }) => {
            if let Some(name) = RESERVED_WORDS.iter().find(|v| *v == name) {
                return Err(TsExportError::ForbiddenTypeName(name));
            }

            let generics = match generics.len() {
                0 => "".into(),
                _ => format!("<{}>", generics.to_vec().join(", ")),
            };

            (format!("type {name}{generics} = {inline_ts}"), *comments)
        }
        _ => return Err(TsExportError::CannotExport(typ.clone())), // TODO: Can this be enforced at a type system level
    };

    let comments = conf
        .comment_exporter
        .map(|v| v(comments))
        .unwrap_or_default();
    Ok(format!("{comments}export {declaration}"))
}

/// Convert a DataType to a TypeScript string
/// Eg. `{ demo: string; }`
pub fn datatype(conf: &ExportConfiguration, typ: &DataType) -> Result<String, TsExportError> {
    Ok(match &typ {
        DataType::Any => "any".into(),
        primitive_def!(i8 i16 i32 u8 u16 u32 f32 f64) => "number".into(),
        primitive_def!(usize isize i64 u64 i128 u128) => match conf.bigint {
            BigIntExportBehavior::String => "string".into(),
            BigIntExportBehavior::Number => "number".into(),
            BigIntExportBehavior::BigInt => "BigInt".into(),
            BigIntExportBehavior::Fail => return Err(TsExportError::BigIntForbidden),
            BigIntExportBehavior::FailWithReason(reason) => {
                return Err(TsExportError::Other(reason.to_owned()))
            }
        },
        primitive_def!(String char) => "string".into(),
        primitive_def!(bool) => "boolean".into(),
        DataType::Literal(literal) => literal.to_ts(),
        DataType::Nullable(def) => format!("{} | null", datatype(conf, def)?),
        DataType::Record(def) => {
            format!(
                // We use this isn't of `Record<K, V>` to avoid issues with circular references.
                "{{ [key: {}]: {} }}",
                datatype(conf, &def.0)?,
                datatype(conf, &def.1)?
            )
        }
        // We use `T[]` instead of `Array<T>` to avoid issues with circular references.
        DataType::List(def) => format!("{}[]", datatype(conf, def)?),
        DataType::Tuple(CustomDataType::Named {
            item: TupleType { fields, .. },
            ..
        })
        | DataType::Tuple(CustomDataType::Anonymous(TupleType { fields, .. })) => match &fields[..]
        {
            [] => "null".to_string(),
            [ty] => datatype(conf, ty)?,
            tys => format!(
                "[{}]",
                tys.iter()
                    .map(|v| datatype(conf, v))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ")
            ),
        },
        DataType::Object(CustomDataType::Named { name, item, .. }) => {
            object_datatype(conf, Some(name), item)?
        }
        DataType::Object(CustomDataType::Anonymous(item)) => object_datatype(conf, None, item)?,
        DataType::Enum(CustomDataType::Named { name, item, .. }) => {
            enum_datatype(conf, Some(name), item)?
        }
        DataType::Enum(CustomDataType::Anonymous(item)) => enum_datatype(conf, None, item)?,
        DataType::Reference(DataTypeReference { name, generics, .. }) => match &generics[..] {
            [] => name.to_string(),
            generics => {
                let generics = generics
                    .iter()
                    .map(|v| datatype(conf, v))
                    .collect::<Result<Vec<_>, _>>()?
                    .join(", ");

                format!("{name}<{generics}>")
            }
        },
        DataType::Generic(GenericType(ident)) => ident.to_string(),
        DataType::Placeholder => {
            return Err(TsExportError::InternalError(
                "Attempted to export a placeholder!",
            ))
        }
    })
}

fn object_datatype(
    conf: &ExportConfiguration,
    name: Option<&'static str>,
    ObjectType { fields, tag, .. }: &ObjectType,
) -> Result<String, TsExportError> {
    match &fields[..] {
        [] => Ok("null".to_string()),
        fields => {
            let mut field_sections = fields
                .iter()
                .filter(|f| f.flatten)
                .map(|field| {
                    datatype(conf, &field.ty)
                        .map(|type_str| format!("({type_str})"))
                        .map_err(|err| TsExportError::WithCtx {
                            ty_name: None,
                            field_name: Some(field.key),
                            err: Box::new(err),
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;

            let mut unflattened_fields = fields
                .iter()
                .filter(|f| !f.flatten)
                .map(|field| {
                    let field_name_safe = sanitise_name("TODO", field.key)?;
                    let field_ts_str = datatype(conf, &field.ty);

                    // https://github.com/oscartbeaumont/rspc/issues/100#issuecomment-1373092211
                    let (key, result) = match field.optional {
                        true => (
                            format!("{field_name_safe}?"),
                            match &field.ty {
                                DataType::Nullable(_) => field_ts_str,
                                _ => field_ts_str.map(|v| format!("{v} | null")),
                            },
                        ),
                        false => (field_name_safe, field_ts_str),
                    };

                    result
                        .map(|v| format!("{key}: {v}"))
                        .map_err(|err| TsExportError::WithCtx {
                            ty_name: None,
                            field_name: Some(field.key),
                            err: Box::new(err),
                        })
                })
                .collect::<Result<Vec<_>, _>>()?;

            if let Some(tag) = tag {
                unflattened_fields.push(format!(
                    "{tag}: \"{}\"",
                    name.ok_or(TsExportError::UnableToTagUnnamedType)?
                ));
            }

            if !unflattened_fields.is_empty() {
                field_sections.push(format!("{{ {} }}", unflattened_fields.join("; ")));
            }

            Ok(field_sections.join(" & "))
        }
    }
}

fn enum_datatype(
    conf: &ExportConfiguration,
    _ty_name: Option<&'static str>,
    EnumType { repr, variants, .. }: &EnumType,
) -> Result<String, TsExportError> {
    Ok(match &variants[..] {
        [] => "never".to_string(),
        variants => variants
            .iter()
            .map(|(variant_name, variant)| {
                let sanitised_name = sanitise_name("TODO", variant_name)?;

                Ok(match (repr.clone(), variant) {
                    (EnumRepr::Internal { tag }, EnumVariant::Unit) => {
                        format!("{{ {tag}: \"{sanitised_name}\" }}")
                    }
                    (EnumRepr::Internal { tag }, EnumVariant::Unnamed(tuple)) => {
                        let typ = datatype(
                            conf,
                            &DataType::Tuple(CustomDataType::Anonymous(tuple.clone())),
                        )
                        .map_err(|err| TsExportError::WithCtx {
                            ty_name: None,
                            field_name: Some("TODO"),
                            err: Box::new(err),
                        })?;

                        format!("({{ {tag}: \"{sanitised_name}\" }} & {typ})")
                    }
                    (EnumRepr::Internal { tag }, EnumVariant::Named(obj)) => {
                        let mut fields = vec![format!("{tag}: \"{sanitised_name}\"")];

                        fields.extend(
                            obj.fields
                                .iter()
                                .map(|v| object_field_to_ts(conf, "TODO", v))
                                .collect::<Result<Vec<_>, _>>()?,
                        );

                        format!("{{ {} }}", fields.join("; "))
                    }
                    (EnumRepr::External, EnumVariant::Unit) => {
                        format!("\"{sanitised_name}\"")
                    }

                    (EnumRepr::External, v) => {
                        let ts_values = datatype(conf, &v.data_type()).map_err(|err| {
                            TsExportError::WithCtx {
                                ty_name: None,
                                field_name: Some("TODO"), // TODO
                                err: Box::new(err),
                            }
                        })?;

                        format!("{{ {sanitised_name}: {ts_values} }}")
                    }
                    (EnumRepr::Untagged, EnumVariant::Unit) => "null".to_string(),
                    (EnumRepr::Untagged, v) => {
                        datatype(conf, &v.data_type()).map_err(|err| {
                            TsExportError::WithCtx {
                                ty_name: None,
                                field_name: Some("TODO"), // TODO
                                err: Box::new(err),
                            }
                        })?
                    }
                    (EnumRepr::Adjacent { tag, .. }, EnumVariant::Unit) => {
                        format!("{{ {tag}: \"{sanitised_name}\" }}")
                    }
                    (EnumRepr::Adjacent { tag, content }, v) => {
                        let ts_values = datatype(conf, &v.data_type()).map_err(|err| {
                            TsExportError::WithCtx {
                                ty_name: None,
                                field_name: Some("TODO"),
                                err: Box::new(err),
                            }
                        })?;

                        format!("{{ {tag}: \"{sanitised_name}\"; {content}: {ts_values} }}")
                    }
                })
            })
            .collect::<Result<Vec<_>, TsExportError>>()?
            .join(" | "),
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
            Self::None => "null".to_string(),
        }
    }
}

/// convert an object field into a Typescript string
pub fn object_field_to_ts(
    conf: &ExportConfiguration,
    typ_name: &str,
    field: &ObjectField,
) -> Result<String, TsExportError> {
    let field_name_safe = sanitise_name(typ_name, field.key)?;

    let (key, ty) = match field.optional {
        true => (
            format!("{field_name_safe}?"),
            match &field.ty {
                DataType::Nullable(ty) => ty,
                ty => ty,
            },
        ),
        false => (field_name_safe, &field.ty),
    };

    Ok(format!("{key}: {}", datatype(conf, ty)?))
}

/// sanitise a string to be a valid Typescript key
pub fn sanitise_name(type_name: &str, field_name: &str) -> Result<String, TsExportError> {
    if let Some(name) = RESERVED_WORDS.iter().find(|v| **v == field_name) {
        return Err(TsExportError::ForbiddenFieldName(
            type_name.to_owned(),
            name,
        ));
    }

    let valid = field_name
        .chars()
        .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
        && field_name
            .chars()
            .next()
            .map(|first| !first.is_numeric())
            .unwrap_or(true);

    Ok(if !valid {
        format!(r#""{field_name}""#)
    } else {
        field_name.to_string()
    })
}

// Taken from: https://github.com/microsoft/TypeScript/issues/2536#issuecomment-87194347
const RESERVED_WORDS: &[&str] = &[
    "break",
    "case",
    "catch",
    "class",
    "const",
    "continue",
    "debugger",
    "default",
    "delete",
    "do",
    "else",
    "enum",
    "export",
    "extends",
    "false",
    "finally",
    "for",
    "function",
    "if",
    "import",
    "in",
    "instanceof",
    "new",
    "null",
    "return",
    "super",
    "switch",
    "this",
    "throw",
    "true",
    "try",
    "typeof",
    "var",
    "void",
    "while",
    "with",
    "as",
    "implements",
    "interface",
    "let",
    "package",
    "private",
    "protected",
    "public",
    "static",
    "yield",
    "any",
    "boolean",
    "constructor",
    "declare",
    "get",
    "module",
    "require",
    "number",
    "set",
    "string",
    "symbol",
    "type",
    "from",
    "of",
    "namespace",
    "async",
    "await",
];
